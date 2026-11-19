//! Runs a fallback on synthetic calldata. Storage reads and call results get distinct sentinel
//! words so a forwarding DELEGATECALL's target can be traced back to its source. Values carry
//! their [`Origin`], so that neither the probe's bytes nor a slot that depends on them or on
//! outside data is reported as a fact of the code.

use std::collections::{BTreeMap, BTreeSet};
use std::error;

use alloy_primitives::keccak256;

use crate::evm::{
    U256,
    calldata::CallData,
    element::Element,
    memory::MemoryChunks,
    op,
    vm::{StepResult, Vm},
};
use crate::{Address, Selector, Slot, Word};

/// `size` bytes of a storage slot, `offset` bytes from its low end (as `StorageRecord::offset`
/// counts).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct StorageRef {
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serialize::hex_bytes")
    )]
    pub slot: Slot,
    pub offset: u8,
    pub size: u8,
}

/// A lookup's callee address, or one 32-byte word of its arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "type", content = "data", bound(serialize = "T: AsRef<[u8]>"))
)]
pub enum ProxyValue<T> {
    /// A literal callee address or argument word.
    Constant(
        #[cfg_attr(
            feature = "serde",
            serde(serialize_with = "crate::serialize::hex_bytes")
        )]
        T,
    ),
    /// Bytes of a storage slot, in the word's low bytes.
    Storage(StorageRef),
}

/// Where a proxy's implementation address comes from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "type", content = "data")
)]
pub enum ProxySource {
    /// A fixed implementation address embedded in the code.
    Address(
        #[cfg_attr(
            feature = "serde",
            serde(serialize_with = "crate::serialize::hex_bytes")
        )]
        Address,
    ),
    /// An address in one of the proxy's own storage slots.
    Storage(StorageRef),
    /// The address another contract (a beacon, a registry) returns for this call: its callee,
    /// its selector and the argument words after it. No selector means empty calldata.
    Call {
        address: ProxyValue<Address>,
        #[cfg_attr(
            feature = "serde",
            serde(serialize_with = "crate::serialize::opt_selector")
        )]
        selector: Option<Selector>,
        args: Vec<ProxyValue<Word>>,
    },
    /// A target chosen by the incoming selector, such as a diamond's facet.
    PerSelector,
    /// The address an EIP-7702 account delegates to.
    Eip7702(
        #[cfg_attr(
            feature = "serde",
            serde(serialize_with = "crate::serialize::hex_bytes")
        )]
        Address,
    ),
}

const DELEGATION: [u8; 3] = [0xef, 0x01, 0x00];

const MAX_DEPTH: u32 = 10;
const GAS_LIMIT: u32 = 100_000;
const MAX_HASHED: u32 = 4096;
const MAX_INPUT: u32 = 65536;
const MAX_MEMBER: u64 = 8;

/// Where a value comes from, when not from the code alone. Branches fork on outside data only,
/// so dispatcher comparisons on the probe take one path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Origin {
    /// The probe's calldata.
    Calldata,
    /// Storage, a call's result, or the environment.
    Outside,
}

const CALLDATA: Option<Origin> = Some(Origin::Calldata);
const OUTSIDE: Option<Origin> = Some(Origin::Outside);

struct Probe {
    data: [u8; 68],
}

impl Probe {
    fn copy(&self, offset: U256, out: &mut [u8]) {
        if let Ok(off) = usize::try_from(offset)
            && off < self.data.len()
        {
            let n = (self.data.len() - off).min(out.len());
            out[..n].copy_from_slice(&self.data[off..off + n]);
        }
    }
}

impl CallData<Origin> for Probe {
    fn load32(&self, offset: U256) -> Element<Origin> {
        let mut data = [0; 32];
        self.copy(offset, &mut data);
        Element {
            data,
            label: CALLDATA,
        }
    }

    fn load(
        &self,
        offset: U256,
        size: U256,
    ) -> Result<(Vec<u8>, Option<Origin>), Box<dyn error::Error>> {
        let size = usize::try_from(size)?;
        if size > 32768 {
            return Err("calldata load size too large".into());
        }
        let mut data = vec![0; size];
        self.copy(offset, &mut data);
        Ok((data, CALLDATA))
    }

