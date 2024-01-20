use std::{collections::HashSet, error, fmt};

use super::{memory::Memory, op, stack::Stack, Element, U256};
use super::{VAL_0_B, VAL_1, VAL_1_B, VAL_256, VAL_32, VAL_32_B};

#[derive(Debug)]
pub struct UnsupportedOpError {
    pub op: op::OpCode,
}
impl std::fmt::Display for UnsupportedOpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UnsupportedOpError({})", op::name(self.op))
    }
}

impl std::error::Error for UnsupportedOpError {}

pub struct StepResult<T> {
    pub op: op::OpCode,
    pub gas_used: u32,
    pub fa: Option<Element<T>>,
    pub sa: Option<Element<T>>,
    pub ul: Option<HashSet<T>>,
}

impl<T> StepResult<T> {
    fn new(op: op::OpCode, gas_used: u32) -> StepResult<T> {
        StepResult {
            op,
            gas_used,
            fa: None,
            sa: None,
            ul: None,
        }
    }
}

#[derive(Clone)]
pub struct Vm<'a, T>
where
    T: Clone + std::fmt::Debug,
{
    code: &'a [u8],
    pc: usize,
    pub stack: Stack<T>,
    memory: Memory<T>,
    pub stopped: bool,
    pub calldata: Element<T>, // don't have calldata.len > 32
}

impl<'a, T> fmt::Debug for Vm<'a, T>
where
    T: Clone + std::fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Vm:\n .pc = {:x} | {}\n .stack = {:?}\n .memory = {:?}",
            self.pc,
            if !self.stopped { op::name(self.code[self.pc]) } else { "" },
            self.stack,
            self.memory
        )
    }
}

