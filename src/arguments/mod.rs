use crate::{
    DynSolType, Selector,
    collections::HashSet,
    evm::{
        U256, VAL_0_B, VAL_1, VAL_1_B, VAL_32_B,
        element::Element,
        op,
        vm::{StepResult, Vm},
    },
    utils::{and_mask_to_type, elabel, execute_until_function_start, match_first_two},
};
use alloy_primitives::uint;
use std::{cmp::max, collections::BTreeMap};

mod calldata;
use calldata::CallDataImpl;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Label {
    CallData,
    Arg(Val),
    IsZeroResult(Val),
}

const RANGE_CONFIDENCE: u8 = 6;

const VAL_2: U256 = uint!(2_U256);
const VAL_31_B: [u8; 32] = uint!(31_U256).to_be_bytes();

// Bound materialization to the simulated 128 KiB calldata capacity.
const MAX_ARGUMENT_WORDS: usize = 4096;
const MAX_ARGUMENT_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Val {
    offset: u32,
    path: Vec<u32>,
    add_val: u32,
    and_mask: Option<U256>,
}

#[derive(PartialEq, Debug)]
enum InfoVal {
    // (x) - number of elements
    Dynamic(u32), // string|bytes|tuple|array
    Array(u32),
}

#[derive(Default, Debug)]
struct Info {
    tinfo: Option<InfoVal>,
    tname: Option<(DynSolType, u8)>,
    children: BTreeMap<u32, Info>,
    range_blocked: bool,
}

impl Info {
    fn clear_range_hints(&mut self) {
        self.tname.take_if(|(_, conf)| *conf == RANGE_CONFIDENCE);
        for child in self.children.values_mut() {
            child.clear_range_hints();
        }
    }

    fn fits_type_budget(&self, budget: &mut usize, depth: usize) -> bool {
        if depth > MAX_ARGUMENT_DEPTH {
            return false;
        }
        let mut last_word = self
            .children
            .last_key_value()
            .map_or(0, |(&offset, _)| offset / 32);
        if let Some(InfoVal::Array(n) | InfoVal::Dynamic(n)) = self.tinfo {
            last_word = max(last_word, n);
        }
        let Some(remaining) = budget.checked_sub(last_word as usize + 1) else {
            return false;
        };
        *budget = remaining;
        self.children
            .values()
            .all(|child| child.fits_type_budget(budget, depth + 1))
    }

    fn has_dynamic(&self) -> bool {
        self.tinfo.is_some() || self.children.values().any(Self::has_dynamic)
    }

    fn to_alloy_type(&self, is_root: bool) -> Vec<DynSolType> {
        if let Some((name, _)) = &self.tname {
            if matches!(name, DynSolType::Bytes) {
                if let Some(InfoVal::Array(0)) | Some(InfoVal::Dynamic(1)) | None = self.tinfo {
                    return vec![name.clone()];
                }
            } else if self.children.is_empty()
                && let Some(InfoVal::Dynamic(_)) | None = self.tinfo
            {
                return vec![name.clone()];
            }
        }

        let start_key = if let Some(InfoVal::Array(_)) = self.tinfo {
            32
        } else {
            0
        };
        let mut end_key = if let Some((k, _)) = self.children.last_key_value() {
            *k
        } else {
            0
        };
        if let Some(InfoVal::Array(n_elements) | InfoVal::Dynamic(n_elements)) = self.tinfo {
            end_key = max(end_key, n_elements * 32);
        }

        let q: Vec<_> = (start_key..=end_key)
            .step_by(32)
            .flat_map(|k| {
                self.children
                    .get(&k)
                    .map_or(vec![DynSolType::Uint(256)], |val| val.to_alloy_type(false))
                    .into_iter()
            })
            .collect();

        let c = if q.len() > 1 && !is_root {
            vec![DynSolType::Tuple(q.clone())]
        } else {
            q.clone()
        };

        match self.tinfo {
            Some(InfoVal::Array(_)) => {
                vec![if q.len() == 1 {
                    DynSolType::Array(Box::new(q[0].clone()))
                } else {
                    DynSolType::Array(Box::new(DynSolType::Tuple(q)))
                }]
            }
            Some(InfoVal::Dynamic(_)) => {
                if end_key == 0 && self.children.is_empty() {
                    return vec![DynSolType::Bytes];
                }
                if end_key == 32 {
                    if self.children.is_empty() {
                        return vec![DynSolType::Array(Box::new(DynSolType::Uint(256)))];
                    }
                    if self.children.len() == 1
                        && self
                            .children
                            .first_key_value()
                            .expect("len checked above")
                            .1
                            .tinfo
                            .is_none()
                    {
                        return vec![DynSolType::Array(Box::new(q[1].clone()))];
                    }
                }
                c
            }
            None => c,
        }
    }
}