    fn len(&self) -> U256 {
        U256::from(self.data.len())
    }

    fn selector(&self) -> [u8; 4] {
        self.data[..4].try_into().expect("4 bytes at least")
    }
}

/// Shared by all forks. Each fork keeps its own last return word.
struct State<'a> {
    code: &'a [u8],
    calldata: &'a [u8],
    selector: Selector,
    loads: BTreeMap<Slot, [u8; 32]>, // persistent slot -> sentinel
    opaque_loads: usize,
    calls: Vec<([u8; 32], Option<ProxySource>)>,
    selector_hashes: BTreeSet<Slot>,
    targets: BTreeSet<ProxySource>,
}

fn sentinel(kind: u8, n: usize) -> [u8; 32] {
    let mut seed = [0u8; 9];
    seed[0] = kind;
    seed[1..].copy_from_slice(&(n as u64).to_be_bytes());
    keccak256(seed).0
}

/// Offset from the low end of a storage word.
fn byte_offset(word: &[u8; 32], bytes: &[u8]) -> Option<u8> {
    let start = word.windows(bytes.len()).position(|w| w == bytes)?;
    Some((32 - start - bytes.len()) as u8)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

impl State<'_> {
    fn load(&mut self, slot: Slot) -> [u8; 32] {
        let n = self.loads.len();
        *self.loads.entry(slot).or_insert_with(|| sentinel(1, n))
    }

    /// A read that names no fixed persistent slot: nothing traces back to it.
    fn opaque_load(&mut self) -> [u8; 32] {
        self.opaque_loads += 1;
        sentinel(3, self.opaque_loads)
    }

    fn call(&mut self, source: Option<ProxySource>) -> [u8; 32] {
        // Address-shaped, so that a decoder's check of an address's upper bits passes.
        let mut word = sentinel(2, self.calls.len());
        word[..12].fill(0);
        self.calls.push((word, source));
        word
    }

    /// An argument word that holds no probe bytes: a constant, or bytes of a storage slot.
    fn argument(&self, word: &[u8; 32], outside: bool) -> Option<ProxyValue<Word>> {
        if !outside {
            return Some(ProxyValue::Constant(*word));
        }
        let storage =
            |slot, offset, size| Some(ProxyValue::Storage(StorageRef { slot, offset, size }));
        let significant = &word[word.iter().take_while(|&&b| b == 0).count()..];
        for (&slot, loaded) in &self.loads {
            if loaded == word {
                return storage(slot, 0, 32);
            }
            if word[..12].iter().all(|&b| b == 0)
                && let Some(offset) = byte_offset(loaded, &word[12..])
            {
                return storage(slot, offset, 20);
            }
            if significant.len() >= 4
                && let Some(offset) = byte_offset(loaded, significant)
            {
                return storage(slot, offset, significant.len() as u8);
            }
        }
        None
    }

    fn call_source(
        &self,
        target: &Element<Origin>,
        input: &[u8],
        chunks: &MemoryChunks<Origin>,
    ) -> Option<ProxySource> {
        let address = match self.resolve(target)? {
            ProxySource::Address(address) => ProxyValue::Constant(address),
            ProxySource::Storage(slot) => ProxyValue::Storage(slot),
            _ => return None,
        };
        let (selector, payload) = if input.is_empty() {
            (None, input)
        } else {
            let (selector, rest) = input.split_first_chunk::<4>()?;
            if chunks.chunks.iter().any(|c| c.dst_range.start < 4) {
                return None;
            }
            (Some(*selector), rest)
        };
        let (words, trailing) = payload.as_chunks::<32>();
        if !trailing.is_empty() {
            return None;
        }
        let mut args = Vec::new();
        let mut per_selector = false;
        for (i, word) in words.iter().enumerate() {
            if word[..4] == self.selector && word[4..] == [0; 28] {
                per_selector = true;
            } else {
                let start = 4 + i * 32;
                let from = |origin| {
                    chunks.chunks.iter().any(|c| {
                        c.src_label == origin
                            && c.dst_range.start < start + 32
                            && c.dst_range.end > start
                    })
                };
                if from(Origin::Calldata) {
                    return None;
                }
                args.push(self.argument(word, from(Origin::Outside))?);
            }
        }
        Some(if per_selector {
            ProxySource::PerSelector
        } else {
            ProxySource::Call {
                address,
                selector,
                args,
            }
        })
    }

    fn keyed_by_selector(&self, slot: Slot) -> bool {
        let value = U256::from_be_bytes(slot);
        (0..MAX_MEMBER).any(|member| {
            value
                .checked_sub(U256::from(member))
                .is_some_and(|base| self.selector_hashes.contains(&base.to_be_bytes::<32>()))
        })
    }

    fn resolve(&self, target: &Element<Origin>) -> Option<ProxySource> {
        let address = target.data.last_chunk::<20>()?;
        if address.iter().all(|&b| b == 0) {
            return None;
        }
        for (&slot, loaded) in &self.loads {
            if let Some(offset) = byte_offset(loaded, address) {
                return Some(if self.keyed_by_selector(slot) {
                    ProxySource::PerSelector
                } else {
                    ProxySource::Storage(StorageRef {
                        slot,
                        offset,
                        size: 20,
                    })
                });
            }
        }
        for (returned, source) in &self.calls {
            if byte_offset(returned, address).is_some() {
                return source.clone();
            }
        }
        // A shorter PUSH holds an address with leading zero bytes without them (EIP-1167's
        // vanity form); four bytes at least, so a small number is not taken for one.
        let significant = &address[address.iter().take_while(|&&b| b == 0).count()..];
        if target.label.is_none() && significant.len() >= 4 && contains(self.code, significant) {
            return Some(ProxySource::Address(*address));
        }
        None
    }
}

