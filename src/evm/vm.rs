use super::{I256, U256, calldata::CallData, element::Element, memory::{Memory, MemoryChunks}, op, stack::Stack};
use super::{VAL_0_B, VAL_1, VAL_1_B, VAL_1M_B, VAL_32, VAL_256, VAL_1024_B};
use std::{error, fmt};

#[derive(Debug)]
pub struct UnsupportedOpError {
    pub op: op::OpCode,
}
impl std::fmt::Display for UnsupportedOpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UnsupportedOpError({})", op::info(self.op).name)
    }
}

impl std::error::Error for UnsupportedOpError {}

pub struct StepResult<T> {
    pub op: op::OpCode,
    pub gas_used: u32,
    pub args: [Element<T>; 2],
    pub exargs: Vec<Element<T>>,
    pub memory_load: Option<MemoryChunks<T>>, // for MLOAD & MCOPY
}

impl<T> StepResult<T> {
    fn new(op: op::OpCode, gas_used: u32) -> Self {
        Self {
            op,
            gas_used,
            args: [const {
                Element {
                    data: [0; 32],
                    label: None,
                }
            }; 2],
            exargs: Vec::new(),
            memory_load: None,
        }
    }
}

pub struct Vm<'a, T, U> {
    pub code: &'a [u8],
    pub pc: usize,
    pub stack: Stack<T>,
    pub memory: Memory<T>,
    pub stopped: bool,
    pub calldata: &'a U,
}

impl<T, U> fmt::Debug for Vm<'_, T, U>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Vm:\n .pc = 0x{:x} ({}) | {}\n .stack = {:?}\n .memory = {:?}",
            self.pc,
            self.pc,
            if !self.stopped {
                op::info(self.code[self.pc]).name
            } else {
                ""
            },
            self.stack,
            self.memory
        )
    }
}