#[derive(Default, Debug)]
struct ArgsResult {
    data: Info,
    not_bool: HashSet<Vec<u32>>,
    decoder_active: bool,
}

impl ArgsResult {
    fn get_or_create(&mut self, path: &[u32]) -> &mut Info {
        path.iter().fold(&mut self.data, |node, &key| {
            node.children.entry(key).or_default()
        })
    }

    fn get_mut(&mut self, path: &[u32]) -> Option<&mut Info> {
        path.iter()
            .try_fold(&mut self.data, |node, &key| node.children.get_mut(&key))
    }

    fn clear_range_hint(&mut self, val: &Val, block: bool) {
        if let Some(info) = self
            .get_mut(&val.path)
            .and_then(|info| info.children.get_mut(&val.offset))
        {
            info.range_blocked |= block;
            info.tname.take_if(|(_, conf)| *conf == RANGE_CONFIDENCE);
        }
    }

    fn has_range_hint(&mut self, path: &[u32], offset: u32) -> bool {
        self.get_mut(&[path, &[offset]].concat())
            .is_some_and(|el| matches!(el.tname, Some((_, RANGE_CONFIDENCE))))
    }

    fn mark_not_bool(&mut self, path: &[u32], offset: u32) {
        let full_path = [path, &[offset]].concat();

        if let Some(el) = self.get_mut(&full_path) {
            el.tname
                .take_if(|(ty, conf)| matches!(ty, DynSolType::Bool) || *conf == RANGE_CONFIDENCE);
        }

        self.not_bool.insert(full_path);
    }

    fn set_tname(&mut self, path: &[u32], offset: u32, tname: DynSolType, confidence: u8) {
        let full_path = [path, &[offset]].concat();

        if (matches!(tname, DynSolType::Bool) || confidence == RANGE_CONFIDENCE)
            && self.not_bool.contains(&full_path)
        {
            return;
        }

        let el = self.get_or_create(&full_path);
        if confidence == RANGE_CONFIDENCE && el.range_blocked {
            return;
        }
        if let Some((_, conf)) = el.tname
            && confidence <= conf
        {
            return;
        }
        el.tname = Some((tname, confidence));
    }

    fn array_in_path(&self, path: &[u32]) -> Vec<bool> {
        path.iter()
            .scan(&self.data, |el, &p| {
                *el = el.children.get(&p)?;
                Some(matches!(el.tinfo, Some(InfoVal::Array(_))))
            })
            .collect()
    }

    fn set_info(&mut self, path: &[u32], tinfo: InfoVal) {
        if path.is_empty() {
            // root
            return;
        }
        let el = self.get_or_create(path);

        if let InfoVal::Dynamic(n) = tinfo {
            match el.tinfo {
                Some(InfoVal::Dynamic(x)) if x > n => {
                    return;
                }
                Some(InfoVal::Dynamic(_)) => {}
                Some(InfoVal::Array(_)) => return,
                None => (),
            };
        }

        if let Some(InfoVal::Array(p)) = el.tinfo
            && let InfoVal::Array(n) = tinfo
            && n < p
        {
            return;
        };
        el.tinfo = Some(tinfo);
    }
}

fn guard_failure_pc(code: &[u8], mut pc: usize, less_than: bool) -> Option<usize> {
    let inverted = *code.get(pc)? == op::ISZERO;
    pc += usize::from(inverted);
    let push = *code.get(pc)?;
    if !(op::PUSH1..=op::PUSH4).contains(&push) {
        return None;
    }
    let size = usize::from(push - op::PUSH0);
    if code.get(pc + 1 + size) != Some(&op::JUMPI) {
        return None;
    }
    Some(if less_than != inverted {
        pc + 2 + size
    } else {
        code.get(pc + 1..pc + 1 + size)?
            .iter()
            .fold(0usize, |acc, byte| (acc << 8) | usize::from(*byte))
    })
}