fn u32_of(el: &Element<Origin>) -> Option<u32> {
    u32::try_from(el).ok()
}

/// Outside data wins over the probe's, as it is what a branch forks on.
fn origin_of(chunks: &MemoryChunks<Origin>) -> Option<Origin> {
    chunks.chunks.iter().map(|c| c.src_label).max()
}

fn observe(
    vm: &mut Vm<Origin, Probe>,
    st: &mut State,
    last_return: &mut Option<[u8; 32]>,
    ret: StepResult<Origin>,
) -> Result<Option<usize>, Box<dyn error::Error>> {
    match ret.op {
        op::RETURNDATASIZE => {
            let called = last_return.is_some();
            let mut data = [0; 32];
            data[31] = if called { 32 } else { 0 };
            *vm.stack.peek_mut()? = Element {
                data,
                label: if called { OUTSIDE } else { None },
            };
        }

        op::SLOAD | op::TLOAD => {
            // The VM's stand-ins for outside data and the probe's bytes name no fixed slot, but
            // a mapping keyed by the selector names the facet for any selector. A transient
            // slot does not outlive the transaction.
            let slot = ret.args[0].data;
            let fixed = ret.op == op::SLOAD
                && match ret.args[0].label {
                    None => true,
                    Some(Origin::Calldata) => st.keyed_by_selector(slot),
                    Some(Origin::Outside) => false,
                };
            let word = if fixed {
                st.load(slot)
            } else {
                st.opaque_load()
            };
            *vm.stack.peek_mut()? = Element {
                data: word,
                label: OUTSIDE,
            };
        }

        op::KECCAK256 => {
            if let (Some(off), Some(size)) = (u32_of(&ret.args[0]), u32_of(&ret.args[1]))
                && size <= MAX_HASHED
            {
                let (input, chunks) = vm.memory.load(off, size);
                let hash = keccak256(&input).0;
                *vm.stack.peek_mut()? = Element {
                    data: hash,
                    label: origin_of(&chunks),
                };
                // A mapping keyed by the selector, or nested in one, picks a slot per selector.
                // The key comes first in Solidity and second in Vyper; bytes4 and uint32 keys
                // pad on opposite sides.
                let (words, _) = input.as_chunks::<32>();
                if size == 64
                    && words.iter().any(|word| {
                        (word[..4] == st.selector && word[4..] == [0; 28])
                            || (word[..28] == [0; 28] && word[28..] == st.selector)
                            || st.keyed_by_selector(*word)
                    })
                {
                    st.selector_hashes.insert(hash);
                }
            } else {
                // The VM's placeholder is not the hash.
                vm.stack.peek_mut()?.label = OUTSIDE;
            }
        }

        op::CALL | op::CALLCODE | op::DELEGATECALL | op::STATICCALL => {
            // exargs: gas, input offset, input size, output offset, output size
            let input = match (u32_of(&ret.exargs[1]), u32_of(&ret.exargs[2])) {
                (Some(off), Some(size)) if size <= MAX_INPUT => Some(vm.memory.load(off, size)),
                _ => None,
            };
            if ret.op == op::DELEGATECALL
                && input
                    .as_ref()
                    .is_some_and(|(data, _)| data.starts_with(st.calldata))
            {
                if let Some(target) = st.resolve(&ret.args[0]) {
                    st.targets.insert(target);
                }
                vm.stopped = true;
            } else {
                vm.stack.peek_mut()?.label = OUTSIDE;
                // An input that cannot be read names no lookup.
                let source =
                    input.and_then(|(data, chunks)| st.call_source(&ret.args[0], &data, &chunks));
                let word = st.call(source);
                if let (Some(off), Some(size)) = (u32_of(&ret.exargs[3]), u32_of(&ret.exargs[4]))
                    && size >= 32
                {
                    vm.memory.store(off, word.to_vec(), OUTSIDE);
                }
                *last_return = Some(word);
            }
        }

        op::RETURNDATACOPY => {
            if let (Some(word), Some(off), Some(src), Some(size)) = (
                *last_return,
                u32_of(&ret.args[0]),
                u32_of(&ret.args[1]),
                u32_of(&ret.exargs[0]),
            ) && size > 0
                && size <= 2048
            {
                let mut data = vec![0; size as usize];
                let src = src as usize;
                if src < 32 {
                    let n = (32 - src).min(data.len());
                    data[..n].copy_from_slice(&word[src..src + n]);
                }
                vm.memory.store(off, data, OUTSIDE);
            }
        }

        op::JUMPI => {
            let other_pc = usize::try_from(&ret.args[0])?;
            if other_pc != 0 && ret.args[1].label == OUTSIDE {
                return Ok(Some(other_pc));
            }
        }

        op::ADDRESS
        | op::ORIGIN
        | op::CALLER
        | op::CALLVALUE
        | op::COINBASE
        | op::TIMESTAMP
        | op::NUMBER
        | op::PREVRANDAO
        | op::GASLIMIT
        | op::CHAINID
        | op::BASEFEE
        | op::BLOBBASEFEE
        | op::GASPRICE
        | op::BALANCE
        | op::SELFBALANCE
        | op::EXTCODESIZE
        | op::EXTCODEHASH
        | op::BLOCKHASH
        | op::BLOBHASH
        | op::GAS => {
            vm.stack.peek_mut()?.label = OUTSIDE;
        }

        op::CALLDATASIZE => {
            vm.stack.peek_mut()?.label = CALLDATA;
        }

        op::ADD..=op::SIGNEXTEND | op::LT..=op::CLZ => {
            vm.stack.peek_mut()?.label = ret
                .args
                .iter()
                .chain(&ret.exargs)
                .filter_map(|arg| arg.label)
                .max();
        }

        op::MLOAD => {
            if let Some(chunks) = &ret.memory_load {
                vm.stack.peek_mut()?.label = origin_of(chunks);
            }
        }

        _ => {}
    }
    Ok(None)
}

