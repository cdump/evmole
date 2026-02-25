use std::collections::BTreeMap;

mod classify;
mod resolve;

/// Event selector is a 32-byte keccak256 hash of the event signature
pub type EventSelector = [u8; 32];

/// Coarse-grained category for `LOGx` topic0 extraction complexity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum EventLogClass {
    /// Topic0 resolves to a single PUSH32 inside the same basic block.
    SameBlockSinglePush32,
    /// Topic0 resolves to PUSH32 in the same block, but multiple PUSH32 exist before LOG.
    SameBlockMultiPush32,
    /// Topic0 comes from predecessor blocks (symbol is Before(n) at LOG site).
    CrossBlockBefore,
    /// Any other source (non-PUSH32 producer or unresolved pattern).
    Other,
}

/// Per-`LOGx` classification record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct EventLogClassRecord {
    pub log_pc: usize,
    pub block_start: usize,
    pub class: EventLogClass,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventExtractionStats {
    pub selectors_total: u64,
    pub selectors_after_mutability_prune: u64,
    pub selectors_pruned_view_or_pure: u64,
    pub jump_classify_cache_hits: u64,
    pub jump_classify_cache_misses: u64,
    pub entry_state_cache_hits: u64,
    pub entry_state_cache_misses: u64,
    pub jump_classify_can_fork_true: u64,
    pub jump_classify_can_fork_false: u64,
    pub probe_cache_hits: u64,
    pub probe_cache_misses: u64,
    pub static_dead_other_prunes: u64,
    pub static_dead_current_prunes: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventExecutionProfile {
    pub states_pushed: u64,
    pub states_popped: u64,
    pub queue_peak: usize,
    pub state_limit_breaks: u64,
    pub visited_cap_hits: u64,

    pub jump_total: u64,
    pub jump_visited_breaks: u64,

    pub jumpi_total: u64,
    pub jumpi_visited_breaks: u64,
    pub jumpi_invalid_other_pc: u64,
    pub jumpi_unreachable_both: u64,
    pub jumpi_unreachable_current: u64,
    pub jumpi_unreachable_other: u64,
    pub jumpi_fork_throttled: u64,
    pub jumpi_fork_deduped: u64,
    pub jumpi_decision_keep: u64,
    pub jumpi_decision_switch: u64,
    pub jumpi_decision_fork: u64,

    pub context_start_by_pc: BTreeMap<usize, u64>,
    pub jumpi_by_pc: BTreeMap<usize, u64>,
    pub jumpi_can_fork_true_by_pc: BTreeMap<usize, u64>,
    pub jumpi_can_fork_false_by_pc: BTreeMap<usize, u64>,
    pub jumpi_cache_hit_by_pc: BTreeMap<usize, u64>,
    pub jumpi_cache_miss_by_pc: BTreeMap<usize, u64>,
    pub jumpi_decision_keep_by_pc: BTreeMap<usize, u64>,
    pub jumpi_decision_switch_by_pc: BTreeMap<usize, u64>,
    pub jumpi_decision_fork_by_pc: BTreeMap<usize, u64>,
    pub jumpi_invalid_other_pc_by_pc: BTreeMap<usize, u64>,
    pub jumpi_unreachable_both_by_pc: BTreeMap<usize, u64>,
    pub jumpi_unreachable_current_by_pc: BTreeMap<usize, u64>,
    pub jumpi_unreachable_other_by_pc: BTreeMap<usize, u64>,
    pub jumpi_fork_throttled_by_pc: BTreeMap<usize, u64>,
    pub jumpi_fork_deduped_by_pc: BTreeMap<usize, u64>,
    pub jumpi_visited_breaks_by_pc: BTreeMap<usize, u64>,
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Checks if a 32-byte value looks like a keccak256 hash (event selector).
fn is_plausible_event_hash(val: &[u8; 32]) -> bool {
    if val == &[0u8; 32] {
        return false;
    }
    if val[..6] == [0u8; 6] {
        return false;
    }
    if val[26..] == [0u8; 6] {
        return false;
    }
    if is_known_non_event_constant(val) {
        return false;
    }
    let mut zero_run = 0u8;
    let mut ff_run = 0u8;
    for &b in val {
        if b == 0 {
            zero_run += 1;
            if zero_run >= 4 {
                return false;
            }
        } else {
            zero_run = 0;
        }
        if b == 0xff {
            ff_run += 1;
            if ff_run >= 4 {
                return false;
            }
        } else {
            ff_run = 0;
        }
    }
    true
}

macro_rules! hex_bytes32 {
    ($s:literal) => {{
        const BYTES: [u8; 32] = {
            const fn hex_val(c: u8) -> u8 {
                match c {
                    b'0'..=b'9' => c - b'0',
                    b'a'..=b'f' => c - b'a' + 10,
                    b'A'..=b'F' => c - b'A' + 10,
                    _ => panic!("invalid hex char"),
                }
            }
            let s = $s.as_bytes();
            let mut out = [0u8; 32];
            let mut i = 0;
            while i < 32 {
                out[i] = (hex_val(s[i * 2]) << 4) | hex_val(s[i * 2 + 1]);
                i += 1;
            }
            out
        };
        BYTES
    }};
}

/// Well-known non-event keccak256 constants that commonly appear as PUSH32
/// but are NOT event selectors. These include:
/// - OpenZeppelin AccessControl role hashes (PAUSER_ROLE, MINTER_ROLE, etc.)
/// - EIP-712 type hashes (domain separator, Permit, Delegation)
/// - EIP-712 version/name hashes (keccak256("1"), keccak256(""))
///
/// Curated from production false-positive analysis across 1730+ contracts.
/// Each entry eliminates FP in 20-65 contracts with zero TP loss.
#[rustfmt::skip]
const KNOWN_NON_EVENT_HASHES: &[[u8; 32]] = &[
    // keccak256("PAUSER_ROLE")
    hex_bytes32!("65d7a28e3265b37a6474929f336521b332c1681b933f6cb9f3376673440d862a"),
    // keccak256("MINTER_ROLE")
    hex_bytes32!("9f2df0fed2c77648de5860a4cc508cd0818c85b8b8a1ab4ceeef8d981c8956a6"),
    // keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)")
    hex_bytes32!("8b73c3c69bb8fe3d512ecc4cf759cc79239f7b179b0ffacaa9a75d522b39400f"),
    // keccak256("ADMIN_ROLE") — OpenZeppelin AccessControl (not DEFAULT_ADMIN_ROLE which is 0x00)
    hex_bytes32!("a49807205ce4d355092ef5a8a18f56e8913cf4a201fbe287825b095693c21775"),
    // keccak256("1") — EIP-712 version hash
    hex_bytes32!("c89efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc6"),
    // keccak256("") — empty hash, used for EXTCODEHASH sentinel
    hex_bytes32!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"),
    // keccak256("Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)")
    hex_bytes32!("6e71edae12b1b97f4d1f60370fef10105fa2faae0126114a169c64845d6126c9"),
    // keccak256("UPGRADER_ROLE")
    hex_bytes32!("189ab7a9244df0848122154315af71fe140f3db0fe014031783b0946b8c9d2e3"),
    // keccak256("OPERATOR_ROLE")
    hex_bytes32!("97667070c54ef182b0f5858b034beac1b6f3089aa2d3188bb1e8929f4fa9b929"),
    // keccak256("SNAPSHOT_ROLE")
    hex_bytes32!("5fdbd35e8da83ee755d5e62a539e5ed7f47126abede0b8b10f9ea43dc6eed07f"),
    // keccak256("BURNER_ROLE")
    hex_bytes32!("3c11d16cbaffd01df69ce1c404f6340ee057498f5f00246190ea54220576a848"),
    // keccak256("EXECUTOR_ROLE")
    hex_bytes32!("d8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e63"),
    // keccak256("CANCELLER_ROLE")
    hex_bytes32!("fd643c72710c63c0180259aba6b2d05451e3591a24e58b62239378085726f783"),
    // keccak256("PROPOSER_ROLE")
    hex_bytes32!("b09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc1"),
    // keccak256("TIMELOCK_ADMIN_ROLE")
    hex_bytes32!("5f58e3a2316349923ce3780f8d587db2d72378aed66a8261c916544fa6846ca5"),
    // keccak256("PREDICATE_ROLE")
    hex_bytes32!("12ff340d0cd9c652c747ca35727e68c547d0f0bfa7758c2e59b9aadc721a202b"),
    // keccak256("DEPOSITOR_ROLE")
    hex_bytes32!("8f4f2da22e8ac8f11e15f9fc141cddbb5deea8800186560abb6e68c5496619a9"),
    // keccak256("URI_SETTER_ROLE")
    hex_bytes32!("7804d923f43a17d325d77e781528e0793b2edd7d8aa4a317c18bf4cd7da5db7e"),
    // keccak256("MANAGER_ROLE")
    hex_bytes32!("241ecf16d79d0f8dbfb92cbc07fe17840425976cf0667f022fe9877caa831b08"),
    // keccak256("GOVERNANCE_ROLE")
    hex_bytes32!("71840dc4906352362b0cdaf79870196c8e42acafade72d5d5a6d59291253ceb1"),
    // keccak256("Delegation(address delegatee,uint256 nonce,uint256 expiry)")
    hex_bytes32!("e48329057bfd03d55e49b547132e39cffd9c1820ad7b9d4c5307691425d15adf"),
    // keccak256("MetaTransaction(uint256 nonce,address from,bytes functionSignature)")
    hex_bytes32!("2d0335ab174d301747ad37e568a4556fead940e3d2551a80ae05629fc44e80b0"),
    // keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract,bytes32 salt)")
    hex_bytes32!("d87cd6ef79d4e2b95e15ce8abf732db51ec771f1ca2edccf22a46c729ac56472"),
    // keccak256("EIP712Domain(string name,uint256 chainId,address verifyingContract)")
    hex_bytes32!("b1188b85de397c4c89df42e52e9bb3e936e8e7a3983bbb543b71ba9ea5234396"),
    // keccak256("KEEPER_ROLE")
    hex_bytes32!("fc8737ab85eb45125971625a9ebdb75cc78e01d5c1fa80c4c6e5203f47bc4fab"),
    // keccak256("GUARDIAN_ROLE")
    hex_bytes32!("55435dd261a4b9b3364963f7738a7a662ad9c84396d64be3365284bb7f0a5041"),
    // keccak256("RELAYER_ROLE")
    hex_bytes32!("e2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc4"),
];

fn is_known_non_event_constant(val: &[u8; 32]) -> bool {
    KNOWN_NON_EVENT_HASHES.iter().any(|known| known == val)
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

fn contract_events_internal(code: &[u8]) -> Vec<EventSelector> {
    if code.is_empty() {
        return Vec::new();
    }
    let Ok((index, classified)) = std::panic::catch_unwind(|| classify::classify_log_sites(code))
    else {
        return Vec::new();
    };
    resolve::resolve_classified_log_sites(code, &index, &classified)
}

/// Extracts all event selectors from contract bytecode.
pub fn contract_events(code: &[u8]) -> Vec<EventSelector> {
    contract_events_internal(code)
}

pub fn contract_events_with_stats(code: &[u8]) -> (Vec<EventSelector>, EventExtractionStats) {
    (
        contract_events_internal(code),
        EventExtractionStats::default(),
    )
}

pub fn contract_events_with_profile(
    code: &[u8],
) -> (
    Vec<EventSelector>,
    EventExtractionStats,
    EventExecutionProfile,
) {
    (
        contract_events_internal(code),
        EventExtractionStats::default(),
        EventExecutionProfile::default(),
    )
}

/// Classifies each `LOGx` site by topic0 source complexity.
///
/// This is a lightweight diagnostic helper intended for analysis/demo usage.
pub fn contract_event_log_classes(code: &[u8]) -> Vec<EventLogClassRecord> {
    if code.is_empty() {
        return Vec::new();
    }
    let Ok((_, classified)) = std::panic::catch_unwind(|| classify::classify_log_sites(code))
    else {
        return Vec::new();
    };
    classified
        .into_iter()
        .map(|v| EventLogClassRecord {
            log_pc: v.site.pc,
            block_start: v.site.block_start,
            class: map_log_site_class(code, &v),
        })
        .collect()
}

fn map_log_site_class(code: &[u8], site: &classify::ClassifiedLogSite) -> EventLogClass {
    match site.class {
        classify::LogSiteClass::Push32 { .. } => {
            use crate::evm::{code_iterator::iterate_code, op};
            let count = iterate_code(code, site.site.block_start, Some(site.site.pc))
                .filter(|(_, cop)| cop.op == op::PUSH32)
                .count();
            if count <= 1 {
                EventLogClass::SameBlockSinglePush32
            } else {
                EventLogClass::SameBlockMultiPush32
            }
        }
        classify::LogSiteClass::PushN { .. } | classify::LogSiteClass::MloadCodecopy { .. } => {
            EventLogClass::Other
        }
        classify::LogSiteClass::CrossBlock { .. } => EventLogClass::CrossBlockBefore,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::evm::op;

    fn append_log1(code: &mut Vec<u8>, selector: [u8; 32]) {
        code.push(op::PUSH32);
        code.extend_from_slice(&selector);
        // stack: [topic0]
        code.extend_from_slice(&[op::PUSH1, 0x00, op::PUSH1, 0x00, op::LOG1]);
    }

    fn append_single_selector_dispatch(code: &mut Vec<u8>, selector: [u8; 4]) -> usize {
        code.extend_from_slice(&[
            op::PUSH1,
            0x00,
            op::CALLDATALOAD,
            op::PUSH1,
            0xE0,
            op::SHR,
            op::PUSH4,
        ]);
        code.extend_from_slice(&selector);
        code.push(op::EQ);
        code.extend_from_slice(&[op::PUSH1, 0x00]);
        let entry_patch = code.len() - 1;
        code.push(op::JUMPI);
        code.push(op::STOP);
        entry_patch
    }

    fn make_plausible_hash() -> [u8; 32] {
        // A value that passes is_plausible_event_hash: no long zero/ff runs, non-zero prefix/suffix.
        [0xabu8; 32]
    }

    // --- Public API tests ---

    #[test]
    fn test_simple_log1() {
        let selector = [0xab; 32];
        let mut code = Vec::new();
        append_log1(&mut code, selector);
        code.push(op::STOP);

        let events = contract_events(&code);
        assert_eq!(events, vec![selector]);
    }

    #[test]
    fn test_require_guarded_event() {
        let function_selector = [0xaa, 0xbb, 0xcc, 0xdd];
        let event_selector = [0x42; 32];

        let mut code = Vec::new();
        let entry_patch = append_single_selector_dispatch(&mut code, function_selector);

        let function_entry = code.len();
        code[entry_patch] = u8::try_from(function_entry).unwrap();
        code.push(op::JUMPDEST);

        // Emulate a require guard:
        // if (!cond) revert(); else emit LOG1(topic0)
        code.extend_from_slice(&[op::PUSH1, 0x00]); // cond = 0
        code.extend_from_slice(&[op::PUSH1, 0x00]); // destination (patched below)
        let emit_patch = code.len() - 1;
        code.extend_from_slice(&[op::JUMPI, op::PUSH1, 0x00, op::PUSH1, 0x00, op::REVERT]);
        let emit_pc = code.len();
        code[emit_patch] = u8::try_from(emit_pc).unwrap();

        code.push(op::JUMPDEST);
        append_log1(&mut code, event_selector);
        code.push(op::STOP);

        let events = contract_events(&code);
        assert_eq!(events, vec![event_selector]);
    }

    #[test]
    fn test_forks_when_both_branches_are_alive() {
        let function_selector = [0xaa, 0xbb, 0xcc, 0xdd];
        let event_true = [0x11; 32];
        let event_false = [0x22; 32];

        let mut code = Vec::new();
        let entry_patch = append_single_selector_dispatch(&mut code, function_selector);

        let function_entry = code.len();
        code[entry_patch] = u8::try_from(function_entry).unwrap();
        code.push(op::JUMPDEST);

        // Always-false condition. VM takes fallthrough branch, but both branches emit,
        // so branch classifier should fork and collect both events.
        code.extend_from_slice(&[op::PUSH1, 0x00]); // cond = 0
        code.extend_from_slice(&[op::PUSH1, 0x00]); // true destination (patched below)
        let true_patch = code.len() - 1;
        code.push(op::JUMPI);

        code.push(op::JUMPDEST);
        append_log1(&mut code, event_false);
        code.push(op::STOP);

        let true_pc = code.len();
        code[true_patch] = u8::try_from(true_pc).unwrap();

        code.push(op::JUMPDEST);
        append_log1(&mut code, event_true);
        code.push(op::STOP);

        let events = contract_events(&code);
        let found: BTreeSet<_> = events.into_iter().collect();
        let expected: BTreeSet<_> = [event_true, event_false].into_iter().collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn test_no_events() {
        let code = alloy_primitives::hex::decode("6080604052348015600e575f80fd5b50").unwrap();
        let events = contract_events(&code);
        assert!(events.is_empty());
    }

    #[test]
    fn test_push32_no_log() {
        let mut code = Vec::new();
        code.push(op::PUSH32);
        code.extend_from_slice(&[0xab; 32]);
        code.push(op::POP);
        code.push(op::STOP);

        let events = contract_events(&code);
        assert!(events.is_empty());
    }

    // --- CC module tests (migrated from cc/mod.rs) ---

    /// Sub-class a: single PUSH32 + LOG1 in one block.
    #[test]
    fn cc_push32_extracts_event() {
        let selector = make_plausible_hash();
        let mut code = Vec::new();
        append_log1(&mut code, selector);
        code.push(op::STOP);

        let events = contract_events(&code);
        assert_eq!(events, vec![selector]);
    }

    /// Sub-class e/f: topic0 pushed in a predecessor block, consumed via JUMP.
    #[test]
    fn cc_cross_block_extracts_event() {
        let selector = [0x11u8; 32];
        let mut code = Vec::new();
        // Block 0: push selector, jump to JUMPDEST
        code.push(op::PUSH32);
        code.extend_from_slice(&selector);
        // PUSH1 <jumpdest_target>
        code.extend_from_slice(&[
            op::PUSH1,
            0x24, // target = 0x24
            op::JUMP,
        ]);
        // Block 1 at 0x24: JUMPDEST, then emit LOG1
        code.push(op::JUMPDEST);
        code.extend_from_slice(&[op::PUSH1, 0x00, op::PUSH1, 0x00, op::LOG1]);
        code.push(op::STOP);

        let events = contract_events(&code);
        assert_eq!(events, vec![selector]);
    }

    /// Sub-class d: Other(pc) pointing at a non-PUSH instruction → skip.
    #[test]
    fn cc_other_small_push1_returns_empty() {
        let mut code = Vec::new();
        // PUSH1 0x01 is a small value, not a plausible event hash
        code.extend_from_slice(&[
            op::PUSH1,
            0x01, // topic0 — small, not PUSH32/PUSHn(5+)
            op::PUSH1,
            0x00,
            op::PUSH1,
            0x00,
            op::LOG1,
            op::STOP,
        ]);

        let events = contract_events(&code);
        assert!(events.is_empty());
    }

    /// Sub-class b: PUSH31 with a plausible hash → extract.
    #[test]
    fn cc_push31_extracts_event() {
        // Build a 32-byte value with leading zero (since PUSH31 only pushes 31 bytes).
        // The first byte will be 0x00 after right-aligning.
        // For is_plausible_event_hash: first 6 bytes must not all be zero.
        // PUSH31 → [0x00, b1..b31] where b1..b6 are non-zero.
        let mut expected = [0u8; 32];
        for i in 1..32 {
            expected[i] = 0xab;
        }
        // expected[0] = 0x00, expected[1..] = 0xab
        // is_plausible_event_hash checks val[..6] != [0;6] — first 6 bytes are [0,ab,ab,ab,ab,ab] → OK

        let mut code = Vec::new();
        code.push(op::PUSH31);
        code.extend_from_slice(&expected[1..]); // 31 bytes
        code.extend_from_slice(&[op::PUSH1, 0x00, op::PUSH1, 0x00, op::LOG1]);
        code.push(op::STOP);

        let events = contract_events(&code);
        assert_eq!(events, vec![expected]);
    }

    /// Sub-class c: CODECOPY + MLOAD pattern.
    #[test]
    fn cc_mload_codecopy_extracts_event() {
        // Layout:
        //   [header: PUSH + PUSH + PUSH + CODECOPY + PUSH + MLOAD + PUSH + PUSH + LOG1 + STOP]
        //   Then at some offset, place the 32-byte event hash in the bytecode.
        //
        // CODECOPY copies code[src..src+32] into memory[dst..dst+32],
        // then MLOAD reads memory[dst] to get the topic.
        let selector = make_plausible_hash();

        let mut code = Vec::new();
        // We'll put the selector at code offset = 0x40 (after the instruction sequence).
        let selector_offset: u8 = 0x40;

        // PUSH1 0x20 (length = 32)
        code.extend_from_slice(&[op::PUSH1, 0x20]);
        // PUSH1 <selector_offset> (source offset in code)
        code.extend_from_slice(&[op::PUSH1, selector_offset]);
        // PUSH1 0x00 (dest offset in memory)
        code.extend_from_slice(&[op::PUSH1, 0x00]);
        // CODECOPY
        code.push(op::CODECOPY);
        // PUSH1 0x00 (memory offset to load)
        code.extend_from_slice(&[op::PUSH1, 0x00]);
        // MLOAD — loads 32 bytes from memory[0]
        code.push(op::MLOAD);
        // Now topic0 is on stack. Push offset+size for LOG1.
        code.extend_from_slice(&[op::PUSH1, 0x00, op::PUSH1, 0x00]);
        code.push(op::LOG1);
        code.push(op::STOP);

        // Pad to selector_offset
        while code.len() < selector_offset as usize {
            code.push(0x00);
        }
        // Place the selector at the expected offset
        code.extend_from_slice(&selector);

        let events = contract_events(&code);
        assert_eq!(events, vec![selector]);
    }
}