// Only empty-revert guards are candidates for ABI range validation.
fn is_empty_revert_guard(code: &[u8], pc: usize, less_than: bool) -> bool {
    let Some(rest) = guard_failure_pc(code, pc, less_than).and_then(|pc| code.get(pc..)) else {
        return false;
    };
    let rest = rest.strip_prefix(&[op::JUMPDEST]).unwrap_or(rest);
    matches!(
        rest,
        [op::PUSH0, op::PUSH0 | op::DUP1, op::REVERT, ..]
            | [op::PUSH0, op::PUSH1, 0, op::REVERT, ..]
            | [op::PUSH1, 0, op::PUSH0 | op::DUP1, op::REVERT, ..]
            | [op::PUSH1, 0, op::PUSH1, 0, op::REVERT, ..]
    )
}

fn direct_min_calldata_words(code: &[u8], pc: usize) -> Option<usize> {
    let pc = pc + usize::from(code.get(pc) == Some(&op::JUMPDEST));
    let &push_op = code.get(pc)?;
    if !(op::PUSH1..=op::PUSH4).contains(&push_op) {
        return None;
    }
    let push_size = (push_op - op::PUSH0) as usize;
    if code.get(pc + 1 + push_size) != Some(&op::CALLDATASIZE)
        || code.get(pc + 2 + push_size) != Some(&op::LT)
    {
        return None;
    }
    let min_size = code
        .get(pc + 1..pc + 1 + push_size)?
        .iter()
        .fold(0usize, |value, &byte| (value << 8) | usize::from(byte));
    (min_size >= 4 && (min_size - 4).is_multiple_of(32)).then_some((min_size - 4) / 32)
}

fn dense_min_calldata_words(code: &[u8], pc: usize, packed: &Element<Label>) -> Option<usize> {
    let pc = pc + usize::from(code.get(pc) == Some(&op::JUMPDEST));
    if code.get(pc) != Some(&op::DUP1) {
        return None;
    }
    let &push_op = code.get(pc + 1)?;
    if !(op::PUSH1..=op::PUSH3).contains(&push_op) {
        return None;
    }
    let push_size = (push_op - op::PUSH0) as usize;
    let mask_bytes = code.get(pc + 2..pc + 2 + push_size)?;
    if code.get(pc + 2 + push_size) != Some(&op::AND)
        || code.get(pc + 3 + push_size) != Some(&op::CALLDATASIZE)
        || code.get(pc + 4 + push_size) != Some(&op::LT)
        || mask_bytes[..mask_bytes.len() - 1]
            .iter()
            .any(|&byte| byte != 0xff)
        || mask_bytes.last() != Some(&0xfe)
    {
        return None;
    }

    let mask = mask_bytes
        .iter()
        .fold(U256::ZERO, |value, &byte| (value << 8) | U256::from(byte));
    let packed = U256::from_be_bytes(packed.data);
    let min_size: usize = (packed & mask).try_into().ok()?;
    (min_size >= 4 && (min_size - 4).is_multiple_of(32)).then_some((min_size - 4) / 32)
}