fn explore(
    mut vm: Vm<Origin, Probe>,
    st: &mut State,
    mut last_return: Option<[u8; 32]>,
    gas_limit: u32,
    depth: u32,
) -> u32 {
    let mut gas_used: u32 = 0;
    while !vm.stopped {
        let ret = match vm.step() {
            Ok(v) => v,
            Err(_) => break,
        };
        gas_used = gas_used.saturating_add(ret.gas_used);
        if gas_used > gas_limit {
            break;
        }
        match observe(&mut vm, st, &mut last_return, ret) {
            Err(_) => break,
            Ok(Some(other_pc)) => {
                if depth < MAX_DEPTH && other_pc < vm.code.len() {
                    let mut forked = vm.fork();
                    forked.pc = other_pc;
                    let budget = (gas_limit - gas_used) / 2;
                    let spent = explore(forked, st, last_return, budget, depth + 1);
                    gas_used = gas_used.saturating_add(spent);
                }
            }
            Ok(None) => {}
        }
    }
    gas_used
}

/// The probe's selector: the highest value from 0xeeeeeeee down that is nowhere in the code as 4
/// bytes, so that no dispatcher matches it, whether it pushes its selectors or reads them from a
/// table in the code (Vyper's dense selector table). A selector with a leading zero byte, pushed
/// shorter, is far below.
fn probe_selector(code: &[u8]) -> Selector {
    const START: u32 = 0xeeee_eeee;
    // Each window takes one value at most, so a free one is at most this far below START.
    let lowest = START.saturating_sub(u32::try_from(code.len()).unwrap_or(u32::MAX));
    let mut taken: Vec<u32> = code
        .windows(4)
        .map(|w| u32::from_be_bytes(w.try_into().expect("4 bytes")))
        .filter(|v| (lowest..=START).contains(v))
        .collect();
    taken.sort_unstable();
    let mut n = START;
    while taken.binary_search(&n).is_ok() {
        n -= 1;
    }
    n.to_be_bytes()
}

