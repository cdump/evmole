from typing import Any

from .opcodes import Op
from .stack import Stack
from .memory import Memory

E256 = 2**256
E256M1 = E256 - 1


class Vm:
    def __init__(self, *, code: bytes, calldata):
        self.code = code
        self.pc = 0
        self.stack = Stack()
        self.memory = Memory()
        self.stopped = False
        self.calldata = calldata

    def __str__(self):
        return '\n'.join(
            (
                f'Vm ({id(self):x}):',
                f' .pc = {self.pc} | {self.current_op()}',
                f' .stack = {self.stack}',
                f' .memory = {self.memory}',
            )
        )

    def __copy__(self):
        obj = Vm(code=self.code, calldata=self.calldata)
        obj.pc = self.pc
        obj.memory._data = self.memory._data[:]
        obj.stack._data = self.stack._data[:]
        obj.stopped = self.stopped
        return obj

    def current_op(self) -> Op:
        return Op(self.code[self.pc])

    def step(self) -> tuple[Op, int, *tuple[Any, ...]]:
        ret = self._exec_next_opcode()
        op, gas_used = ret[0], ret[1]
        assert gas_used != -1, f'Op {op} with unset gas_used'

        if op not in {Op.JUMP, Op.JUMPI}:
            self.pc += op.blen

        if self.pc >= len(self.code):
            self.stopped = True
        return ret

    def _exec_next_opcode(self) -> tuple[Op, int, *tuple[Any, ...]]:
        op = self.current_op()
        gas_used = op.gas if op.gas is not None else -1
        match op:
            case op if op >= Op.PUSH0 and op <= Op.PUSH32:
                n = op - Op.PUSH0
                args = self.code[(self.pc + 1) : (self.pc + 1 + n)] if n != 0 else b'\x00'
                self.stack.push(args)
                return (op, gas_used)

            case op if op in {Op.JUMP, Op.JUMPI}:
                s0 = self.stack.pop_uint()
                assert self.code[s0] == Op.JUMPDEST.code, 'not JUMPDEST, pos %d op %02x' % (s0, self.code[s0])
                if op == Op.JUMPI:
                    s1 = self.stack.pop_uint()
                    if s1 == 0:
                        self.pc += 1
                        return (op, gas_used)
                self.pc = s0
                return (op, gas_used)

            case op if op >= Op.DUP1 and op <= Op.DUP16:
                self.stack.dup(op - Op.DUP1 + 1)
                return (op, gas_used)

            case Op.JUMPDEST:
                return (op, gas_used)

            case Op.REVERT:
                self.stack.pop()
                self.stack.pop()
                self.stopped = True
                return (op, 4)

            case Op.ISZERO:
                raws0 = self.stack.pop()
                s0 = int.from_bytes(raws0, 'big', signed=False)
                res = 0 if s0 else 1
                self.stack.push_uint(res)
                return (op, gas_used, raws0)

            case Op.POP:
                self.stack.pop()
                return (op, gas_used)

            case op if op in {Op.LT, Op.GT, Op.EQ, Op.SUB, Op.DIV, Op.EXP, Op.XOR, Op.AND, Op.SHR}:
                raws0 = self.stack.pop()
                raws1 = self.stack.pop()

                s0 = int.from_bytes(raws0, 'big', signed=False)
                s1 = int.from_bytes(raws1, 'big', signed=False)

                match op:
                    case Op.EQ:
                        res = 1 if s0 == s1 else 0
                    case Op.GT:
                        res = 1 if s0 > s1 else 0
                    case Op.LT:
                        res = 1 if s0 < s1 else 0
                    case Op.SUB:
                        res = (s0 - s1) & E256M1
                    case Op.DIV:
                        res = 0 if s1 == 0 else s0 // s1
                    case Op.EXP:
                        res = pow(s0, s1, E256)
                        gas_used = 50 * (1 + (s1.bit_length() // 8))  # ~approx
                    case Op.XOR:
                        res = s0 ^ s1
                    case Op.AND:
                        res = s0 & s1
                    case Op.SHR:
                        res = 0 if s0 >= 256 else (s1 >> s0) & E256M1
                    case _:
                        raise Exception(f'BUG: op {op} not handled in match')

                self.stack.push_uint(res)
                return (op, gas_used, raws0, raws1)

            case Op.CALLVALUE:
                self.stack.push_uint(0)  # msg.value == 0
                return (op, gas_used)

            case Op.CALLDATALOAD:
                offset = self.stack.pop_uint()
                self.stack.push(self.calldata.load(offset))
                return (op, gas_used)

            case Op.CALLDATASIZE:
                self.stack.push_uint(len(self.calldata))
                return (op, gas_used)

            case op if op >= Op.SWAP1 and op <= Op.SWAP16:
                self.stack.swap(op - Op.SWAP1 + 1)
                return (op, gas_used)

            case Op.MSTORE:
                offset = self.stack.pop_uint()
                value = self.stack.pop()
                self.memory.store(offset, value)
                return (op, 3)

            case Op.MLOAD:
                offset = self.stack.pop_uint()
                val, used = self.memory.load(offset)
                self.stack.push(val)
                return (op, 4, used)

            case Op.CALLDATACOPY:
                mem_off = self.stack.pop_uint()
                src_off = self.stack.pop_uint()
                size = self.stack.pop_uint()
                value = self.calldata.load(src_off, size)
                self.memory.store(mem_off, value)
                return (op, 4)

            case _:
                raise Exception(f'unknown op {op}')
