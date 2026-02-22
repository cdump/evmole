use std::collections::BTreeMap;

use crate::collections::HashSet;

use crate::evm::{
    code_iterator::{CodeOp, iterate_code},
    op,
};

use super::{Block, BlockType};

fn is_static_jump(code: &[u8], prev_pc: usize) -> Option<usize> {
    match code[prev_pc] {
        prev_op @ op::PUSH1..=op::PUSH4 => {
            let n = (prev_op - op::PUSH0) as usize;
            let mut args = [0u8; 4];
            args[4 - n..].copy_from_slice(&code[prev_pc + 1..prev_pc + 1 + n]);
            Some(u32::from_be_bytes(args) as usize)
        }
        _ => None,
    }
}

fn new_block(start: usize) -> Block {
    Block {
        id: start,
        start,
        end: 0,
        btype: BlockType::Terminate { success: false }, // always overwritten
    }
}

pub fn initial_blocks(code: &[u8]) -> BTreeMap<usize, Block> {
    let mut blocks = BTreeMap::new();
    let mut prev_pc = 0;
    let mut block = new_block(0);

    let mut wait_jumpdest = false;

    for (pc, CodeOp { op, opi, .. }) in iterate_code(code, 0, None) {
        if wait_jumpdest {
            if op == op::JUMPDEST {
                block.id = pc;
                block.start = pc;
                wait_jumpdest = false;
            }
            prev_pc = pc;
            continue;
        }

        match op {
            op::JUMPDEST => {
                if block.start != pc {
                    // jdest could be after jumpi - already new block
                    block.end = prev_pc;
                    block.btype = BlockType::Jump { to: pc };
                    blocks.insert(block.id, block);
                    block = new_block(pc);
                }
            }

            op::JUMPI => {
                block.btype = if let Some(true_to) = is_static_jump(code, prev_pc) {
                    BlockType::Jumpi {
                        true_to,
                        false_to: pc + 1,
                    }
                } else {
                    BlockType::DynamicJumpi {
                        true_to: Vec::new(),
                        false_to: pc + 1,
                    }
                };
                block.end = pc;
                blocks.insert(block.id, block);
                block = new_block(pc + opi.size);
            }

            op::JUMP => {
                block.btype = if let Some(to) = is_static_jump(code, prev_pc) {
                    BlockType::Jump { to }
                } else {
                    BlockType::DynamicJump { to: Vec::new() }
                };
                block.end = pc;
                blocks.insert(block.id, block);
                block = new_block(pc + opi.size);
                wait_jumpdest = true;
            }

            op::REVERT | op::RETURN | op::STOP | op::SELFDESTRUCT | op::INVALID => {
                block.btype = BlockType::Terminate {
                    success: op != op::REVERT && op != op::INVALID,
                };

                block.end = pc;
                blocks.insert(block.id, block);
                block = new_block(pc + opi.size);
                wait_jumpdest = true;
            }

            _ => {
                if !opi.known {
                    wait_jumpdest = true;

                    // TODO: think about this
                    if block.start == 0 {
                        block.end = prev_pc;
                        block.btype = BlockType::Terminate { success: false };
                        blocks.insert(block.id, block);
                        block = new_block(pc /* start will be overwritten in wait_jumpdest*/);
                        break;
                    }

                    if block.start != pc {
                        // have valid instructions in block
                        block.end = prev_pc;
                        block.btype = BlockType::Terminate { success: false };
                        blocks.insert(block.id, block);
                        block = new_block(pc /* start will be overwritten in wait_jumpdest*/);
                    }
                }
            }
        }

        prev_pc = pc;
    }

    if !wait_jumpdest && block.start <= prev_pc {
        // jdest could be after jumpi - already new block
        block.end = prev_pc;
        block.btype = BlockType::Terminate { success: false };
        blocks.insert(block.id, block);
    }

    let keys: HashSet<_> = blocks.keys().copied().collect();

    for bl in blocks.values_mut() {
        assert!(
            bl.end >= bl.start,
            "{:?} | st={:?}",
            bl,
            op::info(code[bl.start])
        );
        let new_btype = match &bl.btype {
            BlockType::Jump { to } => {
                if !keys.contains(to) || code[*to] != op::JUMPDEST {
                    Some(BlockType::Terminate { success: false })
                } else {
                    None
                }
            }
            BlockType::Jumpi { true_to, false_to } => {
                let true_valid = keys.contains(true_to) && code[*true_to] == op::JUMPDEST;
                let false_valid = keys.contains(false_to);
                match (true_valid, false_valid) {
                    (true, true) => None,
                    (false, true) => Some(BlockType::Jump { to: *false_to }),
                    (_, false) => Some(BlockType::Terminate { success: false }),
                }
            }
            _ => None,
        };
        if let Some(btype) = new_btype {
            bl.btype = btype;
        }
    }
    blocks
}
