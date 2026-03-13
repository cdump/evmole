use std::collections::BTreeMap;

use crate::collections::{HashMap, HashSet, IndexMap};

use super::{
    Block, BlockType, DynamicJump,
    reachable::{extend_reachable_nodes, get_reachable_nodes},
    state::{StackSym, State},
};

#[derive(Default)]
struct RevIdx {
    /// Maps a block start address to its `State`.
    states: HashMap<usize /*start*/, State>,

    /// Depth-aware block states used when a cached summary does not preserve enough stack.
    /// Re-executing the block with a deeper identity stack keeps older untouched slots as
    /// `Before(n)` instead of collapsing them away.
    deep_states: HashMap<(usize /*start*/, usize /*depth*/), State>,

    /// Maps a destination to all parent paths (each as a vector of block addresses) and their associated state.
    parents: HashMap<usize /*to*/, IndexMap<Vec<usize> /*path*/, State>>,

    /// Replayed parent-path states keyed by the flattened path and the requested stack depth.
    /// A path can be discovered first through a shallow dynamic jump and later reused by a
    /// nested return trampoline that needs a deeper caller slot.
    replayed_paths: HashMap<(Vec<usize>, usize /*depth*/), State>,

    /// Intermediate states: maps a block (by its last element) to states and the set of jump symbols encountered so far.
    istate: HashMap<usize, HashMap<State, HashSet<StackSym>>>,

    /// Keeps track of “bad” paths that exceeded limits or ended in unexpected symbols.
    badpaths: HashSet<Vec<usize>>,

    /// A set of block addresses known to be reachable from entrypoint (pc=0)
    reachable0: HashSet<usize>,
}

impl RevIdx {
    fn set_reachable0(&mut self, r: HashSet<usize>) {
        self.reachable0 = r;
    }

    fn insert_state(&mut self, start: usize, state: State) {
        self.states.insert(start, state);
    }

    fn get_state(&mut self, code: &[u8], start: usize) -> &State {
        self.states.entry(start).or_insert_with(|| {
            let mut st = State::new();
            st.exec(code, start, None);
            st
        })
    }

    // Re-execute a single block with an identity prefix large enough to keep the requested
    // untouched caller slots available as `Before(n)`.
    fn get_state_with_depth(&mut self, code: &[u8], start: usize, depth: usize) -> State {
        if depth == 0 {
            return self.get_state(code, start).clone();
        }

        self.deep_states
            .entry((start, depth))
            .or_insert_with(|| {
                let mut st = State::with_identity(depth);
                st.exec(code, start, None);
                st
            })
            .clone()
    }

    fn insert_direct_parent(&mut self, to: usize, from: usize, state: State) {
        self.parents
            .entry(to)
            .or_default()
            .insert(vec![from], state);
    }

    /// Returns true if the path was new
    fn insert_path_parent(&mut self, to: usize, path: &[usize], state: State) -> bool {
        let ls = path[path.len() - 1];
        assert!(
            self.reachable0.contains(&ls),
            "last element not reachable: r0={:?} ls={} to={} path={:?}",
            self.reachable0,
            ls,
            to,
            path
        );

        self.reachable0.insert(to);

        let entry = self.parents.entry(to).or_default();
        if entry.contains_key(path) {
            return false;
        }
        entry.insert(path.to_vec(), state);
        true
    }

    fn insert_badpath(&mut self, path: &[usize]) -> bool {
        if self.badpaths.contains(path) {
            return false;
        }
        self.badpaths.insert(path.to_vec());
        true
    }