fn analyze(
    vm: &mut Vm<Label, CallDataImpl>,
    args: &mut ArgsResult,
    ret: StepResult<Label>,
) -> Result<(), Box<dyn std::error::Error>> {
    if matches!(
        ret.op,
        op::DIV | op::MOD | op::SUB | op::EXP | op::EQ | op::ISZERO | op::JUMPI | op::SLOAD
    ) {
        // Prior word comparisons rule out new range hints, while preserving
        // types already inferred by the existing ABI cleanup checks.
        let compared_words = ret.op == op::EQ
            && ret
                .args
                .iter()
                .all(|arg| matches!(arg.label, Some(Label::Arg(_))));
        // Comparing a validated word with a constant is its normal use.
        if ret.op != op::EQ || compared_words {
            for operand in &ret.args {
                if let Some(Label::Arg(val)) = &operand.label {
                    args.clear_range_hint(val, compared_words);
                }
            }
        }
    }
    if ret.op == op::REVERT && args.decoder_active {
        // An aborted simulated decoder path does not establish a range type.
        args.data.clear_range_hints();
    }
    if matches!(ret.op, op::LT | op::GT) && args.decoder_active {
        // Orient strict comparisons as value < bound or value > bound.
        let comparison = match (&ret.args[0].label, &ret.args[1].label) {
            (Some(Label::Arg(val)), None) => Some((val, &ret.args[1], ret.op == op::LT)),
            (None, Some(Label::Arg(val))) => Some((val, &ret.args[0], ret.op == op::GT)),
            _ => None,
        };
        if let Some((val, bound, less_than)) = comparison
            && val.add_val == 0
            && val.and_mask.is_none()
            // Slot zero of a dynamic path may be its length. Bounds on that
            // length describe the container, not a scalar argument.
            && (val.offset != 0 || val.path.is_empty())
            && let Ok(bound) = u32::try_from(bound)
            && let Some(upper) = bound.checked_sub(u32::from(less_than))
            && (1..=255).contains(&upper)
            && is_empty_revert_guard(vm.code, vm.pc, less_than)
        {
            // Solidity validates enums this way, including two-member ones, and
            // validates bools differently, so a bound of 1 is uint8 as well.
            args.set_tname(&val.path, val.offset, DynSolType::Uint(8), RANGE_CONFIDENCE);
            // Inserted directly: mark_not_bool would drop the new hint.
            args.not_bool
                .insert([val.path.as_slice(), &[val.offset]].concat());
            return Ok(());
        }
    }
    if matches!(
        ret.op,
        op::SLOAD
            | op::SSTORE
            | op::TLOAD
            | op::TSTORE
            | op::KECCAK256
            | op::BALANCE
            | op::EXTCODESIZE
            | op::EXTCODEHASH
            | op::EXTCODECOPY
            | op::CALL
            | op::CALLCODE
            | op::DELEGATECALL
            | op::STATICCALL
            | op::CREATE
            | op::CREATE2
            | op::SELFDESTRUCT
            | op::LOG0..=op::LOG4
    ) {
        args.decoder_active = false;
    }
    match ret {
        StepResult {
            op: op @ (op::CALLDATALOAD | op::CALLDATACOPY),
            args:
                [
                    elabel!(Label::Arg(Val {
                        offset,
                        path,
                        add_val,
                        ..
                    })),
                    sa,
                    ..,
                ],
            ..
        } => {
            if add_val >= 4 && (add_val - 4) % 32 == 0 {
                let mut full_path = path.clone();
                full_path.push(offset);

                let mut po: u32 = 0;
                if add_val != 4 {
                    po += args
                        .array_in_path(&path)
                        .iter()
                        .fold(0, |s, &is_arr| if is_arr { s + 32 } else { s });
                    if po > (add_val - 4) {
                        po = 0;
                    }
                }

                let new_off = add_val - 4 - po;

                args.set_info(&full_path, InfoVal::Dynamic(new_off / 32));

                let mem_offset = if op == op::CALLDATACOPY {
                    u32::try_from(sa).expect("set as u32 in vm.rs")
                } else {
                    0
                };

                if new_off == 0 && *args.array_in_path(&full_path).last().unwrap_or(&false) {
                    match op {
                        op::CALLDATALOAD => vm.stack.peek_mut()?.data = VAL_1_B,
                        op::CALLDATACOPY => {
                            if let Some(v) = vm.memory.get_mut(mem_offset) {
                                v.data = VAL_1_B.to_vec();
                            }
                        }
                        _ => (),
                    }
                }

                let new_label = Some(Label::Arg(Val {
                    offset: new_off,
                    path: full_path,
                    add_val: 0,
                    and_mask: None,
                }));
                match op {
                    op::CALLDATALOAD => vm.stack.peek_mut()?.label = new_label,
                    op::CALLDATACOPY => {
                        if let Some(v) = vm.memory.get_mut(mem_offset) {
                            args.set_tname(&path, offset, DynSolType::Bytes, 10);
                            v.label = new_label;
                        }
                    }
                    _ => (),
                }
            }
        }

        StepResult {
            op: op @ (op::CALLDATALOAD | op::CALLDATACOPY),
            args: [el, sa, ..],
            ..
        } => {
            if let Ok(off) = u32::try_from(el)
                && (4..131072 - 1024).contains(&off)
            {
                // 131072 is constant from ./calldata.rs
                // -1024: cut 'trustedForwarder'
                args.get_or_create(&[off - 4]);

                let new_label = Some(Label::Arg(Val {
                    offset: off - 4,
                    path: Vec::new(),
                    add_val: 0,
                    and_mask: None,
                }));
                match op {
                    op::CALLDATALOAD => vm.stack.peek_mut()?.label = new_label,
                    op::CALLDATACOPY => {
                        let mem_offset = u32::try_from(sa).expect("set as u32 in vm.rs");
                        if let Some(v) = vm.memory.get_mut(mem_offset) {
                            v.label = new_label;
                        }
                    }
                    _ => (),
                }
            }
        }

        StepResult {
            op: op::CALL | op::CALLCODE | op::DELEGATECALL | op::STATICCALL,
            args: [elabel!(Label::Arg(Val { offset, path, .. })), ..],
            ..
        } => {
            args.mark_not_bool(&path, offset);
            args.set_tname(&path, offset, DynSolType::Address, 20);
        }

        StepResult {
            op: op::ADD,
            args:
                [
                    elabel!(Label::Arg(Val {
                        offset: f_offset,
                        path: f_path,
                        add_val: f_add_val,
                        and_mask: f_and_mask,
                    })),
                    elabel!(Label::Arg(Val {
                        offset: s_offset,
                        path: s_path,
                        add_val: s_add_val,
                        and_mask: s_and_mask,
                    })),
                    ..,
                ],
            ..
        } => {
            args.mark_not_bool(&f_path, f_offset);
            args.mark_not_bool(&s_path, s_offset);
            if let Some(add_val) = f_add_val.checked_add(s_add_val) {
                vm.stack.peek_mut()?.label = Some(Label::Arg(if f_path.len() > s_path.len() {
                    Val {
                        offset: f_offset,
                        path: f_path,
                        add_val,
                        and_mask: f_and_mask,
                    }
                } else {
                    Val {
                        offset: s_offset,
                        path: s_path,
                        add_val,
                        and_mask: s_and_mask,
                    }
                }));
            }
        }

        StepResult {
            op: op::ADD,
            args:
                match_first_two!(
                    Element {
                        label: Some(Label::Arg(Val {
                            offset,
                            path,
                            add_val,
                            and_mask,
                        })),
                        data,
                        ..
                    },
                    ot
                ),
            ..
        } => {
            args.mark_not_bool(&path, offset);
            if offset == 0
                && add_val == 0
                && !path.is_empty()
                && data == VAL_0_B
                && ot.data == U256::MAX.to_be_bytes()
            {
                vm.stack.peek_mut()?.data = VAL_0_B; // sub(-1) as add(0xff..ff)
            }
            if let Ok(val) = u32::try_from(U256::from_be_bytes(ot.data) + U256::from(add_val)) {
                vm.stack.peek_mut()?.label = Some(Label::Arg(Val {
                    offset,
                    path,
                    add_val: val,
                    and_mask,
                }));
            }
        }

        StepResult {
            op: op::SHL,
            args: [shift, elabel!(Label::Arg(Val { offset, path, .. })), ..],
            ..
        } if path.is_empty() && args.decoder_active => {
            args.mark_not_bool(&path, offset);
            if guard_failure_pc(vm.code, vm.pc, true).is_some()
                && let Ok(shift) = usize::try_from(shift)
                && (8..=256).contains(&shift)
                && shift.is_multiple_of(8)
            {
                args.set_tname(&path, offset, DynSolType::FixedBytes(shift / 8), 20);
            }
        }

        StepResult {
            op: op @ op::MUL,
            args:
                match_first_two!(
                    elabel!(Label::Arg(Val {
                        offset: 0,
                        path,
                        add_val: 0,
                        ..
                    })),
                    ot
                ),
            ..
        }
        | StepResult {
            op: op @ op::SHL,
            args:
                [
                    ot,
                    elabel!(Label::Arg(Val {
                        offset: 0,
                        path,
                        add_val: 0,
                        ..
                    })),
                    ..,
                ],
            ..
        } => {
            args.mark_not_bool(&path, 0);
            if let Some(Label::Arg(Val {
                offset: o1,
                path: ref p1,
                ..
            })) = ot.label
            {
                args.mark_not_bool(p1, o1);
            }
            if !path.is_empty() {
                let mut mult: U256 = ot.into();
                if op == op::SHL {
                    mult = VAL_1 << mult;
                }

                match mult {
                    VAL_1 => {
                        if let Some((last, rest)) = path.split_last() {
                            args.set_tname(rest, *last, DynSolType::Bytes, 10);
                        }
                    }

                    VAL_2 => {
                        // slen*2+1 for SSTORE
                        if let Some((last, rest)) = path.split_last() {
                            args.set_tname(rest, *last, DynSolType::String, 20);
                        }
                    }

                    _ => {
                        let otr: Result<u32, _> = mult.try_into();
                        if let Ok(m) = otr
                            && m % 32 == 0
                            && (32..3200).contains(&m)
                        {
                            args.set_info(&path, InfoVal::Array(m / 32));

                            for el in vm.stack.data.iter_mut() {
                                if let Some(Label::Arg(lab)) = &el.label
                                    && lab.offset == 0
                                    && lab.path == path
                                    && lab.add_val == 0
                                {
                                    el.data = VAL_1_B;
                                }
                            }

                            for el in vm.memory.data.iter_mut() {
                                if let Some(Label::Arg(lab)) = &el.1.label
                                    && lab.offset == 0
                                    && lab.path == path
                                    && lab.add_val == 0
                                {
                                    el.1.data = VAL_1_B.to_vec();
                                }
                            }

                            // simulate arglen = 1
                            vm.stack.peek_mut()?.data = mult.to_be_bytes();
                        }
                    }
                }
            }
        }

        // 0 < arr.len || arr.len > 0
        StepResult {
            op: op::LT,
            args:
                [
                    ot,
                    elabel!(Label::Arg(Val {
                        offset: 0,
                        path,
                        add_val: 0,
                        and_mask: None,
                    })),
                    ..,
                ],
            ..
        }
        | StepResult {
            op: op::GT,
            args:
                [
                    elabel!(Label::Arg(Val {
                        offset: 0,
                        path,
                        add_val: 0,
                        and_mask: None,
                    })),
                    ot,
                    ..,
                ],
            ..
        } => {
            args.mark_not_bool(&path, 0);
            // 31 = string for storage
            if ot.data == VAL_0_B || ot.data == VAL_31_B {
                vm.stack.peek_mut()?.data = VAL_1_B;
            }
        }

        StepResult {
            op: op::LT | op::GT | op::MUL,
            args: match_first_two!(elabel!(Label::Arg(Val { offset, path, .. })), _),
            ..
        } => {
            // A validated enum can be checked again, e.g. before a panic.
            if !args.has_range_hint(&path, offset) {
                args.mark_not_bool(&path, offset);
            }
        }

        StepResult {
            op: op::MSTORE8,
            args: [elabel!(Label::Arg(Val { offset, path, .. })), ..],
            ..
        } => {
            args.mark_not_bool(&path, offset);
            args.set_tname(&path, offset, DynSolType::Uint(8), 12);
        }

        // Vyper validates narrow ABI scalars by rejecting non-zero high bits.
        // Modern code branches directly on SHR, while older versions insert
        // ISZERO before the branch.
        StepResult {
            op: op::SHR,
            args: [shift, elabel!(Label::Arg(Val { offset, path, .. })), ..],
            ..
        } if args.decoder_active => {
            if guard_failure_pc(vm.code, vm.pc, true).is_some()
                && let Ok(width) = usize::try_from(shift)
                && (1..256).contains(&width)
            {
                let inferred_type = if width == 1 {
                    Some(DynSolType::Bool)
                } else if width == 160 {
                    Some(DynSolType::Address)
                } else if width.is_multiple_of(8) {
                    Some(DynSolType::Uint(width))
                } else {
                    None
                };
                if let Some(inferred_type) = inferred_type {
                    args.set_tname(&path, offset, inferred_type, 20);
                }
            }
        }

        StepResult {
            op: op::SDIV | op::SMOD,
            args: match_first_two!(elabel!(Label::Arg(Val { offset, path, .. })), _),
            ..
        } => {
            args.mark_not_bool(&path, offset);
            args.set_tname(&path, offset, DynSolType::Int(256), 8);
        }

        StepResult {
            op: op::AND,
            args:
                match_first_two!(
                    elabel!(Label::Arg(Val {
                        offset,
                        path,
                        add_val,
                        and_mask: None,
                    })),
                    ot
                ),
            ..
        } => {
            args.mark_not_bool(&path, offset);
            let mask: U256 = ot.into();
            if let Some(t) = and_mask_to_type(mask) {
                args.set_tname(&path, offset, t, 5);
                vm.stack.peek_mut()?.label = Some(Label::Arg(Val {
                    offset,
                    path,
                    add_val,
                    and_mask: Some(mask),
                }));
            }
        }

        StepResult {
            op: op::EQ,
            args:
                match_first_two!(
                    elabel!(Label::Arg(Val {
                        offset,
                        path,
                        add_val,
                        and_mask: None,
                    })),
                    elabel!(Label::Arg(Val {
                        offset: s_offset,
                        path: s_path,
                        add_val: s_add_val,
                        and_mask: Some(mask),
                    }))
                ),
            ..
        } => {
            if (s_offset == offset)
                && (s_path == path)
                && (s_add_val == add_val)
                && let Some(t) = and_mask_to_type(mask)
            {
                args.set_tname(&path, offset, t, 20);
            }
        }

        StepResult {
            op: op::ISZERO,
            args: [elabel!(Label::Arg(val)), ..],
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::IsZeroResult(val));
        }

        StepResult {
            op: op::ISZERO,
            args: [elabel!(Label::IsZeroResult(val)), ..],
            ..
        } => {
            // Detect check for 0 in DIV, it's not bool in that case: ISZERO, ISZERO, PUSH off, JUMPI, JUMPDEST, DIV
            // for solidity < 0.6.0
            let mut is_bool = true;
            if let Some((&next_op, rest)) = vm.code.get(vm.pc..).and_then(|code| code.split_first())
                && let op::PUSH1..=op::PUSH4 = next_op
            {
                let n = (next_op - op::PUSH0) as usize;
                if rest.get(n) == Some(&op::JUMPI) {
                    let mut arg: [u8; 4] = [0; 4];
                    arg[(4 - n)..].copy_from_slice(&rest[..n]);
                    let jumpdest = u32::from_be_bytes(arg) as usize;
                    if vm.code.get(jumpdest) == Some(&op::JUMPDEST)
                        && jumpdest.checked_add(1).and_then(|pc| vm.code.get(pc)) == Some(&op::DIV)
                    {
                        is_bool = false;
                    }
                }
            }

            if is_bool {
                args.set_tname(&val.path, val.offset, DynSolType::Bool, 5);
            }
        }

        StepResult {
            op: op::SIGNEXTEND,
            args: [s0, elabel!(Label::Arg(Val { offset, path, .. })), ..],
            ..
        } => {
            if s0.data < VAL_32_B {
                let s0: u8 = s0.data[31];
                args.set_tname(&path, offset, DynSolType::Int((s0 as usize + 1) * 8), 20);
            }
        }

        StepResult {
            op: op::BYTE,
            args: [_, elabel!(Label::Arg(Val { offset, path, .. })), ..],
            ..
        } => {
            args.set_tname(&path, offset, DynSolType::FixedBytes(32), 4);
        }

        StepResult {
            op: op::MLOAD,
            memory_load: Some(memory_load),
            ..
        } => {
            if memory_load.chunks.len() == 1
                && memory_load.size == 32
                && memory_load.chunks[0].dst_range.start == 0
                && memory_load.chunks[0].dst_range.end == 32
                && let Label::Arg(val) = &memory_load.chunks[0].src_label
            {
                vm.stack.peek_mut()?.label = Some(Label::Arg(val.clone()));
            }
        }

        _ => (),
    };
    Ok(())
}

