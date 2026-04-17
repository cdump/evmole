//! # Warning
//! This code is in an experimental state and under active development.
//! Code structure are subject to change.
use crate::{
    DynSolType, Selector, Slot,
    collections::HashMap,
    evm::{
        U256, VAL_1, VAL_1_B, VAL_32_B,
        calldata::{CallDataImpl, CallDataLabel, CallDataLabelType},
        element::Element,
        op,
        vm::{StepResult, Vm},
    },
    utils::{and_mask_to_type, elabel, execute_until_function_start, match_first_two},
};
use alloy_primitives::keccak256;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

/// Represents a storage variable record in a smart contract's storage layout.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct StorageRecord {
    /// Storage slot location for the variable
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serialize::slot"))]
    pub slot: Slot,

    /// Byte offset within the storage slot (0-31)
    pub offset: u8,

    /// Variable type
    pub r#type: String,

    /// Function selectors that read from this storage location
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serialize::vec_selector")
    )]
    pub reads: Vec<Selector>,

    /// Function selectors that write to this storage location
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serialize::vec_selector")
    )]
    pub writes: Vec<Selector>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Label {
    Constant,

    Typed(DynSolType),
    Sloaded(Rc<RefCell<StorageElement>>),
    IsZero(Rc<RefCell<StorageElement>>),
    Keccak(u32, SlotExpr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SlotExpr {
    Plain(Slot),
    Mapping {
        key_type: DynSolType,
        base: Box<SlotExpr>,
    },
    DynamicArray {
        base: Box<SlotExpr>,
    },
    HashedConst {
        hash: Slot,
        preimage: Vec<u8>,
    },
    UnknownHash {
        size: u32,
        preimage: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum SlotKey {
    Known(Slot),
    UnknownHash { size: u32, preimage: Vec<u8> },
}

fn should_surface_hashed_slot(preimage: &[u8]) -> bool {
    if matches!(preimage.len(), 32 | 64) {
        return false;
    }

    if preimage.len() > 32 {
        let tail = &preimage[preimage.len() - 32..];
        if tail[..31].iter().all(|b| *b == 0) {
            return false;
        }
    }

    true
}

impl SlotExpr {
    fn canonical_slot(&self) -> Option<Slot> {
        match self {
            SlotExpr::Plain(slot) => Some(*slot),
            SlotExpr::HashedConst {
                hash: slot,
                preimage,
            } => should_surface_hashed_slot(preimage).then_some(*slot),
            SlotExpr::Mapping { base, .. } | SlotExpr::DynamicArray { base } => {
                base.canonical_slot()
            }
            SlotExpr::UnknownHash { .. } => None,
        }
    }

    fn slot_key(&self) -> SlotKey {
        match self {
            SlotExpr::Plain(slot) | SlotExpr::HashedConst { hash: slot, .. } => {
                SlotKey::Known(*slot)
            }
            SlotExpr::Mapping { base, .. } | SlotExpr::DynamicArray { base } => base.slot_key(),
            SlotExpr::UnknownHash { size, preimage } => SlotKey::UnknownHash {
                size: *size,
                preimage: preimage.clone(),
            },
        }
    }
}

impl CallDataLabel for Label {
    fn label(_: usize, tp: &DynSolType, label_type: CallDataLabelType) -> Option<Label> {
        if matches!(label_type, CallDataLabelType::RealValue) {
            Some(Label::Typed(tp.clone()))
        } else {
            None
        }
    }
}

fn get_base_internal_type(val: &DynSolType) -> DynSolType {
    if let DynSolType::Array(t) = val {
        get_base_internal_type(t)
    } else {
        val.clone()
    }
}

fn get_base_score(t: &DynSolType) -> usize {
    match t {
        DynSolType::Uint(256) => 1,
        DynSolType::Uint(8) => 3,
        DynSolType::Bool => 4,
        DynSolType::FixedBytes(32) => 6,
        DynSolType::FixedBytes(_) => 2,
        DynSolType::String | DynSolType::Bytes => 500,
        DynSolType::Array(v) => 5 * get_base_score(v),
        _ => 5,
    }
}

#[derive(Clone, PartialEq, Eq)]
enum StorageType {
    Base(DynSolType),
    Map(DynSolType, Box<StorageType>),
}

impl StorageType {
    fn set_type(&mut self, tp: DynSolType) {
        if let StorageType::Base(DynSolType::String) = self {
            return;
        }
        match self {
            StorageType::Base(DynSolType::Array(v)) => {
                let mut current = v.as_mut();
                while let DynSolType::Array(inner) = current {
                    current = inner;
                    if let DynSolType::Uint(256) = &current {}
                }
                *current = tp;
            }
            StorageType::Base(v) => *v = tp,
            StorageType::Map(_, v) => v.set_type(tp),
        }
    }

    fn get_internal_type(&self) -> DynSolType {
        match self {
            StorageType::Base(t) => get_base_internal_type(t),
            StorageType::Map(_, v) => v.get_internal_type(),
        }
    }

    fn get_score(&self) -> usize {
        match self {
            StorageType::Base(t) => get_base_score(t),
            StorageType::Map(k, v) => 1000 * get_base_score(k) + v.get_score(),
        }
    }

    fn is_string_like(&self) -> bool {
        matches!(
            self,
            StorageType::Base(DynSolType::String | DynSolType::Bytes)
        )
    }

    fn requires_zero_offset(&self) -> bool {
        match self {
            StorageType::Map(_, _) => true,
            StorageType::Base(t) => matches!(
                t,
                DynSolType::Array(_)
                    | DynSolType::FixedArray(_, _)
                    | DynSolType::String
                    | DynSolType::Bytes
                    | DynSolType::Uint(256)
                    | DynSolType::Int(256)
                    | DynSolType::FixedBytes(32)
            ),
        }
    }
}

impl std::fmt::Debug for StorageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageType::Base(v) => write!(f, "{}", v.sol_type_name()),
            StorageType::Map(k, v) => write!(f, "mapping({} => {:?})", k.sol_type_name(), v),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct StorageElement {
    slot_key: SlotKey,
    slot: Option<Slot>,
    slot_expr: SlotExpr,
    stype: StorageType,
    rshift: u8, // in bytes
    is_write: bool,
    last_and: Option<U256>,
    last_or2: Option<Element<Label>>,
}
impl std::fmt::Debug for StorageElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slot_repr = self
            .slot
            .map(alloy_primitives::hex::encode)
            .unwrap_or_else(|| format!("{:?}", self.slot_expr));
        write!(
            f,
            "{}:{:?}:{}:{:?}",
            slot_repr, self.stype, self.rshift, self.last_and
        )
    }
}

type SlotHashMap = HashMap<SlotKey, Vec<Rc<RefCell<StorageElement>>>>;

fn known_constant_hash(
    data: &[u8],
    chunks: &[crate::evm::memory::MemoryChunk<Label>],
) -> Option<Slot> {
    let covered: usize = chunks
        .iter()
        .map(|chunk| chunk.dst_range.end - chunk.dst_range.start)
        .sum();
    if covered == data.len()
        && chunks
            .iter()
            .all(|chunk| matches!(chunk.src_label, Label::Constant))
    {
        Some(keccak256(data).0)
    } else {
        None
    }
}

fn full_word_label(
    chunks: &[crate::evm::memory::MemoryChunk<Label>],
    size: usize,
) -> Option<&Label> {
    match chunks {
        [chunk] if chunk.dst_range.start == 0 && chunk.dst_range.end == size => {
            Some(&chunk.src_label)
        }
        _ => None,
    }
}

fn word_slot_expr(
    data: Slot,
    chunks: &[crate::evm::memory::MemoryChunk<Label>],
) -> (SlotExpr, u32) {
    match full_word_label(chunks, 32) {
        Some(Label::Keccak(depth, expr)) => (expr.clone(), depth + 1),
        _ => (SlotExpr::Plain(data), 0),
    }
}

fn typed_dynamic_key_type(
    chunks: &[crate::evm::memory::MemoryChunk<Label>],
    key_size: usize,
) -> Option<DynSolType> {
    let mut saw_bytes = false;

    for chunk in chunks {
        if chunk.dst_range.start >= key_size {
            continue;
        }

        match &chunk.src_label {
            Label::Typed(DynSolType::String) => return Some(DynSolType::String),
            Label::Typed(DynSolType::Bytes) => saw_bytes = true,
            _ => {}
        }
    }

    if saw_bytes {
        Some(DynSolType::Bytes)
    } else {
        None
    }
}

fn normalize_slot_expr(slot_expr: &SlotExpr) -> (SlotKey, Option<Slot>, StorageType) {
    let mut current = slot_expr;
    let mut stype = StorageType::Base(DynSolType::Uint(256));

    loop {
        match current {
            SlotExpr::Mapping { key_type, base } => {
                stype = StorageType::Map(key_type.clone(), Box::new(stype));
                current = base;
            }
            SlotExpr::DynamicArray { base } => {
                stype = match stype {
                    StorageType::Base(inner) => {
                        StorageType::Base(DynSolType::Array(Box::new(inner)))
                    }
                    StorageType::Map(_, _) => {
                        StorageType::Base(DynSolType::Array(Box::new(DynSolType::Uint(256))))
                    }
                };
                current = base;
            }
            _ => return (current.slot_key(), current.canonical_slot(), stype),
        }
    }
}

fn storage_slot_element(storage: &StorageElement) -> Element<Label> {
    Element {
        data: storage.slot.unwrap_or_default(),
        label: match &storage.slot_expr {
            SlotExpr::Plain(_) => None,
            expr => Some(Label::Keccak(0, expr.clone())),
        },
    }
}

fn is_scalar_storage_type(stype: &StorageType) -> bool {
    matches!(
        stype,
        StorageType::Base(
            DynSolType::Bool
                | DynSolType::Address
                | DynSolType::String
                | DynSolType::Bytes
                | DynSolType::Uint(_)
                | DynSolType::Int(_)
                | DynSolType::FixedBytes(_)
        )
    )
}

fn is_suspicious_opaque_root(stype: &StorageType) -> bool {
    matches!(
        stype,
        StorageType::Base(DynSolType::String | DynSolType::Bytes)
    ) || matches!(
        stype,
        StorageType::Base(DynSolType::Uint(bits) | DynSolType::Int(bits)) if *bits >= 128
    ) || matches!(stype, StorageType::Base(DynSolType::FixedBytes(size)) if *size >= 16)
}

fn is_legitimate_packed_root(stype: &StorageType) -> bool {
    matches!(stype, StorageType::Base(DynSolType::Address))
}

fn looks_like_opaque_bitfield_slot(entries: &[(Selector, StorageElement)]) -> bool {
    let mut nonzero_offsets: BTreeSet<u8> = BTreeSet::new();
    let mut min_nonzero_offset: Option<u8> = None;
    let mut has_suspicious_root = false;
    let mut has_legitimate_root = false;

    for (_, entry) in entries {
        if !is_scalar_storage_type(&entry.stype) {
            return false;
        }

        if entry.rshift == 0 {
            has_suspicious_root |= is_suspicious_opaque_root(&entry.stype);
            has_legitimate_root |= is_legitimate_packed_root(&entry.stype);
        } else {
            nonzero_offsets.insert(entry.rshift);
            min_nonzero_offset =
                Some(min_nonzero_offset.map_or(entry.rshift, |current| current.min(entry.rshift)));
        }
    }

    if nonzero_offsets.len() < 4 || min_nonzero_offset.unwrap_or_default() < 16 {
        return false;
    }

    has_suspicious_root && !has_legitimate_root
}

#[derive(Default)]
struct Storage {
    loaded: SlotHashMap,
}
impl Storage {
    fn remove(&mut self, val: &Rc<RefCell<StorageElement>>) {
        let key = val.borrow().slot_key.clone();
        self.loaded.get_mut(&key).unwrap().retain(|x| x != val);
    }

    fn sstore(&mut self, slot: Element<Label>, rshift: u8, vtype: DynSolType) {
        let x = self.get(slot, true);
        x.borrow_mut().stype.set_type(vtype);
        x.borrow_mut().rshift = rshift;
    }

    fn sload(&mut self, slot: Element<Label>) -> Rc<RefCell<StorageElement>> {
        self.get(slot, false)
    }

    fn get(&mut self, slot: Element<Label>, is_write: bool) -> Rc<RefCell<StorageElement>> {
        let slot_expr = match slot.label {
            Some(Label::Keccak(_, expr)) => expr,
            _ => SlotExpr::Plain(slot.data),
        };
        let (slot_key, canonical_slot, stype) = normalize_slot_expr(&slot_expr);

        let v = Rc::new(RefCell::new(StorageElement {
            slot_key: slot_key.clone(),
            slot: canonical_slot,
            slot_expr,
            stype,
            rshift: 0,
            is_write,
            last_and: None,
            last_or2: None,
        }));
        self.loaded.entry(slot_key).or_default().push(v.clone());
        v
    }
}

fn analyze(
    vm: &mut Vm<Label, CallDataImpl<Label>>,
    st: &mut Storage,
    ret: StepResult<Label>,
) -> Result<Option<usize>, Box<dyn std::error::Error>> {
    match ret {
        StepResult {
            op: op::PUSH0..=op::PUSH32,
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::Constant);
        }

        StepResult {
            op: op::CODECOPY,
            args: [mem_off, ..],
            ..
        } => {
            let off: u32 = mem_off.try_into()?;
            if let Some(entry) = vm.memory.get_mut(off) {
                entry.label = Some(Label::Constant);
            }
        }

        StepResult {
            op:
                op::ADD
                | op::MUL
                | op::SUB
                | op::DIV
                | op::SDIV
                | op::MOD
                | op::SMOD
                | op::EXP
                | op::SIGNEXTEND
                | op::LT
                | op::GT
                | op::SLT
                | op::SGT
                | op::EQ
                | op::AND
                | op::OR
                | op::XOR
                | op::BYTE
                | op::SHL
                | op::SHR
                | op::SAR,
            args: [elabel!(Label::Constant), elabel!(Label::Constant), ..],
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::Constant);
        }

        StepResult {
            op: op::ADD | op::MUL | op::SUB | op::XOR | op::SHL,
            args:
                match_first_two!(
                    elabel!(lb @ (Label::Sloaded(_) | Label::Typed(_))),
                    elabel!(Label::Constant)
                ),
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(lb);
        }

        StepResult {
            op: op::NOT | op::ISZERO,
            args: [elabel!(Label::Constant), ..],
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::Constant);
        }

        StepResult {
            op: op::CALLVALUE, ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::Typed(DynSolType::Uint(256)));
        }

        //TODO signextend & byte
        StepResult {
            op: op::ISZERO,
            args: [elabel!(label @ Label::Typed(DynSolType::Bool)), ..],
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(label);
        }

        StepResult {
            op: op::SIGNEXTEND,
            args: [_, elabel!(label @ Label::Typed(_)), ..],
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(label);
        }

        StepResult {
            op: op::ADD | op::SUB,
            args: match_first_two!(elabel!(label @ Label::Keccak(_,_)), _),
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(label);
        }

        StepResult {
            op: op::SLOAD,
            args: [slot, ..],
            ..
        } => {
            *vm.stack.peek_mut()? = Element {
                label: Some(Label::Sloaded(st.sload(slot))),
                data: VAL_1_B,
            };
        }

        StepResult {
            op: op::JUMPI,
            args: [fa, ..],
            ..
        } => {
            let other_pc = usize::try_from(fa).expect("set to usize in vm.rs");
            return Ok(Some(other_pc));
        }

        StepResult {
            op: op::CALLER | op::ORIGIN | op::ADDRESS,
            ..
        } => {
            *vm.stack.peek_mut()? = Element {
                label: Some(Label::Typed(DynSolType::Address)),
                data: VAL_1_B,
            };
        }

        StepResult {
            op: op::ISZERO,
            args: [elabel!(Label::Sloaded(sl)), ..],
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::IsZero(sl));
        }

        StepResult {
            op: op::ISZERO,
            args: [elabel!(Label::IsZero(sl)), ..],
            ..
        } => {
            sl.borrow_mut().stype.set_type(DynSolType::Bool);
        }

        StepResult {
            op: op::SIGNEXTEND,
            args: [s0, elabel!(Label::Sloaded(sl)), ..],
            ..
        } => {
            if s0.data < VAL_32_B {
                let s0: u8 = s0.data[31];
                sl.borrow_mut()
                    .stype
                    .set_type(DynSolType::Int((s0 as usize + 1) * 8));
            }
        }

        StepResult {
            op: op::BYTE,
            args: [_, elabel!(Label::Sloaded(sl)), ..],
            ..
        } => sl.borrow_mut().stype.set_type(DynSolType::FixedBytes(32)),

        StepResult {
            op: op::EQ,
            args: match_first_two!(elabel!(Label::Typed(tp)), elabel!(Label::Sloaded(sl))),
            ..
        } => {
            sl.borrow_mut().stype.set_type(tp);
        }

        StepResult {
            op: op::OR,
            args:
                match_first_two!(elabel!(Label::Sloaded(sl)), tt @ Element{label: Some(Label::Typed(_) | Label::Constant), ..} ),
            ..
        } => {
            sl.borrow_mut().last_or2 = Some(tt);
            vm.stack.peek_mut()?.label = Some(Label::Sloaded(sl));
        }

        StepResult {
            op: op::AND,
            args: match_first_two!(elabel!(label @ Label::Typed(_)), _),
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(label);
        }

        StepResult {
            op: op::AND,
            args:
                match_first_two!(elabel!(Label::Sloaded(sl)), ot @ Element{label: Some(Label::Constant), ..} ),
            ..
        } => {
            let mask: U256 = ot.into();
            sl.borrow_mut().last_and = Some(mask);

            if let Some(t) = and_mask_to_type(mask) {
                sl.borrow_mut().stype.set_type(t);
            } else if mask == VAL_1 && sl.borrow().rshift == 0 {
                // string, check for SSO (only at rshift 0, not within packed fields)
                sl.borrow_mut().stype.set_type(DynSolType::String);
            }
            vm.stack.peek_mut()?.label = Some(Label::Sloaded(sl));
        }

        StepResult {
            op: op::SSTORE,
            args: [slot, value, ..],
            ..
        } => {
            if let Some(Label::Sloaded(ref sl)) = value.label {
                st.remove(sl);
            }

            match value.label {
                Some(Label::Typed(t)) => st.sstore(slot, 0, t),
                Some(Label::Sloaded(sl)) => {
                    let sbr = sl.borrow();
                    if let Some(lor) = &sbr.last_or2 {
                        if let Some(land) = sbr.last_and {
                            let tv = land.trailing_ones();

                            let shifted_mask = land >> tv;
                            let sz = shifted_mask.trailing_zeros();

                            let dt = match &lor.label {
                                Some(Label::Typed(tp)) => tp.clone(),
                                Some(Label::Sloaded(sl2)) => sl2.borrow().stype.get_internal_type(),
                                _ => {
                                    if sz == 160 {
                                        DynSolType::Address
                                    } else {
                                        DynSolType::Uint(sz)
                                    }
                                }
                            };
                            st.sstore(slot, (tv / 8) as u8, dt);
                        } else {
                            st.sstore(slot, 0, sbr.stype.get_internal_type());
                        }
                    } else {
                        // println!("SET {:?} TO {:?} | {:?}", slot, sbr.stype.get_internal_type(), sbr);
                        st.sstore(slot, 0, sbr.stype.get_internal_type());
                    }
                }
                _ => st.sstore(slot, 0, DynSolType::Uint(256)),
            }
        }

        StepResult {
            op: op::DIV,
            args: [elabel!(Label::Sloaded(sl)), ot, ..],
            ..
        } => {
            let mask: U256 = ot.into();

            if mask > VAL_1
                && (mask & (mask - VAL_1)).is_zero()
                && (mask.bit_len() - 1).is_multiple_of(8)
            {
                let slot = {
                    let storage = sl.borrow();
                    storage_slot_element(&storage)
                };
                let nl = st.sload(slot);
                let bl = mask.bit_len() - 1;
                nl.borrow_mut().rshift = (bl / 8) as u8;
                vm.stack.peek_mut()?.label = Some(Label::Sloaded(nl));

                // TODO: postprocess this
                // sl.borrow_mut().stype.set_type(if bl == 160 { DynSolType::Address } else { DynSolType::Uint(bl) });
            } else {
                vm.stack.peek_mut()?.label = Some(Label::Sloaded(sl));
            }
        }

        StepResult {
            op: op::SHR,
            args: [shift_amount, elabel!(Label::Sloaded(sl)), ..],
            ..
        } => {
            let shift: U256 = (&shift_amount).into();
            if !shift.is_zero() && shift.bit_len() <= 9 {
                let bits: usize = shift.to();
                if bits.is_multiple_of(8) {
                    let slot = {
                        let storage = sl.borrow();
                        storage_slot_element(&storage)
                    };
                    let nl = st.sload(slot);
                    nl.borrow_mut().rshift = (bits / 8) as u8;
                    vm.stack.peek_mut()?.label = Some(Label::Sloaded(nl));
                } else {
                    vm.stack.peek_mut()?.label = Some(Label::Sloaded(sl));
                }
            } else {
                vm.stack.peek_mut()?.label = Some(Label::Sloaded(sl));
            }
        }

        StepResult {
            op: op::KECCAK256,
            args: [fa, sa, ..],
            ..
        } => {
            let off = u32::try_from(fa)?;
            let sz = u32::try_from(sa)?;
            let (data, used) = vm.memory.load(off, sz);
            let constant_hash = known_constant_hash(&data, &used.chunks);

            if let Some(hash) = constant_hash {
                vm.stack.peek_mut()?.data = hash;
            }

            let mut depth = 0;
            let mut slot_expr = constant_hash.map_or_else(
                || SlotExpr::UnknownHash {
                    size: sz,
                    preimage: data.clone(),
                },
                |hash| SlotExpr::HashedConst {
                    hash,
                    preimage: data.clone(),
                },
            );

            if sz == 64 {
                let (_val, used) = vm.memory.load_element(off); // value
                let (sval, sused) = vm.memory.load_element(off + 32); // slot
                let key_type = match full_word_label(&used.chunks, 32) {
                    Some(Label::Typed(tp)) => tp.clone(),
                    _ => DynSolType::Uint(256),
                };
                let key_depth = match full_word_label(&used.chunks, 32) {
                    Some(Label::Keccak(d, _)) => d + 1,
                    _ => 0,
                };
                let (base_expr, base_depth) = word_slot_expr(sval.data, &sused.chunks);
                depth = key_depth.max(base_depth);
                if depth < 6 {
                    slot_expr = SlotExpr::Mapping {
                        key_type,
                        base: Box::new(base_expr),
                    };
                }
            } else if sz > 32 {
                let key_size = sz - 32;
                let (sval, sused) = vm.memory.load_element(off + key_size);
                let (base_expr, base_depth) = word_slot_expr(sval.data, &sused.chunks);
                let key_depth = used
                    .chunks
                    .iter()
                    .filter(|chunk| chunk.dst_range.start < key_size as usize)
                    .filter_map(|chunk| match chunk.src_label {
                        Label::Keccak(d, _) => Some(d + 1),
                        _ => None,
                    })
                    .max()
                    .unwrap_or(0);
                let key_type = typed_dynamic_key_type(&used.chunks, key_size as usize)
                    .unwrap_or(DynSolType::String);
                let tail_looks_like_slot = full_word_label(&sused.chunks, 32).is_some()
                    || sval.data[..31].iter().all(|b| *b == 0);
                depth = key_depth.max(base_depth);
                if tail_looks_like_slot && depth < 6 {
                    slot_expr = SlotExpr::Mapping {
                        key_type,
                        base: Box::new(base_expr),
                    };
                }
            } else if sz == 32 {
                let (val, used) = vm.memory.load_element(off); // value
                let (base_expr, base_depth) = word_slot_expr(val.data, &used.chunks);
                depth = base_depth;
                if depth < 6 {
                    slot_expr = SlotExpr::DynamicArray {
                        base: Box::new(base_expr),
                    };
                }
            }
            vm.stack.peek_mut()?.label = Some(Label::Keccak(depth, slot_expr));
        }
        _ => (),
    };
    Ok(None)
}