impl<'a, T> Vm<'a, T>
where
    T: std::fmt::Debug + Clone + Eq + std::hash::Hash,
{
    pub fn new(code: &'a [u8], calldata: Element<T>) -> Vm<T> {
        Vm {
            code,
            pc: 0,
            stack: Stack::new(),
            memory: Memory::new(),
            stopped: code.is_empty(),
            calldata,
        }
    }

    pub fn step(&mut self) -> Result<StepResult<T>, Box<dyn error::Error>> {
        let op = self.code[self.pc];
        let ret = self.exec_opcode(op)?;

        if op != op::JUMP && op != op::JUMPI {
            self.pc += 1
        }

        if self.pc >= self.code.len() {
            self.stopped = true;
        }

        Ok(ret)
    }

    fn exec_opcode(&mut self, op: op::OpCode) -> Result<StepResult<T>, Box<dyn error::Error>> {
        match op {
            op::PUSH0..=op::PUSH32 => {
                let n = (op - op::PUSH0) as usize;
                if self.pc + 1 + n > self.code.len() {
                    return Err(UnsupportedOpError { op }.into())
                }
                let mut args: [u8; 32] = [0; 32];
                args[(32 - n)..].copy_from_slice(&self.code[self.pc + 1..self.pc + 1 + n]);
                self.stack.push(Element {
                    data: args,
                    label: None,
                });
                self.pc += n;
                Ok(StepResult::new(op, if n == 0 { 2 } else { 3 }))
            }

            op::JUMP | op::JUMPI => {
                let s0 = self.stack.pop_uint()?;
                if op == op::JUMPI {
                    let s1 = self.stack.pop_uint()?;
                    if s1.is_zero() {
                        self.pc += 1;
                        return Ok(StepResult::new(op, 10));
                    }
                }
                let cres: Result<usize, _> = s0.try_into();
                if let Ok(newpc) = cres {
                    if newpc >= self.code.len() || self.code[newpc] != op::JUMPDEST {
                        Err(UnsupportedOpError { op }.into())
                    } else {
                        self.pc = newpc;
                        Ok(StepResult::new(op, if op == op::JUMP { 8 } else { 10 }))
                    }
                } else {
                    Err(UnsupportedOpError { op }.into())
                }
            }

            op::DUP1..=op::DUP16 => {
                self.stack.dup(op - op::DUP1 + 1)?;
                Ok(StepResult::new(op, 3))
            }

            op::JUMPDEST => Ok(StepResult::new(op, 1)),

            op::REVERT => {
                // skip 2 stack pop()s
                self.stopped = true;
                Ok(StepResult::new(op, 4))
            }

            op::EQ
            | op::LT
            | op::GT
            | op::SUB
            | op::ADD
            | op::DIV
            | op::MUL
            | op::EXP
            | op::XOR
            | op::AND
            | op::OR
            | op::SHR
            | op::SHL
            | op::BYTE
            | op::SLT
            | op::SGT => {
                let raws0 = self.stack.pop()?;
                let raws1 = self.stack.pop()?;

                let s0: U256 = (&raws0).into();
                let s1: U256 = (&raws1).into();

                let mut gas_used: u32 = 3;
                let res: U256 = match op {
                    op::EQ => {
                        if s0 == s1 {
                            VAL_1
                        } else {
                            U256::ZERO
                        }
                    }
                    op::LT => {
                        if s0 < s1 {
                            VAL_1
                        } else {
                            U256::ZERO
                        }
                    }
                    op::GT => {
                        if s0 > s1 {
                            VAL_1
                        } else {
                            U256::ZERO
                        }
                    }
                    op::SUB => s0 - s1,
                    op::ADD => s0 + s1,
                    op::DIV => {
                        gas_used = 5;
                        if s1.is_zero() {
                            U256::ZERO
                        } else {
                            s0 / s1
                        }
                    }
                    op::MUL => {
                        gas_used = 5;
                        s0 * s1
                    }
                    op::EXP => {
                        gas_used = 50 * (1 + s1.bit_len() / 8) as u32; // ~approx
                        s0.pow(s1)
                    }
                    op::XOR => s0 ^ s1,
                    op::AND => s0 & s1,
                    op::OR => s0 | s1,
                    op::SHR => {
                        if s0 >= VAL_256 {
                            U256::ZERO
                        } else {
                            s1 >> s0
                        }
                    }
                    op::SHL => {
                        if s0 >= VAL_256 {
                            U256::ZERO
                        } else {
                            s1 << s0
                        }
                    }
                    op::SLT | op::SGT => {
                        let sign0 = s0.bit(255);
                        let sign1 = s1.bit(255);
                        U256::from(if op == op::SLT {
                            if sign0 == sign1 {
                                s0 < s1
                            } else {
                                sign0
                            }
                        } else if sign0 == sign1 {
                            // op::SGT
                            s0 > s1
                        } else {
                            !sign0
                        })
                    }
                    op::BYTE => {
                        if s0 >= VAL_32 {
                            U256::ZERO
                        } else {
                            let i: usize = s0.to();
                            U256::from(raws1.data[i])
                        }
                    }
                    _ => {
                        panic!("bug");
                    }
                };
                self.stack.push_uint(res);
                let mut ret = StepResult::new(op, gas_used);
                ret.fa = Some(raws0);
                ret.sa = Some(raws1);
                Ok(ret)
            }

            op::ISZERO => {
                let raws0 = self.stack.pop()?;
                self.stack.push(Element {
                    data: if raws0.data == VAL_0_B {
                        VAL_1_B
                    } else {
                        VAL_0_B
                    },
                    label: None,
                });
                let mut ret = StepResult::new(op, 3);
                ret.fa = Some(raws0);
                Ok(ret)
            }

            op::POP => {
                let _ = self.stack.pop();
                Ok(StepResult::new(op, 2))
            }

            op::CALLVALUE => {
                self.stack.push_uint(U256::ZERO); // msg.value == 0
                Ok(StepResult::new(op, 2))
            }

            op::CALLDATALOAD => {
                let raws0 = self.stack.pop()?;
                let offset: U256 = (&raws0).into();
                self.stack.push(self.calldata.load(offset, 32));
                let mut ret = StepResult::new(op, 3);
                ret.fa = Some(raws0);
                Ok(ret)
            }

            op::CALLDATASIZE => {
                self.stack.push(Element {
                    data: VAL_32_B,
                    label: None,
                });
                Ok(StepResult::new(op, 2))
            }

            op::SWAP1..=op::SWAP16 => {
                self.stack.swap(op - op::SWAP1 + 1)?;
                Ok(StepResult::new(op, 3))
            }

            op::MSTORE => {
                let off = self.stack.pop_uint()?;
                let val = self.stack.pop()?;
                let off32: u32 = off.try_into()?;
                self.memory.store(off32, val.data.to_vec(), val.label);
                Ok(StepResult::new(op, 3))
            }

            op::MLOAD => {
                let off = self.stack.pop_uint()?;
                let off32: u32 = off.try_into()?;
                let (val, used) = self.memory.load(off32);
                self.stack.push(Element {
                    data: val,
                    label: None,
                });
                let mut ret = StepResult::new(op, 4);
                ret.ul = Some(used);
                Ok(ret)
            }

            op::NOT => {
                let v = self.stack.pop_uint()?;
                self.stack.push_uint(!v);
                Ok(StepResult::new(op, 3))
            }

            op::SIGNEXTEND => {
                let raws0 = self.stack.pop()?;
                let raws1 = self.stack.pop()?;

                let s0: U256 = (&raws0).into();
                let s1: U256 = (&raws1).into();

                self.stack.push_uint(if s0 < VAL_32 {
                    let sign_bit_idx = (raws0.data[31] * 8 + 7) as usize;
                    let mask = (VAL_1 << sign_bit_idx) - VAL_1;
                    if s1.bit(sign_bit_idx) {
                        s1 | !mask
                    } else {
                        s1 & mask
                    }
                } else {
                    s1
                });

                let mut ret = StepResult::new(op, 5);
                ret.fa = Some(raws0);
                ret.sa = Some(raws1);
                Ok(ret)
            }

            op::ADDRESS => {
                self.stack.push(Element {
                    data: VAL_1_B,
                    label: None,
                });
                Ok(StepResult::new(op, 2))
            }

            op::CALLDATACOPY => {
                let mem_off = self.stack.pop_uint()?;
                let src_off = self.stack.pop_uint()?;
                let size = self.stack.pop_uint()?;

                let size32: usize = size.try_into()?;
                if size32 > 256 {
                    Err(UnsupportedOpError { op }.into())
                } else {
                    let mem_off32: u32 = mem_off.try_into()?; // TODO: custom error?

                    let value = self.calldata.load(src_off, size32);
                    let mut data: Vec<u8> = vec![0; size32];

                    let l = std::cmp::min(size32, 32);
                    data[0..l].copy_from_slice(&value.data[0..l]);

                    self.memory.store(mem_off32, data, value.label);
                    Ok(StepResult::new(op, 4))
                }
            }

            _ => Err(UnsupportedOpError { op }.into()),
        }
    }
}
