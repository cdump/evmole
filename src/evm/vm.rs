use std::{collections::HashSet, error, fmt};

use super::{memory::Memory, op, stack::Stack, Element, U256};
use super::{VAL_0_B, VAL_1, VAL_1024, VAL_1M, VAL_1_B, VAL_256, VAL_32, VAL_4};

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
    fn new(op: op::OpCode, gas_used: u32) -> Self {
        Self {
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
    pub code: &'a [u8],
    pub pc: usize,
    pub stack: Stack<T>,
    pub memory: Memory<T>,
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
            "Vm:\n .pc = 0x{:x} | {}\n .stack = {:?}\n .memory = {:?}",
            self.pc,
            if !self.stopped {
                op::name(self.code[self.pc])
            } else {
                ""
            },
            self.stack,
            self.memory
        )
    }
}

impl<'a, T> Vm<'a, T>
where
    T: std::fmt::Debug + Clone + Eq + std::hash::Hash,
{
    pub fn new(code: &'a [u8], calldata: Element<T>) -> Self {
        Self {
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

    #[allow(clippy::type_complexity)]
    fn bop(
        &mut self,
        op: op::OpCode,
        f: fn(&Element<T>, U256, &Element<T>, U256) -> (u32, U256),
    ) -> Result<StepResult<T>, Box<dyn error::Error>> {
        let raws0 = self.stack.pop()?;
        let raws1 = self.stack.pop()?;

        let s0: U256 = (&raws0).into();
        let s1: U256 = (&raws1).into();

        let (gas_used, res) = f(&raws0, s0, &raws1, s1);

        self.stack.push_uint(res);
        let mut ret = StepResult::new(op, gas_used);
        ret.fa = Some(raws0);
        ret.sa = Some(raws1);
        Ok(ret)
    }

    fn exec_opcode(&mut self, op: op::OpCode) -> Result<StepResult<T>, Box<dyn error::Error>> {
        match op {
            op::PUSH0..=op::PUSH32 => {
                let n = (op - op::PUSH0) as usize;
                if self.pc + 1 + n > self.code.len() {
                    return Err(UnsupportedOpError { op }.into());
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

            op::DUP1..=op::DUP16 => {
                self.stack.dup(op - op::DUP1 + 1)?;
                Ok(StepResult::new(op, 3))
            }

            op::SWAP1..=op::SWAP16 => {
                self.stack.swap(op - op::SWAP1 + 1)?;
                Ok(StepResult::new(op, 3))
            }

            op::JUMP | op::JUMPI => {
                let s0 = self.stack.pop_uint()?;
                let mut ret = StepResult::new(op, if op == op::JUMP { 8 } else { 10 });
                if op == op::JUMPI {
                    ret.sa = Some(self.stack.peek()?.clone());
                    let s1 = self.stack.pop_uint()?;
                    if s1.is_zero() {
                        self.pc += 1;
                        ret.fa = Some(Element {
                            data: s0.to_be_bytes(),
                            label: None,
                        });
                        return Ok(ret);
                    } else {
                        ret.fa = Some(Element {
                            data: U256::from(self.pc + 1).to_be_bytes(),
                            label: None,
                        });
                    }
                }
                let cres: Result<usize, _> = s0.try_into();
                if let Ok(newpc) = cres {
                    if newpc >= self.code.len() || self.code[newpc] != op::JUMPDEST {
                        Err(UnsupportedOpError { op }.into())
                    } else {
                        self.pc = newpc;
                        Ok(ret)
                    }
                } else {
                    Err(UnsupportedOpError { op }.into())
                }
            }

            op::JUMPDEST => Ok(StepResult::new(op, 1)),

            op::ADD => self.bop(op, |_, s0, _, s1| (3, s0 + s1)),

            op::MUL => self.bop(op, |_, s0, _, s1| (5, s0 * s1)),

            op::SUB => self.bop(op, |_, s0, _, s1| (3, s0 - s1)),

            op::DIV => self.bop(op, |_, s0, _, s1| {
                (5, if s1.is_zero() { U256::ZERO } else { s0 / s1 })
            }),

            op::MOD => self.bop(op, |_, s0, _, s1| {
                (5, if s1.is_zero() { U256::ZERO } else { s0 % s1 })
            }),

            op::EXP => self.bop(op, |_, s0, _, s1| {
                (
                    50 * (1 + s1.bit_len() / 8) as u32, /*approx*/
                    s0.pow(s1),
                )
            }),

            op::SIGNEXTEND => self.bop(op, |raws0, s0, _, s1| {
                (
                    5,
                    if s0 < VAL_32 {
                        let sign_bit_idx = (raws0.data[31] * 8 + 7) as usize;
                        let mask = (VAL_1 << sign_bit_idx) - VAL_1;
                        if s1.bit(sign_bit_idx) {
                            s1 | !mask
                        } else {
                            s1 & mask
                        }
                    } else {
                        s1
                    },
                )
            }),

            op::LT => self.bop(op, |_, s0, _, s1| {
                (3, if s0 < s1 { VAL_1 } else { U256::ZERO })
            }),

            op::GT => self.bop(op, |_, s0, _, s1| {
                (3, if s0 > s1 { VAL_1 } else { U256::ZERO })
            }),

            op::SLT => self.bop(op, |_, s0, _, s1| {
                (3, {
                    let sign0 = s0.bit(255);
                    let sign1 = s1.bit(255);
                    U256::from(if sign0 == sign1 { s0 < s1 } else { sign0 })
                })
            }),

            op::SGT => self.bop(op, |_, s0, _, s1| {
                (3, {
                    let sign0 = s0.bit(255);
                    let sign1 = s1.bit(255);
                    U256::from(if sign0 == sign1 { s0 > s1 } else { !sign0 })
                })
            }),

            op::EQ => self.bop(op, |_, s0, _, s1| {
                (3, if s0 == s1 { VAL_1 } else { U256::ZERO })
            }),

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

            op::AND => self.bop(op, |_, s0, _, s1| (3, s0 & s1)),

            op::OR => self.bop(op, |_, s0, _, s1| (3, s0 | s1)),

            op::XOR => self.bop(op, |_, s0, _, s1| (3, s0 ^ s1)),

            op::NOT => {
                let v = self.stack.pop_uint()?;
                self.stack.push_uint(!v);
                Ok(StepResult::new(op, 3))
            }

            op::BYTE => self.bop(op, |_, s0, raws1, _| {
                (3, {
                    if s0 >= VAL_32 {
                        U256::ZERO
                    } else {
                        let i: usize = s0.to();
                        U256::from(raws1.data[i])
                    }
                })
            }),

            op::SHL => self.bop(op, |_, s0, _, s1| {
                (3, if s0 >= VAL_256 { U256::ZERO } else { s1 << s0 })
            }),

            op::SHR => self.bop(op, |_, s0, _, s1| {
                (3, if s0 >= VAL_256 { U256::ZERO } else { s1 >> s0 })
            }),

            op::KECCAK256 => {
                self.stack.pop()?;
                self.stack.pop()?;
                self.stack.push_uint(VAL_1);
                Ok(StepResult::new(op, 30))
            }

            op::ADDRESS
            | op::ORIGIN
            | op::CALLER
            | op::COINBASE
            | op::CALLVALUE
            | op::TIMESTAMP
            | op::NUMBER
            | op::PREVRANDAO
            | op::GASLIMIT
            | op::CHAINID
            | op::BASEFEE
            | op::BLOBBASEFEE
            | op::GASPRICE => {
                self.stack.push_uint(U256::ZERO);
                Ok(StepResult::new(op, 2))
            }

            op::BALANCE => {
                self.stack.pop()?;
                self.stack.push_uint(U256::ZERO);
                Ok(StepResult::new(op, 100))
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
                self.stack.push_uint(VAL_4);
                Ok(StepResult::new(op, 2))
            }

            op::CALLDATACOPY => {
                let raws0 = self.stack.pop()?;
                let mem_off: U256 = (&raws0).into();
                let mem_off32: u32 = mem_off.try_into()?;

                let raws1 = self.stack.pop()?;
                let src_off: U256 = (&raws1).into();

                let size: usize = self.stack.pop_uint()?.try_into()?;

                if size > 512 {
                    Err(UnsupportedOpError { op }.into())
                } else {
                    let value = self.calldata.load(src_off, size);
                    let mut data: Vec<u8> = vec![0; size];

                    let l = std::cmp::min(size, 32);
                    data[0..l].copy_from_slice(&value.data[0..l]);

                    self.memory.store(mem_off32, data, value.label.clone());

                    let mut ret = StepResult::new(op, 4);
                    ret.fa = Some(raws1); // calldata offset, like in CALLDATALOAD
                    ret.sa = Some(raws0); // memory off
                    Ok(ret)
                }
            }

            op::CODESIZE => {
                self.stack.push_uint(U256::from(self.code.len()));
                Ok(StepResult::new(op, 2))
            }

            op::CODECOPY => {
                let mem_off: u32 = self.stack.pop_uint()?.try_into()?;
                let src_off: usize = self.stack.pop_uint()?.try_into()?;
                let size: usize = self.stack.pop_uint()?.try_into()?;

                if size > 32768 {
                    Err(UnsupportedOpError { op }.into())
                } else {
                    let mut data: Vec<u8> = vec![0; size];
                    let code_len = self.code.len();
                    if src_off < code_len {
                        let n = std::cmp::min(size, code_len - src_off);
                        data[0..n].copy_from_slice(&self.code[src_off..src_off + n]);
                    }
                    self.memory.store(mem_off, data, None);
                    Ok(StepResult::new(op, 3))
                }
            }

            op::EXTCODESIZE | op::EXTCODEHASH => {
                self.stack.pop()?;
                self.stack.push_uint(VAL_1);
                Ok(StepResult::new(op, 100))
            }

            op::RETURNDATASIZE => {
                self.stack.push_uint(VAL_1024);
                Ok(StepResult::new(op, 2))
            }

            op::RETURNDATACOPY => {
                let mem_off: u32 = self.stack.pop_uint()?.try_into()?;
                self.stack.pop()?;
                let size: usize = self.stack.pop_uint()?.try_into()?;
                if size > 1024 {
                    Err(UnsupportedOpError { op }.into())
                } else {
                    let data: Vec<u8> = vec![0; size];
                    self.memory.store(mem_off, data, None);
                    Ok(StepResult::new(op, 3))
                }
            }

            op::BLOCKHASH => {
                self.stack.pop()?;
                self.stack.push_uint(VAL_1);
                Ok(StepResult::new(op, 20))
            }

            op::SELFBALANCE => {
                self.stack.push_uint(U256::ZERO);
                Ok(StepResult::new(op, 5))
            }

            op::POP => {
                self.stack.pop()?;
                Ok(StepResult::new(op, 2))
            }

            op::MLOAD => {
                let off: u32 = self.stack.pop_uint()?.try_into()?;
                let (val, used) = self.memory.load(off);

                self.stack.push(val);
                let mut ret = StepResult::new(op, 4);
                ret.ul = Some(used);
                Ok(ret)
            }

            op::MSTORE => {
                let off = self.stack.pop_uint()?.try_into()?;
                let val = self.stack.pop()?;

                self.memory.store(off, val.data.to_vec(), val.label);
                Ok(StepResult::new(op, 3))
            }

            op::MSTORE8 => {
                let off: u32 = self.stack.pop_uint()?.try_into()?;
                let val = self.stack.pop()?;

                self.memory.store(off, vec![val.data[31]], val.label);
                Ok(StepResult::new(op, 3))
            }

            op::MSIZE => {
                self.stack.push_uint(U256::from(self.memory.size()));
                Ok(StepResult::new(op, 2))
            }

            op::SLOAD => {
                let slot = self.stack.pop()?;
                let mut ret = StepResult::new(op, 100);
                ret.fa = Some(slot);
                self.stack.push_uint(U256::ZERO);
                Ok(ret)
            }

            op::SSTORE => {
                let slot = self.stack.pop()?;
                let sval = self.stack.pop()?;
                let mut ret = StepResult::new(op, 100);
                ret.fa = Some(slot);
                ret.sa = Some(sval);
                Ok(ret)
            }

            op::GAS => {
                self.stack.push_uint(VAL_1M);
                Ok(StepResult::new(op, 2))
            }

            op::LOG0..=op::LOG4 => {
                let n = (op - op::LOG0) as u32;
                for _ in 0..n + 2 {
                    self.stack.pop()?;
                }
                Ok(StepResult::new(op, 375 * (n + 1)))
            }

            op::CREATE | op::CREATE2 => {
                self.stack.pop()?;
                self.stack.pop()?;
                self.stack.pop()?;
                if op == op::CREATE2 {
                    self.stack.pop()?;
                }
                self.stack.push_uint(U256::ZERO);
                Ok(StepResult::new(op, 32000))
            }

            op::CALL | op::DELEGATECALL | op::STATICCALL => {
                self.stack.pop()?;
                let p1 = self.stack.pop()?;
                let p2 = self.stack.pop()?;
                self.stack.pop()?;
                self.stack.pop()?;
                self.stack.pop()?;

                if op == op::CALL {
                    self.stack.pop()?;
                }

                self.stack.push_uint(U256::ZERO); // failure

                let mut ret = StepResult::new(op, 100);
                ret.fa = Some(p1);
                if op == op::CALL {
                    ret.sa = Some(p2);
                }
                Ok(ret)
            }

            op::REVERT | op::RETURN => {
                self.stopped = true;
                let offset = self.stack.pop()?;
                let size = self.stack.pop()?;
                let mut ret = StepResult::new(op, 5);
                ret.fa = Some(offset);
                ret.sa = Some(size);
                Ok(ret)
            }

            op::STOP | op::SELFDESTRUCT => {
                // skip stack pop()s
                self.stopped = true;
                Ok(StepResult::new(op, 5))
            }
            _ => Err(UnsupportedOpError { op }.into()),
        }
    }
}
