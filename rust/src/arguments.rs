use alloy_dyn_abi::DynSolType;

use crate::{
    evm::{
        op,
        vm::{StepResult, Vm},
        Element, U256, VAL_0_B, VAL_1, VAL_1_B, VAL_32_B,
    },
    Selector,
};
use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
};

const VAL_2: U256 = ruint::uint!(2_U256);
const VAL_31_B: [u8; 32] = ruint::uint!(31_U256).to_be_bytes();
const VAL_131072_B: [u8; 32] = ruint::uint!(131072_U256).to_be_bytes();

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Val {
    offset: u32,
    path: Vec<u32>,
    add_val: u32,
    and_mask: Option<U256>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Label {
    CallData,
    Arg(Val),
    IsZeroResult(Val),
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
}

impl Info {
    fn new() -> Self {
        Self {
            tinfo: None,
            tname: None,
            children: BTreeMap::new(),
        }
    }

    fn to_alloy_type(&self, is_root: bool) -> Vec<DynSolType> {
        if let Some((name, _)) = &self.tname {
            if matches!(name, DynSolType::Bytes) {
                if let Some(InfoVal::Array(0)) | Some(InfoVal::Dynamic(1)) | None = self.tinfo {
                    return vec![name.clone()];
                }
            } else if self.children.is_empty() {
                if let Some(InfoVal::Dynamic(_)) | None = self.tinfo {
                    return vec![name.clone()];
                }
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
                        && self.children.first_key_value().unwrap().1.tinfo.is_none()
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

#[derive(Debug)]
struct ArgsResult {
    data: Info,
    not_bool: BTreeSet<Vec<u32>>,
}

impl ArgsResult {
    fn new() -> Self {
        Self {
            data: Info::new(),
            not_bool: BTreeSet::new(),
        }
    }

    fn get_or_create(&mut self, path: &[u32]) -> &mut Info {
        path.iter().fold(&mut self.data, |node, &key| {
            node.children.entry(key).or_default()
        })
    }

    fn get_mut(&mut self, path: &[u32]) -> Option<&mut Info> {
        path.iter()
            .try_fold(&mut self.data, |node, &key| node.children.get_mut(&key))
    }

    fn mark_not_bool(&mut self, path: &[u32], offset: u32) {
        let full_path = [path, &[offset]].concat();

        if let Some(el) = self.get_mut(&full_path) {
            if let Some((v, _)) = &mut el.tname {
                if matches!(v, DynSolType::Bool) {
                    el.tname = None;
                }
            }
        }

        self.not_bool.insert(full_path);
    }

    fn set_tname(&mut self, path: &[u32], offset: u32, tname: DynSolType, confidence: u8) {
        let full_path = [path, &[offset]].concat();

        if matches!(tname, DynSolType::Bool) && self.not_bool.contains(&full_path) {
            return;
        }

        let el = self.get_or_create(&full_path);
        if let Some((_, conf)) = el.tname {
            if confidence <= conf {
                return;
            }
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
                Some(InfoVal::Dynamic(x)) => {
                    if x > n {
                        return;
                    }
                }
                Some(InfoVal::Array(_)) => return,
                None => (),
            };
        }

        if let Some(InfoVal::Array(p)) = el.tinfo {
            if let InfoVal::Array(n) = tinfo {
                if n < p {
                    return;
                }
            };
        }
        el.tinfo = Some(tinfo);
    }
}

fn and_mask_to_type(mask: U256) -> Option<DynSolType> {
    if mask.is_zero() {
        return None;
    }

    if (mask & (mask + VAL_1)).is_zero() {
        // 0x0000ffff
        let bl = mask.bit_len();
        if bl % 8 == 0 {
            return Some(if bl == 160 {
                DynSolType::Address
            } else {
                DynSolType::Uint(bl)
            });
        }
    } else {
        // 0xffff0000
        let mask = U256::from_le_bytes(mask.to_be_bytes() as [u8; 32]);
        if (mask & (mask + VAL_1)).is_zero() {
            let bl = mask.bit_len();
            if bl % 8 == 0 {
                return Some(DynSolType::FixedBytes(bl / 8));
            }
        }
    }
    None
}

fn analyze(
    vm: &mut Vm<Label>,
    args: &mut ArgsResult,
    ret: StepResult<Label>,
) -> Result<(), Box<dyn std::error::Error>> {
    match ret {
        StepResult {
            op: op::CALLDATASIZE,
            ..
        } => {
            let v = vm.stack.peek_mut()?;
            v.data = VAL_131072_B;
        }

        StepResult {
            op: op @ (op::CALLDATALOAD | op::CALLDATACOPY),
            fa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset,
                            path,
                            add_val,
                            ..
                        })),
                    ..
                }),
            sa,
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

                let mem_offset: u32 = if op == op::CALLDATACOPY {
                    U256::from_be_bytes(sa.unwrap().data)
                        .try_into()
                        .expect("set as u32 in vm.rs")
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
            fa: Some(el),
            sa,
            ..
        } => {
            let offr: Result<u32, _> = U256::from_be_bytes(el.data).try_into();
            if let Ok(off) = offr {
                if (4..131072 - 1024).contains(&off) {
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
                            let mem_offset: u32 = U256::from_be_bytes(sa.unwrap().data)
                                .try_into()
                                .expect("set as u32 in vm.rs");
                            if let Some(v) = vm.memory.get_mut(mem_offset) {
                                v.label = new_label;
                            }
                        }
                        _ => (),
                    }
                }
            }
        }

        StepResult {
            op: op::ADD,
            fa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset: f_offset,
                            path: f_path,
                            add_val: f_add_val,
                            and_mask: f_and_mask,
                        })),
                    ..
                }),
            sa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset: s_offset,
                            path: s_path,
                            add_val: s_add_val,
                            and_mask: s_and_mask,
                        })),
                    ..
                }),
            ..
        } => {
            args.mark_not_bool(&f_path, f_offset);
            args.mark_not_bool(&s_path, s_offset);
            vm.stack.peek_mut()?.label = Some(Label::Arg(if f_path.len() > s_path.len() {
                Val {
                    offset: f_offset,
                    path: f_path,
                    add_val: f_add_val + s_add_val,
                    and_mask: f_and_mask,
                }
            } else {
                Val {
                    offset: s_offset,
                    path: s_path,
                    add_val: s_add_val + f_add_val,
                    and_mask: s_and_mask,
                }
            }));
        }

        StepResult {
            op: op::ADD,
            fa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset,
                            path,
                            add_val,
                            and_mask,
                        })),
                    data,
                    ..
                }),
            sa: Some(ot),
            ..
        }
        | StepResult {
            op: op::ADD,
            sa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset,
                            path,
                            add_val,
                            and_mask,
                        })),
                    data,
                    ..
                }),
            fa: Some(ot),
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
            let r: Result<u32, _> = (U256::from_be_bytes(ot.data) + U256::from(add_val)).try_into();
            if let Ok(val) = r {
                vm.stack.peek_mut()?.label = Some(Label::Arg(Val {
                    offset,
                    path,
                    add_val: val,
                    and_mask,
                }));
            }
        }

        StepResult {
            op: op @ op::MUL,
            fa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset: 0,
                            path,
                            add_val: 0,
                            ..
                        })),
                    ..
                }),
            sa: Some(ot),
            ..
        }
        | StepResult {
            op: op @ op::MUL,
            sa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset: 0,
                            path,
                            add_val: 0,
                            ..
                        })),
                    ..
                }),
            fa: Some(ot),
            ..
        }
        | StepResult {
            op: op @ op::SHL,
            sa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset: 0,
                            path,
                            add_val: 0,
                            ..
                        })),
                    ..
                }),
            fa: Some(ot),
            ..
        } => {
            args.mark_not_bool(&path, 0);
            if let Some(Label::Arg(Val {
                offset: o1,
                path: p1,
                ..
            })) = ot.label
            {
                args.mark_not_bool(&p1, o1);
            }
            if !path.is_empty() {
                let mut mult = U256::from_be_bytes(ot.data);
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
                        if let Ok(m) = otr {
                            if m % 32 == 0 && (32..3200).contains(&m) {
                                args.set_info(&path, InfoVal::Array(m / 32));

                                for el in vm.stack.data.iter_mut() {
                                    if let Some(Label::Arg(lab)) = &el.label {
                                        if lab.offset == 0 && lab.path == path && lab.add_val == 0 {
                                            el.data = VAL_1_B;
                                        }
                                    }
                                }

                                for el in vm.memory.data.iter_mut() {
                                    if let Some(Label::Arg(lab)) = &el.1.label {
                                        if lab.offset == 0 && lab.path == path && lab.add_val == 0 {
                                            el.1.data = VAL_1_B.to_vec();
                                        }
                                    }
                                }

                                // simulate arglen = 1
                                vm.stack.peek_mut()?.data = ot.data; // ==mult.to_be_bytes();
                            }
                        }
                    }
                }
            }
        }

        // 0 < arr.len || arr.len > 0
        StepResult {
            op: op::LT,
            sa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset: 0,
                            path,
                            add_val: 0,
                            and_mask: None,
                        })),
                    ..
                }),
            fa: Some(ot),
            ..
        }
        | StepResult {
            op: op::GT,
            fa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset: 0,
                            path,
                            add_val: 0,
                            and_mask: None,
                        })),
                    ..
                }),
            sa: Some(ot),
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
            fa:
                Some(Element {
                    label: Some(Label::Arg(Val { offset, path, .. })),
                    ..
                }),
            ..
        }
        | StepResult {
            op: op::LT | op::GT | op::MUL,
            sa:
                Some(Element {
                    label: Some(Label::Arg(Val { offset, path, .. })),
                    ..
                }),
            ..
        } => {
            args.mark_not_bool(&path, offset);
        }

        StepResult {
            op: op::AND,
            fa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset,
                            path,
                            add_val,
                            and_mask: None,
                        })),
                    ..
                }),
            sa: Some(ot),
            ..
        }
        | StepResult {
            op: op::AND,
            sa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset,
                            path,
                            add_val,
                            and_mask: None,
                        })),
                    ..
                }),
            fa: Some(ot),
            ..
        } => {
            args.mark_not_bool(&path, offset);
            let mask = U256::from_be_bytes(ot.data);
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
            fa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset,
                            path,
                            add_val,
                            and_mask: None,
                        })),
                    ..
                }),
            sa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset: s_offset,
                            path: s_path,
                            add_val: s_add_val,
                            and_mask: Some(mask),
                        })),
                    ..
                }),
            ..
        }
        | StepResult {
            op: op::EQ,
            sa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset,
                            path,
                            add_val,
                            and_mask: None,
                        })),
                    ..
                }),
            fa:
                Some(Element {
                    label:
                        Some(Label::Arg(Val {
                            offset: s_offset,
                            path: s_path,
                            add_val: s_add_val,
                            and_mask: Some(mask),
                        })),
                    ..
                }),
            ..
        } => {
            if (s_offset == offset) && (s_path == path) && (s_add_val == add_val) {
                if let Some(t) = and_mask_to_type(mask) {
                    args.set_tname(&path, offset, t, 20);
                }
            }
        }

        StepResult {
            op: op::ISZERO,
            fa:
                Some(Element {
                    label: Some(Label::Arg(val)),
                    ..
                }),
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::IsZeroResult(val));
        }

        StepResult {
            op: op::ISZERO,
            fa:
                Some(Element {
                    label: Some(Label::IsZeroResult(val)),
                    ..
                }),
            ..
        } => {
            // Detect check for 0 in DIV, it's not bool in that case: ISZERO, ISZERO, PUSH off, JUMPI, JUMPDEST, DIV
            // for solidity < 0.6.0
            let mut is_bool = true;
            let op = vm.code[vm.pc];
            if let op::PUSH1..=op::PUSH4 = op {
                let n = (op - op::PUSH0) as usize;
                if vm.code[vm.pc + n + 1] == op::JUMPI {
                    let mut arg: [u8; 4] = [0; 4];
                    arg[(4 - n)..].copy_from_slice(&vm.code[vm.pc + 1..vm.pc + 1 + n]);
                    let jumpdest = u32::from_be_bytes(arg) as usize;
                    if jumpdest + 1 < vm.code.len()
                        && vm.code[jumpdest] == op::JUMPDEST
                        && vm.code[jumpdest + 1] == op::DIV
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
            sa:
                Some(Element {
                    label: Some(Label::Arg(Val { offset, path, .. })),
                    ..
                }),
            fa: Some(s0),
            ..
        } => {
            if s0.data < VAL_32_B {
                let s0: u8 = s0.data[31];
                args.set_tname(&path, offset, DynSolType::Int((s0 as usize + 1) * 8), 20);
            }
        }

        StepResult {
            op: op::BYTE,
            sa:
                Some(Element {
                    label: Some(Label::Arg(Val { offset, path, .. })),
                    ..
                }),
            ..
        } => {
            args.set_tname(&path, offset, DynSolType::FixedBytes(32), 4);
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
///
/// # Examples
///
/// ```
/// use evmole::function_arguments;
/// use hex::decode;
///
/// let code = decode("6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256").unwrap();
/// let selector = [0x21, 0x25, 0xb6, 0x5b];
///
/// let arguments: String = function_arguments(&code, &selector, 0);
///
/// assert_eq!(arguments, "uint32,address,uint224");
/// ```

pub fn function_arguments_alloy(
    code: &[u8],
    selector: &Selector,
    gas_limit: u32,
) -> Vec<DynSolType> {
    if cfg!(feature = "trace") {
        println!(
            "Processing selector {:02x}{:02x}{:02x}{:02x}",
            selector[0], selector[1], selector[2], selector[3]
        );
    }
    let mut cd: [u8; 32] = [0; 32];
    cd[0..4].copy_from_slice(selector);
    let mut vm = Vm::<Label>::new(
        code,
        Element {
            data: cd,
            label: Some(Label::CallData),
        },
    );
    let mut args = ArgsResult::new();
    let mut gas_used = 0;
    let mut inside_function = false;
    let real_gas_limit = if gas_limit == 0 {
        5e4 as u32
    } else {
        gas_limit
    };
    while !vm.stopped {
        if cfg!(feature = "trace") && inside_function {
            println!("args: {:?}", args);
            println!("not_bool: {:?}", args.not_bool);
            println!("{:#?}", args.data);
            println!("{:?}\n", vm);
        }
        let ret = match vm.step() {
            Ok(v) => v,
            Err(_e) => {
                break;
            }
        };
        gas_used += ret.gas_used;
        if gas_used > real_gas_limit {
            break;
        }

        if !inside_function {
            if ret.op == op::EQ || ret.op == op::XOR || ret.op == op::SUB {
                let p = vm.stack.peek().unwrap().data; // unwrap is safe unless we have bug in our evm implementation
                if (ret.op == op::EQ && p == VAL_1_B) || (ret.op != op::EQ && p == VAL_0_B) {
                    if let Some(v) = ret.fa {
                        if v.data[28..32] == vm.calldata.data[0..4] {
                            inside_function = true;
                        }
                    }
                }
            }
            continue;
        }

        if analyze(&mut vm, &mut args, ret).is_err() {
            break;
        }
    }

    if args.data.children.is_empty() {
        vec![]
    } else {
        args.data.to_alloy_type(true)
    }
}

pub fn function_arguments(code: &[u8], selector: &Selector, gas_limit: u32) -> String {
    function_arguments_alloy(code, selector, gas_limit)
        .into_iter()
        .map(|t| t.sol_type_name().to_string())
        .collect::<Vec<String>>()
        .join(",")
}
