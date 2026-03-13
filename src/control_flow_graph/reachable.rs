use std::collections::BTreeMap;

use crate::collections::{HashMap, HashSet};

use super::{Block, BlockType};

pub fn get_reachable_nodes(
    blocks: &BTreeMap<usize, Block>,
    from: usize,
    initial_visited: Option<HashSet<usize>>,
) -> HashSet<usize> {
    let mut global_visited = initial_visited.unwrap_or_default();
    let mut reachable = HashSet::default();
    loop {
        let before = reachable.len();
        let mut visited = HashSet::default();
        let mut queue = vec![from];
        while let Some(current) = queue.pop() {
            if !visited.insert(current) {
                continue;
            }

            let bl = if let Some(bl) = blocks.get(&current) {
                bl
            } else {
                continue;
            };

            match bl.btype {
                BlockType::Terminate { .. } => {}
                BlockType::Jump { to } => {
                    queue.push(to);
                }
                BlockType::Jumpi { true_to, false_to } => {
                    queue.push(true_to);
                    queue.push(false_to);
                }
                BlockType::DynamicJump { ref to } => {
                    queue.extend(to.iter().filter_map(|dj| {
                        dj.to.filter(|_t| {
                            let p = dj.path.last().unwrap();
                            visited.contains(p) || global_visited.contains(p)
                        })
                    }));
                }
                BlockType::DynamicJumpi {
                    true_to: _,
                    false_to,
                } => {
                    queue.push(false_to);
                }
            }
        }
        reachable.extend(&visited);
        global_visited.extend(visited);
        if before == reachable.len() {
            break;
        }
    }
    reachable
}

pub(crate) fn extend_reachable_nodes(
    blocks: &BTreeMap<usize, Block>,
    reachable: &mut HashSet<usize>,
    witness_targets: &HashMap<usize, Vec<usize>>,
    seeds: impl IntoIterator<Item = usize>,
) -> bool {
    let mut queue: Vec<usize> = seeds.into_iter().collect();
    let before = reachable.len();

    while let Some(current) = queue.pop() {
        if !reachable.insert(current) {
            continue;
        }

        let bl = if let Some(bl) = blocks.get(&current) {
            bl
        } else {
            continue;
        };

        match bl.btype {
            BlockType::Terminate { .. } => {}
            BlockType::Jump { to } => {
                queue.push(to);
            }
            BlockType::Jumpi { true_to, false_to } => {
                queue.push(true_to);
                queue.push(false_to);
            }
            BlockType::DynamicJump { .. } => {}
            BlockType::DynamicJumpi {
                true_to: _,
                false_to,
            } => {
                queue.push(false_to);
            }
        }

        if let Some(targets) = witness_targets.get(&current) {
            queue.extend(targets.iter().copied());
        }
    }

    reachable.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_basic_block(btype: BlockType) -> Block {
        Block {
            id: 0,
            start: 0,
            end: 1,
            btype,
        }
    }

    #[test]
    fn test_simple_jump() {
        let mut blocks = BTreeMap::new();
        blocks.insert(0, create_basic_block(BlockType::Jump { to: 1 }));
        blocks.insert(
            1,
            create_basic_block(BlockType::Terminate { success: false }),
        );

        let reachable = get_reachable_nodes(&blocks, 0, None);
        assert_eq!(reachable, HashSet::from_iter([0, 1]));
    }

    #[test]
    fn test_conditional_jump() {
        let mut blocks = BTreeMap::new();
        blocks.insert(
            0,
            create_basic_block(BlockType::Jumpi {
                true_to: 1,
                false_to: 2,
            }),
        );
        blocks.insert(
            1,
            create_basic_block(BlockType::Terminate { success: false }),
        );
        blocks.insert(
            2,
            create_basic_block(BlockType::Terminate { success: false }),
        );

        let reachable = get_reachable_nodes(&blocks, 0, None);
        assert_eq!(reachable, HashSet::from_iter([0, 1, 2]));
    }

    #[test]
    fn test_dynamic_jump() {
        use super::super::DynamicJump;

        let mut blocks = BTreeMap::new();
        blocks.insert(
            0,
            create_basic_block(BlockType::DynamicJump {
                to: vec![
                    DynamicJump {
                        to: Some(1),
                        path: vec![0],
                    },
                    DynamicJump {
                        to: Some(2),
                        path: vec![0],
                    },
                ],
            }),
        );
        blocks.insert(
            1,
            create_basic_block(BlockType::Terminate { success: false }),
        );
        blocks.insert(
            2,
            create_basic_block(BlockType::Terminate { success: false }),
        );

        let reachable = get_reachable_nodes(&blocks, 0, None);
        assert_eq!(reachable, HashSet::from_iter([0, 1, 2]));
    }

    #[test]
    fn test_extend_reachable_nodes_follows_static_edges() {
        let mut blocks = BTreeMap::new();
        blocks.insert(1, create_basic_block(BlockType::Jump { to: 2 }));
        blocks.insert(
            2,
            create_basic_block(BlockType::Terminate { success: false }),
        );

        let mut reachable = HashSet::from_iter([0]);
        let changed = extend_reachable_nodes(&blocks, &mut reachable, &HashMap::default(), [1]);

        assert!(changed);
        assert_eq!(reachable, HashSet::from_iter([0, 1, 2]));
    }

    #[test]
    fn test_extend_reachable_nodes_uses_witness_targets() {
        let mut blocks = BTreeMap::new();
        blocks.insert(2, create_basic_block(BlockType::Jump { to: 3 }));
        blocks.insert(3, create_basic_block(BlockType::Jump { to: 5 }));
        blocks.insert(
            4,
            create_basic_block(BlockType::Terminate { success: false }),
        );
        blocks.insert(
            5,
            create_basic_block(BlockType::Terminate { success: false }),
        );
        blocks.insert(
            6,
            create_basic_block(BlockType::Terminate { success: false }),
        );

        let witness_targets = HashMap::from_iter([(3, vec![4]), (5, vec![6])]);
        let mut reachable = HashSet::from_iter([0, 1]);
        let changed = extend_reachable_nodes(&blocks, &mut reachable, &witness_targets, [2]);

        assert!(changed);
        assert_eq!(reachable, HashSet::from_iter([0, 1, 2, 3, 4, 5, 6]));
    }
}