/// Extracts function arguments
///
/// # Arguments
///
/// * `code` - A slice of deployed contract bytecode
/// * `selector` - A function selector
/// * `gas_limit` - Maximum allowed gas usage; set to `0` to use defaults
/// ```
pub fn function_arguments(code: &[u8], selector: &Selector, gas_limit: u32) -> Vec<DynSolType> {
    let calldata = CallDataImpl {
        selector: *selector,
    };
    let mut vm = Vm::new(code, &calldata);
    let mut args = ArgsResult {
        decoder_active: true,
        ..ArgsResult::default()
    };
    let mut gas_used = 0;
    let real_gas_limit = if gas_limit == 0 {
        5e4 as u32
    } else {
        gas_limit
    };

    if let Some(g) = execute_until_function_start(&mut vm, real_gas_limit) {
        gas_used += g;
    } else {
        return vec![];
    }

    // Read Vyper's minimum calldata size to recover head words that the one
    // concrete execution path never reads.
    let mut min_calldata_words = direct_min_calldata_words(vm.code, vm.pc);
    if let Ok(packed) = vm.stack.peek()
        && let Some(words) = dense_min_calldata_words(vm.code, vm.pc, packed)
    {
        min_calldata_words = Some(words);
    }

    while !vm.stopped {
        let ret = match vm.step() {
            Ok(v) => v,
            Err(_e) => {
                // println!("{}", _e);
                break;
            }
        };
        gas_used += ret.gas_used;
        if gas_used > real_gas_limit {
            break;
        }

        if analyze(&mut vm, &mut args, ret).is_err() {
            break;
        }
    }

    // A single array without decoded elements contributes a length word to
    // the minimum tail size. Exclude that word when recovering its ABI head.
    let head_words = min_calldata_words.and_then(|words| {
        let mut dynamic_args = args
            .data
            .children
            .values()
            .filter(|info| info.has_dynamic());
        match (dynamic_args.next(), dynamic_args.next()) {
            (None, None) => Some(words),
            (Some(info), None)
                if matches!(info.tinfo, Some(InfoVal::Array(_))) && info.children.is_empty() =>
            {
                words.checked_sub(1)
            }
            _ => None,
        }
    });
    if let Some(words) = head_words.filter(|&words| words <= MAX_ARGUMENT_WORDS) {
        for offset in (0..words * 32).step_by(32) {
            args.get_or_create(&[offset as u32]);
        }
    }

    let mut type_budget = MAX_ARGUMENT_WORDS * 2;
    if args.data.children.is_empty() || !args.data.fits_type_budget(&mut type_budget, 0) {
        vec![]
    } else {
        args.data.to_alloy_type(true)
    }
}