    /// Returns the parent paths for a given destination that end in reachable nodes
    fn get_parents(&self, to: usize) -> Vec<(Vec<usize>, State)> {
        if let Some(m) = self.parents.get(&to) {
            m.iter()
                .filter_map(|(path, state)| {
                    if self.reachable0.contains(&path[path.len() - 1]) {
                        Some((path.clone(), state.clone()))
                    } else {
                        // eprintln!("no for {} {:?}", to, p);
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Adds an intermediate state for a given block (identified by its last element in a path),
    /// associated with a given state and jump symbol.
    /// Returns `true` if this combination is new.
    fn add_inter_state(&mut self, last: usize, state: &State, jmp: &StackSym) -> bool {
        let entry = self.istate.entry(last).or_default();
        if let Some(st) = entry.get_mut(state) {
            if st.contains(jmp) {
                return false;
            }
            st.insert(jmp.to_owned());
        } else {
            entry.insert(state.to_owned(), HashSet::from_iter([jmp.to_owned()]));
        }
        true
    }

    fn clear_inter_state(&mut self) {
        self.istate.clear();
    }

    fn materialize_path_state(
        &mut self,
        code: &[u8],
        path: &[usize],
        summary: &State,
        required_depth: usize,
    ) -> State {
        if required_depth < summary.explicit_len() {
            return summary.clone();
        }

        if path.len() == 1 {
            return self.get_state_with_depth(code, path[0], required_depth);
        }

        // Stored parent-path states are intentionally minimal for the jump that first discovered
        // them. If a later consumer asks for a deeper stack slot, replay the whole path with a
        // widened identity prefix and memoize that richer version.
        let key = (path.to_vec(), required_depth);
        if let Some(state) = self.replayed_paths.get(&key) {
            return state.clone();
        }

        let mut iter = path.iter().copied();
        let first = iter.next().expect("parent path must not be empty");
        let mut state = self.get_state_with_depth(code, first, required_depth);

        for start in iter {
            let depth = required_depth.max(state.max_before().unwrap_or_default());
            let parent = self.get_state_with_depth(code, start, depth);
            state = state.resolve_with_parent(&parent);
        }

        self.replayed_paths.insert(key, state.clone());
        state
    }
}

/// Recursively explores dynamic jump paths starting from a given path
/// Returns vector for dynamic jumps and energy used
fn resolve_dynamic_jump_path(
    code: &[u8],
    rev_idx: &mut RevIdx,
    path: Vec<usize>,
    path_weight: usize,
    stack_pos: usize,
    state: State,
    energy_limit: usize,
) -> (Vec<DynamicJump>, usize) {
    const MAX_PATH_WEIGHT: usize = 256;
    assert!(path_weight <= MAX_PATH_WEIGHT);

    let current = *path.last().unwrap();
    let mut energy_used = 0;
    let mut dynamic_jumps: Vec<DynamicJump> = Vec::new();

    let parents = rev_idx.get_parents(current);

    // crate::utils::log(format!("parents for {} : {:?}", current, parents));

    for (parent_path, parent_state) in parents.into_iter() {
        energy_used += 1;
        if energy_used > energy_limit {
            break;
        }

        // Cycle detection for direct (single-block) parent paths only.
        // A direct parent that is already in our current backwards path represents a
        // CFG back-edge (loop). Each loop iteration inflates the tracked stack position
        // via state composition, causing the algorithm to look at the wrong slot and
        // produce None. The base-case path (zero loop iterations) already finds the
        // correct destination, so skip these back-edges.
        // Multi-element parent paths (pre-discovered paths from resolved DynamicJumps)
        // are excluded from this check to avoid blocking legitimate resolution chains.
        if parent_path.len() == 1 && path.contains(&parent_path[0]) {
            continue;
        }

        let mut current_state = state.clone();
        let mut required_depth = current_state
            .max_before()
            .map_or(stack_pos, |depth| depth.max(stack_pos));

        loop {
            // Materializing with a deeper prefix can reveal older `Before(n)` references that
            // were hidden by the shallower summary, so widen until the dependency frontier stops
            // growing.
            let materialized =
                rev_idx.materialize_path_state(code, &path, &current_state, required_depth);
            let next_required = materialized
                .max_before()
                .map_or(stack_pos, |depth| depth.max(stack_pos));
            current_state = materialized;

            if next_required <= required_depth {
                required_depth = next_required;
                break;
            }

            required_depth = next_required;
        }

        let parent_state =
            rev_idx.materialize_path_state(code, &parent_path, &parent_state, required_depth);
        let jump_sym = parent_state.get_stack(stack_pos);
        let new_state = current_state.resolve_with_parent(&parent_state);

        let mut new_path = Vec::with_capacity(path.len() + parent_path.len());
        new_path.extend(&path);
        new_path.extend(parent_path);

        // Count each reused parent path as one search step. A cached multi-block parent can have
        // a very long flattened witness path, but replay already treats it as one summarized edge
        // with richer state recovered on demand. Using the flattened length here would reject
        // valid nested-return chains before symbolic resolution has a chance to continue.
        let new_path_weight = path_weight + 1;
        if new_path_weight > MAX_PATH_WEIGHT {
            if rev_idx.insert_badpath(&new_path) {
                dynamic_jumps.push(DynamicJump {
                    path: new_path,
                    to: None,
                });
            }
            continue;
        }

        // Only proceed if this (state, jump) combination is new.
        if !rev_idx.add_inter_state(*new_path.last().unwrap(), &new_state, &jump_sym) {
            // TODO: add this path?
            continue;
        }

        match jump_sym {
            StackSym::Before(new_stack_pos) => {
                // eprintln!("before {} from {:?}", b, newpath);
                let (jumps, used) = resolve_dynamic_jump_path(
                    code,
                    rev_idx,
                    new_path,
                    new_path_weight,
                    new_stack_pos,
                    new_state,
                    energy_limit - energy_used,
                );
                energy_used += used;
                dynamic_jumps.extend(jumps);
            }

            StackSym::Jumpdest(to) => {
                // crate::utils::log(format!("found {} from {:?}", to, new_path));
                if rev_idx.insert_path_parent(to, &new_path, new_state) {
                    dynamic_jumps.push(DynamicJump {
                        path: new_path,
                        to: Some(to),
                    });
                }
            }
            StackSym::Pushed(_) | StackSym::Other(_) => {
                // push, but not jumpdest or other opcode
                if rev_idx.insert_badpath(&new_path) {
                    dynamic_jumps.push(DynamicJump {
                        path: new_path,
                        to: None,
                    });
                }
            }
        }
    }
    (dynamic_jumps, energy_used)
}

/// Resolves dynamic jumps for the given code and basic blocks by recursively exploring
/// possible execution paths.
///
/// This function first “executes” each block to update its state and, when possible,
/// converts dynamic jumps (or conditional dynamic jumps) into static jumps. For those
/// still dynamic, it uses a recursive exploration (bounded by an energy limit) to
/// determine possible jump targets.
///
/// Finally, if all paths from a dynamic jump lead to the same target, the block’s
/// type is changed to a static jump.
///
/// # Parameters
/// - `code`: The code bytes to execute.
/// - `blocks`: A mapping from block start addresses to `Block`s.
///
/// # Returns
/// An updated `BTreeMap` with resolved jump targets.
pub fn resolve_dynamic_jumps(
    code: &[u8],
    mut blocks: BTreeMap<usize, Block>,
) -> BTreeMap<usize, Block> {
    // Map block start addresses to the initial stack position extracted from the block.
    let mut stack_pos: Vec<(usize, usize)> = Vec::default();

    let mut rev_idx = RevIdx::default();
    rev_idx.set_reachable0(HashSet::from_iter([0]));

    // First stage resolve
    for block in blocks.values_mut() {
        if !matches!(
            block.btype,
            BlockType::DynamicJump { .. } | BlockType::DynamicJumpi { .. }
        ) {
            continue;
        }
        let mut state = State::new();
        match state.exec(code, block.start, None) {
            Some(StackSym::Jumpdest(to)) => match block.btype {
                BlockType::DynamicJump { .. } => block.btype = BlockType::Jump { to },
                BlockType::DynamicJumpi { false_to, .. } => {
                    block.btype = BlockType::Jumpi {
                        true_to: to,
                        false_to,
                    }
                }
                _ => unreachable!("unexpected block type"),
            },
            Some(StackSym::Before(new_stack_pos)) => {
                stack_pos.push((block.start, new_stack_pos));
            }
            _ => {}
        }
        rev_idx.insert_state(block.start, state);
    }

    // Build direct parent relationships from known static jump targets.
    for block in blocks.values() {
        let state = rev_idx.get_state(code, block.start);
        match block.btype {
            BlockType::Jump { to } => {
                let state = state.to_owned();
                rev_idx.insert_direct_parent(to, block.start, state);
            }
            BlockType::Jumpi { true_to, false_to } => {
                let state = state.to_owned();
                rev_idx.insert_direct_parent(true_to, block.start, state.clone());
                rev_idx.insert_direct_parent(false_to, block.start, state);
            }
            BlockType::Terminate { .. } => {}
            BlockType::DynamicJump { .. } => {} // empty at this point
            BlockType::DynamicJumpi { false_to, .. } => {
                let state = state.to_owned();
                rev_idx.insert_direct_parent(false_to, block.start, state);
            }
        }
    }

    let mut total_energy_used = 0;
    const ENERGY_LIMIT: usize = 500_000;
    let mut reachable = get_reachable_nodes(&blocks, 0, None);
    let mut witness_targets: HashMap<usize, Vec<usize>> = HashMap::default();

    for _itpos in 0..128 {
        if total_energy_used >= ENERGY_LIMIT {
            break;
        }

        rev_idx.set_reachable0(reachable.clone());

        let mut found_new_paths = false;
        let mut new_targets = Vec::new();

        for (start, stack_pos) in &stack_pos {
            if !reachable.contains(start) {
                continue;
            }
            if total_energy_used >= ENERGY_LIMIT {
                break;
            }

            let state = rev_idx.get_state(code, *start).to_owned();
            let (jumps, energy_used) = resolve_dynamic_jump_path(
                code,
                &mut rev_idx,
                vec![*start],
                1,
                *stack_pos,
                state,
                ENERGY_LIMIT - total_energy_used,
            );
            total_energy_used += energy_used;

            if !jumps.is_empty() {
                found_new_paths = true;
                for jump in &jumps {
                    if let Some(to) = jump.to {
                        let witness = *jump.path.last().unwrap();
                        witness_targets.entry(witness).or_default().push(to);
                        new_targets.push(to);
                    }
                }
                match blocks.get_mut(start).unwrap().btype {
                    BlockType::DynamicJump { ref mut to } => {
                        to.extend(jumps);
                    }
                    BlockType::DynamicJumpi {
                        ref mut true_to, ..
                    } => {
                        true_to.extend(jumps);
                    }
                    _ => unreachable!("unexpected block type"),
                }
            }
            rev_idx.clear_inter_state();
        }
        if !found_new_paths {
            break;
        }
        extend_reachable_nodes(&blocks, &mut reachable, &witness_targets, new_targets);
    }

    // `None` paths are usually impossible branch combinations from the backwards search,
    // such as traversing contradictory `JUMPI` outcomes that cannot both happen in one
    // top-to-bottom execution. Clear those paths now, because we can't do anything better anyway.
    for (start, _) in &stack_pos {
        if let BlockType::DynamicJump { to: ref mut dj } = blocks.get_mut(start).unwrap().btype {
            dj.retain(|b| b.to.is_some());
        }
    }

    // Merge jump targets if all dynamic jumps from a block lead to the same target.
    // After the `None`-path cleanup above, every remaining entry has `to: Some(...)`.
    for (start, _) in stack_pos {
        let mut one_to = None;

        if let BlockType::DynamicJump { to: ref dj } = blocks.get(&start).unwrap().btype {
            let mut targets = dj.iter().map(|v| v.to.unwrap());
            if let Some(first) = targets.next()
                && targets.all(|t| t == first)
            {
                one_to = Some(first);
            }
        }
        if let Some(to) = one_to {
            blocks.get_mut(&start).unwrap().btype = BlockType::Jump { to };
        }
    }

    blocks
}