/// Where the implementation address comes from, for each DELEGATECALL in the fallback that
/// passes the incoming calldata on. An empty list means no source was found.
pub fn contract_proxy(code: &[u8]) -> Vec<ProxySource> {
    if let Some((prefix, address)) = code.split_last_chunk::<20>()
        && prefix == DELEGATION
    {
        return vec![ProxySource::Eip7702(*address)];
    }
    let selector = probe_selector(code);
    let mut data = [0xee; 68];
    data[..4].copy_from_slice(&selector);
    data[36..].fill(0xdd);
    let probe = Probe { data };

    let mut st = State {
        code,
        calldata: &probe.data,
        selector,
        loads: BTreeMap::new(),
        opaque_loads: 0,
        calls: Vec::new(),
        selector_hashes: BTreeSet::new(),
        targets: BTreeSet::new(),
    };
    let vm = Vm::new(code, &probe);
    explore(vm, &mut st, None, GAS_LIMIT, 0);
    st.targets.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;

    fn sources(code: &str) -> Vec<ProxySource> {
        contract_proxy(&hex::decode(code).unwrap())
    }

    fn storage(slot: Slot) -> ProxySource {
        ProxySource::Storage(StorageRef {
            slot,
            offset: 0,
            size: 20,
        })
    }

    /// Forwards the calldata to the word `push` leaves on the stack.
    fn forward(push: &str) -> String {
        format!("{push}365f5f375f5f365f845af400")
    }

    const LOOKUP: &str = "73cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd";

    /// Calls `LOOKUP` with selector 0x12345678 and the word `push` leaves, then forwards the
    /// calldata to the address it returns.
    fn registry(push: &str) -> String {
        format!(
            "631234567860e01b608052{push}6084526020608060246080{LOOKUP}5afa50365f5f375f5f365f6080515af400"
        )
    }

    fn word(v: u8) -> Word {
        let mut w = [0; 32];
        w[31] = v;
        w
    }

    #[test]
    fn slot_depending_on_outside_data_or_probe_is_not_fixed() {
        // sload(keccak256(key . 1))
        let mapping = |key: &str| forward(&format!("{key}5f52600160205260405f2054"));
        let mut preimage = [0; 64];
        preimage[31] = 7;
        preimage[63] = 1;
        assert_eq!(sources(&mapping("6007")), [storage(keccak256(preimage).0)]);
        assert_eq!(sources(&mapping("30")), []); // address()
        assert_eq!(sources(&mapping("600435")), []); // calldataload(4)
        // A mapping keyed by msg.sig names the facet for every selector.
        assert_eq!(
            sources(&mapping("5f3563ffffffff60e01b16")),
            [ProxySource::PerSelector]
        );
        // keccak256(abi.encode(7, msg.sig)): the key second, as Vyper's mappings have it.
        assert_eq!(
            sources(&forward("60075f525f3563ffffffff60e01b1660205260405f2054")),
            [ProxySource::PerSelector]
        );
        // storage[msg.sig][7]: nested in a mapping keyed by the selector.
        assert_eq!(
            sources(&forward(
                "5f3563ffffffff60e01b165f52600560205260405f2060205260075f5260405f2054"
            )),
            [ProxySource::PerSelector]
        );
        // sload(keccak256(0, 0x2000)): longer than is hashed.
        assert_eq!(sources(&forward("6120005f2054")), []);
    }

    #[test]
    fn unreadable_call_input_is_no_lookup() {
        // staticcall(gas, LOOKUP, 0x80, size, 0x80, 0x20), then forward to the returned address.
        let lookup =
            |size: &str| format!("60206080{size}6080{LOOKUP}5afa50365f5f375f5f365f6080515af400");
        assert_eq!(
            sources(&lookup("5f")),
            [ProxySource::Call {
                address: ProxyValue::Constant([0xcd; 20]),
                selector: None,
                args: vec![],
            }]
        );
        assert_eq!(sources(&lookup("62100000")), []); // 1 MiB
    }

    #[test]
    fn dynamic_lookup_selector_is_not_reported_as_fixed() {
        // Copy msg.sig to 0x80 and use those four bytes as the registry call input.
        let code =
            format!("60045f6080376020608060046080{LOOKUP}5afa50365f5f375f5f365f6080515af400");
        assert_eq!(sources(&code), []);
    }

    #[test]
    fn caller_dependent_target_is_not_a_literal_address() {
        assert_eq!(
            sources(&forward(LOOKUP)),
            [ProxySource::Address([0xcd; 20])]
        );
        // XOR with CALLER leaves the literal unchanged when the sampled caller is zero.
        assert_eq!(sources(&forward(&format!("{LOOKUP}3318"))), []);
    }

    #[test]
    fn argument_from_probe_is_no_constant() {
        assert_eq!(
            sources(&registry("6042")),
            [ProxySource::Call {
                address: ProxyValue::Constant([0xcd; 20]),
                selector: Some([0x12, 0x34, 0x56, 0x78]),
                args: vec![ProxyValue::Constant(word(0x42))],
            }]
        );
        // and(calldataload(4), 2**160 - 1)
        let masked = "60043573ffffffffffffffffffffffffffffffffffffffff16";
        assert_eq!(sources(&registry(masked)), []);
        assert_eq!(sources(&registry("36")), []); // calldatasize()
    }

    #[test]
    fn probe_avoids_selectors_anywhere_in_the_code() {
        assert_eq!(
            probe_selector(&hex::decode("eeeeeeeeed").unwrap()),
            [0xee, 0xee, 0xee, 0xec]
        );
        // Compares the selector with 4 bytes of a table in the code, as Vyper's dense selector
        // table does, and stops on a match; else forwards to sload(0). The table holds 0xeeeeeeee,
        // which the probe would have taken when it avoided PUSH4 operands only.
        let table = concat!(
            "600460245f39",                 // codecopy(0, 0x24, 4)
            "5f5160e01c5f3560e01c14602257", // jumpi(0x22, eq(mload(0) >> 224, selector))
            "5f54365f5f375f5f365f845af400", // forward to sload(0)
            "5b00",                         // 0x22: stop
            "eeeeeeee",                     // 0x24: the table
        );
        assert_eq!(sources(table), [storage([0; 32])]);
    }

    #[test]
    fn transient_slot_is_no_source() {
        assert_eq!(sources(&forward("5f54")), [storage([0; 32])]);
        assert_eq!(sources(&forward("5f5c")), []);
    }
}