#[cfg(test)]
mod tests {
    use super::function_arguments;
    use crate::selectors::function_selectors;
    use crate::{ContractInfoArgs, contract_info};
    use alloy_primitives::hex;

    #[test]
    fn test_double_iszero_at_end_of_code() {
        let code = hex::decode("60003560e01c631122334414600f575b6004351515").unwrap();

        let info = contract_info(ContractInfoArgs::new(&code).with_arguments());

        let functions = info.functions.unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].selector, [0x11, 0x22, 0x33, 0x44]);
        assert_eq!(functions[0].arguments, Some(vec![crate::DynSolType::Bool]));
    }

    #[test]
    fn test_add_two_args_add_val_overflow() {
        // calldataload(4) + 0xfffffff0, then that value added to itself:
        // the two add_val constants overflow u32
        let code = hex::decode("60003560e01c631122334414600f575b60043563fffffff001800100").unwrap();

        let info = contract_info(ContractInfoArgs::new(&code).with_arguments());

        let functions = info.functions.unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(
            functions[0].arguments,
            Some(vec![crate::DynSolType::Uint(256)])
        );
    }

    #[test]
    fn test_code_offset_buffer() {
        // This is a solidity trick to quickly zeroize memory, but
        // causing crashes in earlier evmole implementation
        //
        // Mainnet 0x27e70bfdf7de32bae2274c8d37d51934ff098910
        let code = hex::decode("6000608052600060a052600060c052600060e05260006101005260006101205260006101405260006101605260006101805260006101a05260006101c05260006101e05260006102005260006102205260006102405260006102605260006102805260006102a05260006102c05260006102e05260006103005260006103205260006103405260006103605260006103805260006103a05260006103c05260006103e0526000610400526000610420526000610440527f66702d66702d7075662d763100000000000000000000000000000000000000006000554360018060000101556101806103dc610200396102005160045561022051600855610240516006556102605160075561028051600a556102a051600b556102c0516001556102e0516010556103005160115561032051601455610340516080906103dc9060208101101561014c57600080fd5b602061034051016103dc01101561016257600080fd5b6103405160208101101561017557600080fd5b602061034051016103dc0161038039610380516012556103a0516013556103c0516015556103e051601655610360516020906103dc90810110156101b857600080fd5b610360516103dc016104003961040051600c556103dc610240810110156101de57600080fd5b6102406103dc016103605260006104605261040051610480525b610480511561033157602061036051600160006104605114610218575060005b6102405760206104605160206104605102041461023457600080fd5b60206104605102610243565b60005b6103605101101561025357600080fd5b600160006104605114610264575060005b61028c5760206104605160206104605102041461028057600080fd5b6020610460510261028f565b60005b6103605101610420396104205161044051810110156102ad57600080fd5b610440516104205101610440527f7061796d656e740000000000000000000000000000000000000000000000000060c0526104605160e05261042051604060c020556104605160016104605101101561030557600080fd5b6001610460510161046052610480516001111561032157600080fd5b60016104805103610480526101f8565b341561033c57600080fd5b60016003557f587ece4cd19692c5be1a4184503d607d45542d2aca0698c0068f52e09ccb541c6040610200a16066806103766000396000f3007c010000000000000000000000000000000000000000000000000000000060003504608081905263696eb8fb1415603e576000546104a0908152602090f35b366000803760008036600060016000015460155a03f4605c57600080fd5b3d6000803e3d6000f3").unwrap();

        for sig in function_selectors(&code, 0, None).0.keys() {
            let _ = function_arguments(&code, sig, 0);
        }
    }
}