impl<'a, T, U> Vm<'a, T, U>
where
    T: std::fmt::Debug + Clone + Eq,
    U: CallData<T>,
{
    pub fn new(code: &'a [u8], calldata: &'a U) -> Self {
        Self {
            code,
            pc: 0,
            stack: Stack::<T>::new(),
            memory: Memory::<T>::new(),
            stopped: code.is_empty(),
            calldata,
        }
    }

    pub fn fork(&self) -> Self {
        Vm {
            code: self.code,
            pc: self.pc,
            stack: self.stack.clone(),
            memory: self.memory.clone(),
            stopped: self.stopped,
            calldata: self.calldata,
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
        ret.args[0] = raws0;
        ret.args[1] = raws1;
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
                self.stack.push_data(args);
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
                let cres = usize::try_from(s0);
                let mut ret = StepResult::new(op, if op == op::JUMP { 8 } else { 10 });
                if op == op::JUMPI {
                    ret.args[1] = self.stack.peek()?.clone();
                    let s1 = self.stack.pop_uint()?;
                    if s1.is_zero() {
                        self.pc += 1;
                        if let Ok(other_pc) = cres {
                            if other_pc < self.code.len() {
                                ret.args[0] = Element {
                                    data: s0.to_be_bytes(),
                                    label: None,
                                };
                            }
                        }
                        return Ok(ret);
                    } else {
                        let other_pc = self.pc + 1;
                        if other_pc < self.code.len() {
                            ret.args[0] = Element {
                                data: U256::from(other_pc).to_be_bytes(),
                                label: None,
                            };
                        }
                    }
                }
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

            op::SDIV => self.bop(op, |_, s0, _, s1| {
                (
                    5,
                    if s1.is_zero() {
                        U256::ZERO
                    } else {
                        I256::from_raw(s0)
                            .wrapping_div(I256::from_raw(s1))
                            .into_raw()
                    },
                )
            }),

            op::MOD => self.bop(op, |_, s0, _, s1| {
                (5, if s1.is_zero() { U256::ZERO } else { s0 % s1 })
            }),

            op::SMOD => self.bop(op, |_, s0, _, s1| {
                (
                    5,
                    if s1.is_zero() {
                        U256::ZERO
                    } else {
                        (I256::from_raw(s0) % I256::from_raw(s1)).into_raw()
                    },
                )
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
                    let sign0 = s0.bit(U256::BITS - 1);
                    let sign1 = s1.bit(U256::BITS - 1);
                    U256::from(if sign0 == sign1 { s0 < s1 } else { sign0 })
                })
            }),

            op::SGT => self.bop(op, |_, s0, _, s1| {
                (3, {
                    let sign0 = s0.bit(U256::BITS - 1);
                    let sign1 = s1.bit(U256::BITS - 1);
                    U256::from(if sign0 == sign1 { s0 > s1 } else { !sign0 })
                })
            }),

            op::EQ => self.bop(op, |_, s0, _, s1| {
                (3, if s0 == s1 { VAL_1 } else { U256::ZERO })
            }),

            op::ISZERO => {
                let raws0 = self.stack.pop()?;
                self.stack.push_data(if raws0.data == VAL_0_B {
                    VAL_1_B
                } else {
                    VAL_0_B
                });
                let mut ret = StepResult::new(op, 3);
                ret.args[0] = raws0;
                Ok(ret)
            }

            op::AND => self.bop(op, |_, s0, _, s1| (3, s0 & s1)),

            op::OR => self.bop(op, |_, s0, _, s1| (3, s0 | s1)),

            op::XOR => self.bop(op, |_, s0, _, s1| (3, s0 ^ s1)),

            op::NOT => {
                let raws0 = self.stack.pop()?;
                let v: U256 = (&raws0).into();
                self.stack.push_uint(!v);
                let mut ret = StepResult::new(op, 3);
                ret.args[0] = raws0;
                Ok(ret)
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

            op::SAR => self.bop(op, |_, s0, _, s1| {
                (
                    3,
                    if s0 < VAL_256 {
                        s1 >> s0
                    } else if s1.bit(U256::BITS - 1) {
                        U256::MAX
                    } else {
                        U256::ZERO
                    },
                )
            }),

            op::MULMOD | op::ADDMOD => {
                let raws0 = self.stack.pop()?;
                let raws1 = self.stack.pop()?;
                let raws2 = self.stack.pop()?;
                let s0: U256 = (&raws0).into();
                let s1: U256 = (&raws1).into();
                let s2: U256 = (&raws2).into();
                self.stack.push_uint(if op == op::MULMOD {
                    s0.mul_mod(s1, s2)
                } else {
                    s0.add_mod(s1, s2)
                });
                let mut ret = StepResult::new(op, 8);
                ret.args[0] = raws0;
                ret.args[1] = raws1;
                ret.exargs.push(raws2);
                Ok(ret)
            }

            op::KECCAK256 => {
                let offset = self.stack.pop()?;
                let size = self.stack.pop()?;
                let mut ret = StepResult::new(op, 30 + 6 * 32);
                ret.args[0] = offset;
                ret.args[1] = size;
                self.stack.push_data(VAL_1_B);
                Ok(ret)
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
                self.stack.push_data(VAL_0_B);
                Ok(StepResult::new(op, 2))
            }

            op::BALANCE => {
                self.stack.pop()?;
                self.stack.push_data(VAL_0_B);
                Ok(StepResult::new(op, 100))
            }

            op::CALLDATALOAD => {
                let raws0 = self.stack.pop()?;
                let offset: U256 = (&raws0).into();
                self.stack.push(self.calldata.load32(offset));
                let mut ret = StepResult::new(op, 3);
                ret.args[0] = raws0;
                Ok(ret)
            }

            op::CALLDATASIZE => {
                self.stack.push_uint(self.calldata.len());
                Ok(StepResult::new(op, 2))
            }

            op::CALLDATACOPY => {
                let raws0 = self.stack.pop()?;
                let mem_off: U256 = (&raws0).into();
                let mem_off32: u32 = mem_off.try_into()?;

                let raws1 = self.stack.pop()?;
                let src_off: U256 = (&raws1).into();

                let raws2 = self.stack.pop()?;
                let size: U256 = (&raws2).into();

                let (data, label) = self.calldata.load(src_off, size)?;
                self.memory.store(mem_off32, data, label);

                let mut ret = StepResult::new(op, 4);
                ret.args[0] = raws1; // calldata offset, like in CALLDATALOAD
                ret.args[1] = raws0; // memory off
                ret.exargs.push(raws2); // size
                Ok(ret)
            }

            op::CODESIZE => {
                self.stack.push_uint(U256::from(self.code.len()));
                Ok(StepResult::new(op, 2))
            }

            op::CODECOPY => {
                let raws0 = self.stack.pop()?;
                let src_off: usize = self.stack.pop_uint()?.try_into()?;
                let raws2 = self.stack.pop()?;
                let mem_off: u32 = (&raws0).try_into()?;
                let size: usize = (&raws2).try_into()?;

                if size > 32768 {
                    Err(UnsupportedOpError { op }.into())
                } else {
                    let mut data: Vec<u8> = vec![0; size];
                    let code_len = self.code.len();
                    if src_off < code_len {
                        let n = std::cmp::min(size, code_len - src_off);
                        data[0..n].copy_from_slice(&self.code[src_off..src_off + n]);
                    }
                    let mut ret = StepResult::new(op, 3);
                    ret.args[0] = raws0;
                    ret.args[1] = raws2;
                    self.memory.store(mem_off, data, None);
                    Ok(ret)
                }
            }

            op::EXTCODESIZE | op::EXTCODEHASH => {
                let mut ret = StepResult::new(op, 100);
                ret.args[0] = self.stack.pop()?;
                self.stack.push_data(VAL_1_B);
                Ok(ret)
            }

            op::EXTCODECOPY => {
                self.stack.pop()?;
                self.stack.pop()?;
                self.stack.pop()?;
                self.stack.pop()?;
                Ok(StepResult::new(op, 100))
            }

            op::RETURNDATASIZE => {
                self.stack.push_data(VAL_1024_B);
                Ok(StepResult::new(op, 2))
            }

            op::RETURNDATACOPY => {
                let raws0 = self.stack.pop()?;
                let mem_off: U256 = (&raws0).into();
                let mem_off32: u32 = mem_off.try_into()?;

                self.stack.pop()?;
                let size: usize = self.stack.pop_uint()?.try_into()?;
                if size > 2048 {
                    Err(UnsupportedOpError { op }.into())
                } else {
                    let data: Vec<u8> = vec![0; size];
                    self.memory.store(mem_off32, data, None);

                    let mut ret = StepResult::new(op, 3);
                    ret.args[0] = raws0;
                    Ok(ret)
                }
            }

            op::BLOCKHASH => {
                self.stack.pop()?;
                self.stack.push_data(VAL_1_B);
                Ok(StepResult::new(op, 20))
            }

            op::BLOBHASH => {
                self.stack.pop()?;
                self.stack.push_data(VAL_1_B);
                Ok(StepResult::new(op, 3))
            }

            op::SELFBALANCE => {
                self.stack.push_data(VAL_0_B);
                Ok(StepResult::new(op, 5))
            }

            op::POP => {
                self.stack.pop()?;
                Ok(StepResult::new(op, 2))
            }

            op::MLOAD => {
                let raws0 = self.stack.pop()?;
                let s0: U256 = (&raws0).into();
                let off: u32 = s0.try_into()?;
                let (val, used) = self.memory.load_element(off);
                self.stack.push(val);
                let mut ret = StepResult::new(op, 4);
                ret.memory_load = Some(used);
                ret.args[0] = raws0;
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

            op::SLOAD | op::TLOAD => {
                let mut ret = StepResult::new(op, 100);
                ret.args[0] = self.stack.pop()?; // slot
                self.stack.push_data(VAL_0_B);
                Ok(ret)
            }

            op::SSTORE | op::TSTORE => {
                let mut ret = StepResult::new(op, 100);
                ret.args[0] = self.stack.pop()?; // slot
                ret.args[1] = self.stack.pop()?; // value
                Ok(ret)
            }

            op::MCOPY => {
                let dest_offset = self.stack.pop()?;
                let dest_offset_num: U256 = (&dest_offset).into();
                let dest_offset_u32: u32 = dest_offset_num.try_into()?;

                let offset: u32 = self.stack.pop_uint()?.try_into()?;
                let size: u32 = self.stack.pop_uint()?.try_into()?;
                if size > 2048 {
                    return Err(UnsupportedOpError { op }.into());
                }

                let (data, used) = self.memory.load(offset, size);
                let label = if used.chunks.len() == 1 {
                    Some(used.chunks[0].src_label.clone())
                } else {
                    None
                };
                self.memory.store(dest_offset_u32, data, label);
                let gas_used: u32 = 3 + 3 * size.div_ceil(32);
                let mut ret = StepResult::new(op, gas_used);
                ret.memory_load = Some(used);
                ret.args[0] = dest_offset;
                Ok(ret)
            }

            op::GAS => {
                self.stack.push_data(VAL_1M_B);
                Ok(StepResult::new(op, 2))
            }

            op::PC => {
                self.stack.push_uint(U256::from(self.pc));
                Ok(StepResult::new(op, 2))
            }

            op::LOG0..=op::LOG4 => {
                let n = (op - op::LOG0) as u32;
                let mut ret = StepResult::new(op, 375 * (n + 1));
                self.stack.pop()?;
                self.stack.pop()?;
                if n > 0 {
                    ret.args[0] = self.stack.pop()?;
                    for _ in 0..n - 1 {
                        self.stack.pop()?;
                    }
                }
                Ok(ret)
            }

            op::CREATE | op::CREATE2 => {
                self.stack.pop()?;
                self.stack.pop()?;
                self.stack.pop()?;
                if op == op::CREATE2 {
                    self.stack.pop()?;
                }
                self.stack.push_data(VAL_0_B);
                Ok(StepResult::new(op, 32000))
            }

            op::CALL | op::CALLCODE | op::DELEGATECALL | op::STATICCALL => {
                let mut ret = StepResult::new(op, 100);

                let gas = self.stack.pop()?;
                let address = self.stack.pop()?;
                let p2 = self.stack.pop()?;
                let p3 = self.stack.pop()?;
                let p4 = self.stack.pop()?;
                let p5 = self.stack.pop()?;

                ret.exargs.reserve(5);
                ret.exargs.push(gas);

                ret.args[0] = address;
                if op == op::CALL || op == op::CALLCODE {
                    let p6 = self.stack.pop()?;
                    ret.args[1] = p2; // value
                    ret.exargs.push(p3); // args offset
                    ret.exargs.push(p4); // args size
                    ret.exargs.push(p5); // ret offset
                    ret.exargs.push(p6); // ret size
                } else {
                    // args[1] (value) is alredy set to zero
                    ret.exargs.push(p2); // args offset
                    ret.exargs.push(p3); // args size
                    ret.exargs.push(p4); // ret offset
                    ret.exargs.push(p5); // ret size
                }

                self.stack.push_data(VAL_1_B); // success

                Ok(ret)
            }

            op::REVERT | op::RETURN => {
                self.stopped = true;
                let offset = self.stack.pop()?;
                let size = self.stack.pop()?;
                let mut ret = StepResult::new(op, 5);
                ret.args[0] = offset;
                ret.args[1] = size;
                Ok(ret)
            }

            op::STOP | op::SELFDESTRUCT | op::INVALID => {
                // skip stack pop()s
                self.stopped = true;
                Ok(StepResult::new(op, 5))
            }
            _ => Err(UnsupportedOpError { op }.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct DummyCallData {}

    impl CallData<u8> for DummyCallData {
        fn load32(&self, _: U256) -> Element<u8> {
            Element {
                data: [0; 32],
                label: None,
            }
        }

        fn load(&self, _: U256, _: U256) -> Result<(Vec<u8>, Option<u8>), Box<dyn error::Error>> {
            Err("unsupported".into())
        }

        fn selector(&self) -> [u8; 4] {
            [0; 4]
        }

        fn len(&self) -> U256 {
            U256::ZERO
        }
    }

    #[test]
    fn test_arithmetic() {
        let mut vm = Vm::new(&[], &DummyCallData {});
        let cases = [
            (
                I256::unchecked_from(-1).into_raw(),
                op::ADD,
                U256::from(3),
                U256::from(2),
            ),
            (
                I256::unchecked_from(-1).into_raw(),
                op::LT,
                U256::from(3),
                U256::from(0),
            ),
            (
                I256::unchecked_from(-1).into_raw(),
                op::SLT,
                U256::from(3),
                U256::from(1),
            ),
            (
                I256::unchecked_from(-4).into_raw(),
                op::SDIV,
                U256::from(2),
                I256::unchecked_from(-2).into_raw(),
            ),
            (
                I256::unchecked_from(-4).into_raw(),
                op::SDIV,
                I256::unchecked_from(-2).into_raw(),
                U256::from(2),
            ),
            (
                I256::MIN.into_raw(),
                op::SDIV,
                I256::unchecked_from(-1).into_raw(),
                I256::MIN.into_raw(),
            ),
            (
                I256::unchecked_from(-4).into_raw(),
                op::SDIV,
                U256::ZERO,
                U256::ZERO,
            ),
        ];

        for (lhs, op, rhs, expected) in cases.into_iter() {
            vm.stack.push_uint(rhs);
            vm.stack.push_uint(lhs);
            assert!(vm.exec_opcode(op).is_ok());
            let r = vm.stack.pop_uint().unwrap();
            assert_eq!(r, expected);
        }
    }
}
