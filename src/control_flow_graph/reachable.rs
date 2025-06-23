use std::collections::BTreeMap;

use crate::collections::HashSet;

use super::{Block, BlockType, INVALID_JUMP_START};

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
            if current >= INVALID_JUMP_START {
                continue;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_basic_block(btype: BlockType) -> Block {
        Block {
            start: 0,
            end: 1,
            btype,
        }
    }

    #[test]
    fn test_simple_jump() {
        let mut blocks = BTreeMap::new();
        blocks.insert(0, create_basic_block(BlockType::Jump { to: 1 }));
        blocks.insert(1, create_basic_block(BlockType::Terminate { success: false }));

        let reachable = get_reachable_nodes(&blocks, 0, None);
        assert_eq!(reachable, HashSet::from_iter([0, 1]));
    }

    #[test]
    fn test_conditional_jump() {
        let mut blocks = BTreeMap::new();
        blocks.insert(0, create_basic_block(BlockType::Jumpi { true_to: 1, false_to: 2 }));
        blocks.insert(1, create_basic_block(BlockType::Terminate { success: false }));
        blocks.insert(2, create_basic_block(BlockType::Terminate { success: false }));

        let reachable = get_reachable_nodes(&blocks, 0, None);
        assert_eq!(reachable, HashSet::from_iter([0, 1, 2]));
    }

    #[test]
    fn test_dynamic_jump() {
        use super::super::DynamicJump;

        let mut blocks = BTreeMap::new();
        blocks.insert(0, create_basic_block(BlockType::DynamicJump {
            to: vec![
                DynamicJump { to: Some(1), path: vec![0] },
                DynamicJump { to: Some(2), path: vec![0] },
            ]
        }));
        blocks.insert(1, create_basic_block(BlockType::Terminate { success: false }));
        blocks.insert(2, create_basic_block(BlockType::Terminate { success: false }));

        let reachable = get_reachable_nodes(&blocks, 0, None);
        assert_eq!(reachable, HashSet::from_iter([0, 1, 2]));
    }
}