fn analyze_rec(
    mut vm: Vm<Label, CallDataImpl<Label>>,
    st: &mut Storage,
    gas_limit: u32,
    depth: u32,
) -> u32 {
    let mut gas_used = 0;

    while !vm.stopped {
        if cfg!(feature = "trace_storage") {
            println!("{vm:?}\n");
            println!("storage: {:?}\n", st.loaded);
        }
        let ret = match vm.step() {
            Ok(v) => v,
            Err(_e) => {
                // println!("{}", _e);
                break;
            }
        };
        gas_used += ret.gas_used;
        if gas_used > gas_limit {
            break;
        }

        match analyze(&mut vm, st, ret) {
            Err(_) => {
                // println!("errbrk");
                break;
            }
            Ok(Some(other_pc)) => {
                if depth < 8 && other_pc < vm.code.len() {
                    let mut cloned = vm.fork();
                    cloned.pc = other_pc;
                    gas_used += analyze_rec(cloned, st, (gas_limit - gas_used) / 2, depth + 1);
                }
            }
            Ok(None) => {}
        }
    }

    gas_used
}

fn analyze_one_function(
    code: &[u8],
    selector: Selector,
    arguments: &[DynSolType],
    is_fallback: bool,
    gas_limit: u32,
) -> SlotHashMap {
    if cfg!(feature = "trace_storage") {
        println!(
            "analyze selector {}\n",
            alloy_primitives::hex::encode(selector)
        );
    }

    let calldata = CallDataImpl::<Label>::new(selector, arguments);
    let mut vm = Vm::new(code, &calldata);

    let mut st = Storage::default();
    let mut gas_used = 0;

    if !is_fallback {
        if let Some(g) = execute_until_function_start(&mut vm, gas_limit) {
            gas_used += g;
        } else {
            return st.loaded;
        }
    }

    #[allow(unused_assignments)]
    if gas_used < gas_limit {
        gas_used += analyze_rec(vm, &mut st, gas_limit - gas_used, 0);
    }

    st.loaded
        .into_iter()
        .map(|(k, v)| {
            // Filter out impossible packed entries: full-slot/container types cannot start mid-slot.
            let v: Vec<_> = v
                .into_iter()
                .filter(|e| {
                    let br = e.borrow();
                    !(br.rshift > 0 && br.stype.requires_zero_offset())
                })
                .collect();
            let string_like_elements: Vec<_> = v
                .iter()
                .filter(|e| e.borrow().stype.is_string_like())
                .cloned()
                .collect();
            let map_elements: Vec<_> = v
                .clone()
                .into_iter()
                .filter(|e| {
                    let br = e.borrow();
                    if let StorageType::Map(_, _) = br.stype {
                        br.rshift == 0
                    } else {
                        false
                    }
                })
                .collect();
            (
                k,
                if !string_like_elements.is_empty() {
                    string_like_elements
                } else if !map_elements.is_empty() {
                    map_elements
                } else {
                    v
                },
            )
        })
        .collect()
}

