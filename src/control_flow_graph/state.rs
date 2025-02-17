use crate::evm::{
    code_iterator::{iterate_code, CodeOp},
    op,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// A symbolic value on the stack.
pub enum StackSym {
    Before(usize),
    Pushed([u8; 4]), // only PUSH[1..4] handled
    Jumpdest(usize /*to*/),
    Other(usize /*pc*/),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    stack: Vec<StackSym>,
}

impl State {
    pub fn new() -> Self {
        let mut stack = Vec::with_capacity(16);
        stack.push(StackSym::Before(0));
        Self { stack }
    }

    /// Returns the symbol at the given stack position (0 is the top of the stack)
    /// If `pos` is beyond the current stack, a new `Before` symbol is synthesized
    pub fn get_stack(&self, pos: usize) -> StackSym {
        let slen = self.stack.len();
        if pos < slen {
            self.stack[slen - pos - 1].clone()
        } else if let StackSym::Before(b) = self.stack[0] {
            StackSym::Before(b + 1 + (pos - slen))
        } else {
            panic!("first stack element is not Before: {:?}", self.stack);
        }
    }

    /// Resolves this state using a parent state.
    ///
    /// This function "updates" the current state's 'Before' symbols using the parent's stack
    pub fn resolve_with_parent(&self, parent: &State) -> State {
        let base_before = if let StackSym::Before(v) = self.stack[0] {
            v
        } else {
            panic!("first stack element is not Before: {:?}", self.stack);
        };

        let parent_len = parent.stack.len();
        let extra = if parent_len > base_before {
            parent_len - base_before - 1
        } else {
            0
        };
        let mut new_stack = Vec::with_capacity(self.stack.len() + extra);

        if parent_len > base_before {
            new_stack.extend_from_slice(&parent.stack[..(parent_len - base_before - 1)]);
        }

        // Replace every Before(i) symbol in self with the parent's corresponding stack symbol
        new_stack.extend(self.stack.iter().map(|el| match el {
            StackSym::Before(idx) => parent.get_stack(*idx),
            other => other.clone(),
        }));

        State { stack: new_stack }
    }

    /// Executes the given code starting at 'start' and updates the state
    ///
    /// After execution, the state's stack is "minimalized" by trimming redundant 'Before' symbols
    pub fn exec(&mut self, code: &[u8], start: usize) -> Option<StackSym> {
        assert!(matches!(self.stack.first(), Some(StackSym::Before(_))));
        let r = self.real_exec(code, start);
        assert!(matches!(self.stack.first(), Some(StackSym::Before(_))));

        // Before(4),Before(3),Before(2),Before(0) => Before(2),Before(0)
        let base_before = if let StackSym::Before(v) = self.stack[0] {
            v
        } else {
            panic!("first stack element is not Before: {:?}", self.stack);
        };

        let prefix_len = self
            .stack
            .iter()
            .enumerate()
            .take_while(|(pos, el)| {
                matches!(el, StackSym::Before(nb) if *nb + pos == base_before && *nb > 0)
            })
            .count();

        if prefix_len > 1 {
            self.stack.drain(..prefix_len - 1);
        }
        r
    }

    fn real_exec(&mut self, code: &[u8], start_pc: usize) -> Option<StackSym> {
        for (pc, CodeOp { op, opi, .. }) in iterate_code(code, start_pc) {
            // Ensure the stack has at least (stack_in + 1) entries (the extra one preserves the initial Before)
            if self.stack.len() < opi.stack_in + 1 {
                let needed = opi.stack_in + 1 - self.stack.len();
                if let StackSym::Before(base_before) = self.stack[0] {
                    self.stack.splice(
                        0..0,
                        ((base_before + 1)..(base_before + 1 + needed))
                            .rev()
                            .map(StackSym::Before),
                    );
                } else {
                    panic!(
                        "Expected first stack element to be Before: {:?}",
                        self.stack
                    )
                };
            }

            match op {
                op::PUSH1..=op::PUSH4 => {
                    let n = (op - op::PUSH0) as usize;
                    let mut args = [0u8; 4];
                    args[4 - n..].copy_from_slice(&code[pc + 1..pc + 1 + n]);
                    let val = u32::from_be_bytes(args) as usize;
                    self.stack
                        .push(if val < code.len() && code[val] == op::JUMPDEST {
                            StackSym::Jumpdest(val)
                        } else {
                            StackSym::Pushed(args)
                        });
                }
                op::DUP1..=op::DUP16 => {
                    let n = (op - op::DUP1 + 1) as usize;
                    let stack_len = self.stack.len();
                    self.stack.push(self.stack[stack_len - n].clone());
                }
                op::SWAP1..=op::SWAP16 => {
                    let n = (op - op::SWAP1 + 1) as usize;
                    let stack_len = self.stack.len();
                    self.stack.swap(stack_len - 1, stack_len - 1 - n);
                }
                op::AND => {
                    let s1 = self.stack.pop().expect("Stack underflow in AND");
                    let s2 = self.stack.pop().expect("Stack underflow in AND");
                    if s1 == StackSym::Pushed([0xff; 4]) {
                        self.stack.push(s2);
                    } else if s2 == StackSym::Pushed([0xff; 4]) {
                        self.stack.push(s1);
                    } else {
                        self.stack.push(StackSym::Other(pc));
                    }
                }

                op::JUMP => {
                    let to = self.stack.pop();
                    assert!(to.is_some());
                    return to; // already Some()
                }
                op::JUMPI => {
                    let to = self.stack.pop();
                    assert!(to.is_some());
                    self.stack.pop(); // condition
                    return to; // already Some()
                }

                op::JUMPDEST => {
                    // If we hit a JUMPDEST not at the start of our block, return
                    if pc != start_pc {
                        return None;
                    }
                }

                op::REVERT | op::RETURN | op::STOP | op::SELFDESTRUCT | op::INVALID => {
                    for _ in 0..opi.stack_in {
                        self.stack.pop();
                    }
                    for _ in 0..opi.stack_out {
                        self.stack.push(StackSym::Other(pc));
                    }
                    return None;
                }

                _ => {
                    for _ in 0..opi.stack_in {
                        self.stack.pop();
                    }
                    for _ in 0..opi.stack_out {
                        self.stack.push(StackSym::Other(pc));
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_drain() {
        let cases = vec![
            (
                vec![
                    StackSym::Before(10),
                    StackSym::Before(5),
                    StackSym::Other(999),
                ],
                vec![
                    StackSym::Before(10),
                    StackSym::Before(5),
                    StackSym::Other(999),
                ],
            ),
            (
                vec![
                    StackSym::Before(10),
                    StackSym::Before(9),
                    StackSym::Before(8),
                    StackSym::Other(999),
                ],
                vec![StackSym::Before(8), StackSym::Other(999)],
            ),
            (
                vec![
                    StackSym::Before(10),
                    StackSym::Before(9),
                    StackSym::Before(8),
                ],
                vec![StackSym::Before(8)],
            ),
            (
                vec![
                    StackSym::Before(3),
                    StackSym::Before(2),
                    StackSym::Before(3),
                    StackSym::Other(999),
                ],
                vec![
                    StackSym::Before(2),
                    StackSym::Before(3),
                    StackSym::Other(999),
                ],
            ),
            (
                vec![
                    StackSym::Before(3),
                    StackSym::Before(2),
                    StackSym::Before(0),
                    StackSym::Before(1),
                ],
                vec![
                    StackSym::Before(2),
                    StackSym::Before(0),
                    StackSym::Before(1),
                ],
            ),
            (vec![StackSym::Before(0)], vec![StackSym::Before(0)]),
        ];

        for (input_stack, expected_output_stack) in cases.into_iter() {
            let mut state = State::new();
            state.stack = input_stack;
            state.exec(&[], 0);

            assert_eq!(state.stack, expected_output_stack);
        }
    }

    #[test]
    fn test_resolve_with_parent() {
        let test_cases = vec![
            // (self, parent, expected)
            (
                vec![StackSym::Before(1), StackSym::Other(42)],
                vec![StackSym::Before(0)],
                vec![StackSym::Before(1), StackSym::Other(42)],
            ),
            (
                vec![StackSym::Before(2)],
                vec![StackSym::Before(1)],
                vec![StackSym::Before(3)],
            ),
            (
                vec![
                    StackSym::Before(2),
                    StackSym::Before(0),
                    StackSym::Other(99),
                ],
                vec![
                    StackSym::Before(10),
                    StackSym::Before(9),
                    StackSym::Pushed([1, 2, 3, 4]),
                ],
                vec![
                    StackSym::Before(10),
                    StackSym::Pushed([1, 2, 3, 4]),
                    StackSym::Other(99),
                ],
            ),
            (
                vec![
                    StackSym::Before(3),
                    StackSym::Before(1),
                    StackSym::Before(0),
                ],
                vec![
                    StackSym::Before(5),
                    StackSym::Other(200),
                    StackSym::Other(201),
                    StackSym::Pushed([9, 9, 9, 9]),
                    StackSym::Other(202),
                ],
                vec![
                    StackSym::Before(5),
                    StackSym::Other(200),
                    StackSym::Pushed([9, 9, 9, 9]),
                    StackSym::Other(202),
                ],
            ),
        ];

        for (self_stack, parent_stack, expected_stack) in test_cases.into_iter() {
            let self_state = State { stack: self_stack };
            let parent_state = State {
                stack: parent_stack,
            };
            let resolved_state = self_state.resolve_with_parent(&parent_state);
            assert_eq!(resolved_state.stack, expected_stack);
        }
    }
}