pub fn contract_storage<I, D>(code: &[u8], functions: I, gas_limit: u32) -> Vec<StorageRecord>
where
    I: IntoIterator<Item = (Selector, usize, D)>,
    D: AsRef<[DynSolType]>,
{
    let real_gas_limit = if gas_limit == 0 {
        1e6 as u32
    } else {
        gas_limit
    };

    let mut slot_records: BTreeMap<(Slot, u8), Vec<(Selector, StorageElement)>> = BTreeMap::new();

    let functions: Vec<_> = functions.into_iter().collect();
    let selectors: BTreeSet<Selector> = functions.iter().map(|(sel, _, _)| *sel).collect();
    let mut fallback_selector: Selector = [0xff, 0xff, 0xff, 0xff];
    while selectors.contains(&fallback_selector) {
        let val = u32::from_be_bytes(fallback_selector) - 1;
        fallback_selector = val.to_be_bytes();
    }

    for &(sel, _, ref arguments) in &functions {
        let st = analyze_one_function(code, sel, arguments.as_ref(), false, real_gas_limit);
        for (_slot_key, loaded) in st.into_iter() {
            for ld in loaded.into_iter() {
                let v = (*ld).borrow();
                let Some(slot) = v.slot else {
                    continue;
                };
                slot_records
                    .entry((slot, v.rshift))
                    .or_default()
                    .push((sel, v.clone()));
            }
        }
    }

    // fallback()
    let st = analyze_one_function(code, fallback_selector, &[], true, real_gas_limit);
    for (_slot_key, loaded) in st.into_iter() {
        for ld in loaded.into_iter() {
            let v = (*ld).borrow();
            let Some(slot) = v.slot else {
                continue;
            };
            slot_records
                .entry((slot, v.rshift))
                .or_default()
                .push((fallback_selector, v.clone()));
        }
    }

    let mut normalized_slot_records = BTreeMap::new();
    let mut grouped_by_slot: BTreeMap<Slot, Vec<_>> = BTreeMap::new();

    for ((slot, offset), entries) in slot_records {
        grouped_by_slot
            .entry(slot)
            .or_default()
            .push((offset, entries));
    }

    for (slot, groups) in grouped_by_slot {
        let flattened: Vec<_> = groups
            .iter()
            .flat_map(|(_, entries)| entries.iter().cloned())
            .collect();

        if looks_like_opaque_bitfield_slot(&flattened) {
            let collapsed_entries: Vec<_> = flattened
                .into_iter()
                .map(|(selector, mut entry)| {
                    entry.rshift = 0;
                    entry.stype = StorageType::Base(DynSolType::FixedBytes(32));
                    (selector, entry)
                })
                .collect();
            normalized_slot_records.insert((slot, 0), collapsed_entries);
        } else {
            for (offset, entries) in groups {
                normalized_slot_records.insert((slot, offset), entries);
            }
        }
    }

    let mut ret: Vec<StorageRecord> = Vec::with_capacity(normalized_slot_records.len());

    for ((slot, offset), entries) in normalized_slot_records.into_iter() {
        let mut reads: BTreeSet<Selector> = BTreeSet::new();
        let mut writes: BTreeSet<Selector> = BTreeSet::new();

        let mut best_type: StorageType = StorageType::Base(DynSolType::Uint(256));
        let mut best_score = best_type.get_score();

        for (selector, selem) in entries.into_iter() {
            if selector != fallback_selector {
                if selem.is_write {
                    writes.insert(selector);
                } else {
                    reads.insert(selector);
                }
            }

            let tt = selem.stype;

            let score = tt.get_score();
            if score > best_score {
                // println!(
                //     "{:?} => {:?} ({} => {})",
                //     best_type,
                //     tt,
                //     best_score,
                //     score
                // );
                // best_type = selem.stype;
                best_type = tt;
                best_score = score;
            }
        }

        ret.push(StorageRecord {
            slot,
            offset,
            r#type: format!("{best_type:?}"),
            reads: reads.into_iter().collect(),
            writes: writes.into_iter().collect(),
        })
    }

    if cfg!(feature = "trace_storage") {
        for r in ret.iter() {
            println!(
                "slot {} off {}",
                alloy_primitives::hex::encode(r.slot),
                r.offset
            );
            println!(" type: {}", r.r#type);
            println!(
                " reads: {:?}",
                r.reads
                    .iter()
                    .map(alloy_primitives::hex::encode)
                    .collect::<Vec<_>>()
            );
            println!(
                " writes: {:?}",
                r.writes
                    .iter()
                    .map(alloy_primitives::hex::encode)
                    .collect::<Vec<_>>()
            );
        }
    }
    ret
}
