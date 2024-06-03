from typing import Any

from .element import Element
from .memory import Memory
from .opcodes import Op, OpCode
from .opcodes import name as opcode2name
from .stack import Stack

E256 = 2**256
E256M1 = E256 - 1


class UnsupportedOpError(Exception):
    op: OpCode

    def __init__(self, op):
        self.op = op

    def __str__(self):
        return f'{self.__class__.__name__}({opcode2name(self.op)})'


class Vm:
    def __init__(self, *, code: bytes, calldata: Element):
        self.code = code
        self.pc = 0
        self.stack = Stack()
        self.memory = Memory()
        self.stopped = len(code) == 0
        self.calldata = calldata

    def __str__(self):
        return '\n'.join(
            (
                'Vm:',
                f' .pc = {hex(self.pc)} | {opcode2name(self.current_op()) if not self.stopped else ""}',
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

    def current_op(self) -> OpCode:
        return OpCode(self.code[self.pc])

    def step(self) -> tuple[OpCode, int, *tuple[Any, ...]]:
        op = self.current_op()
        ret = self._exec_opcode(op)
        if op not in {Op.JUMP, Op.JUMPI}:
            self.pc += 1

        if self.pc >= len(self.code):
            self.stopped = True
        return (op, *ret)

    def _bop(self, cb):
        raws0 = self.stack.pop()
        raws1 = self.stack.pop()

        s0 = int.from_bytes(raws0.data, 'big', signed=False)
        s1 = int.from_bytes(raws1.data, 'big', signed=False)

        gas_used, res = cb(raws0, s0, raws1, s1)

        self.stack.push_uint(res)
        return (gas_used, raws0, raws1)

    def _exec_opcode(self, op: OpCode) -> tuple[int, *tuple[Any, ...]]:
        match op:
            case op if op >= Op.PUSH0 and op <= Op.PUSH32:
                n = op - Op.PUSH0
                args = self.code[(self.pc + 1) : (self.pc + 1 + n)].rjust(32, b'\x00')
                self.stack.push(Element(data=args))
                self.pc += n
                return (2 if n == 0 else 3,)

            case op if op in {Op.JUMP, Op.JUMPI}:
                s0 = self.stack.pop_uint()
                if op == Op.JUMPI:
                    s1 = self.stack.pop_uint()
                    if s1 == 0:
                        self.pc += 1
                        return (10,)
                if s0 >= len(self.code) or self.code[s0] != Op.JUMPDEST:
                    raise UnsupportedOpError(op)
                self.pc = s0
                return (8 if op == Op.JUMP else 10,)

            case op if op >= Op.DUP1 and op <= Op.DUP16:
                self.stack.dup(op - Op.DUP1 + 1)
                return (3,)

            case Op.JUMPDEST:
                return (1,)

            case Op.REVERT:
                # skip 2 stack pop()s
                self.stopped = True
                return (4,)

            case Op.EQ:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 1 if s0 == s1 else 0))

            case Op.LT:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 1 if s0 < s1 else 0))

            case Op.GT:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 1 if s0 > s1 else 0))

            case Op.SUB:
                return self._bop(lambda raws0, s0, raws1, s1: (3, (s0 - s1) & E256M1))

            case Op.ADD:
                return self._bop(lambda raws0, s0, raws1, s1: (3, (s0 + s1) & E256M1))

            case Op.DIV:
                return self._bop(lambda raws0, s0, raws1, s1: (5, 0 if s1 == 0 else s0 // s1))

            case Op.MUL:
                return self._bop(lambda raws0, s0, raws1, s1: (5, (s0 * s1) & E256M1))

            case Op.EXP:
                return self._bop(
                    lambda raws0, s0, raws1, s1: (50 * (1 + (s1.bit_length() // 8)), pow(s0, s1, E256))
                )  # ~approx gas

            case Op.XOR:
                return self._bop(lambda raws0, s0, raws1, s1: (3, s0 ^ s1))

            case Op.AND:
                return self._bop(lambda raws0, s0, raws1, s1: (3, s0 & s1))

            case Op.OR:
                return self._bop(lambda raws0, s0, raws1, s1: (3, s0 | s1))

            case Op.SHR:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 0 if s0 >= 256 else (s1 >> s0) & E256M1))

            case Op.SHL:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 0 if s0 >= 256 else (s1 << s0) & E256M1))

            case Op.BYTE:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 0 if s0 >= 32 else raws1.data[s0]))

            case op if op in {Op.SLT, Op.SGT}:
                raws0 = self.stack.pop()
                raws1 = self.stack.pop()

                s0 = int.from_bytes(raws0.data, 'big', signed=True)
                s1 = int.from_bytes(raws1.data, 'big', signed=True)
                if op == Op.SLT:
                    res = 1 if s0 < s1 else 0
                else:
                    res = 1 if s0 > s1 else 0
                self.stack.push_uint(res)
                return (3,)

            case Op.ISZERO:
                raws0 = self.stack.pop()
                s0 = int.from_bytes(raws0.data, 'big', signed=False)
                res = 0 if s0 else 1
                self.stack.push_uint(res)
                return (3, raws0)

            case Op.POP:
                self.stack.pop()
                return (2,)

            case Op.CALLVALUE:
                self.stack.push_uint(0)  # msg.value == 0
                return (2,)

            case Op.CALLDATALOAD:
                raws0 = self.stack.pop()
                offset = int.from_bytes(raws0.data, 'big', signed=False)
                self.stack.push(self.calldata.load(offset))
                return (3, raws0)

            case Op.CALLDATASIZE:
                self.stack.push_uint(len(self.calldata))
                return (2,)

            case op if op >= Op.SWAP1 and op <= Op.SWAP16:
                self.stack.swap(op - Op.SWAP1 + 1)
                return (3,)

            case Op.MSTORE:
                offset = self.stack.pop_uint()
                value = self.stack.pop()
                self.memory.store(offset, value)
                return (3,)

            case Op.MLOAD:
                offset = self.stack.pop_uint()
                val, used = self.memory.load(offset)
                self.stack.push(Element(data=val))
                return (4, used)

            case Op.NOT:
                s0 = self.stack.pop_uint()
                self.stack.push_uint(E256M1 - s0)
                return (3,)

            case Op.SIGNEXTEND:
                s0 = self.stack.pop_uint()
                raws1 = self.stack.pop()
                s1 = int.from_bytes(raws1.data, 'big', signed=False)
                if s0 <= 31:
                    sign_bit = 1 << (s0 * 8 + 7)
                    if s1 & sign_bit:
                        res = s1 | (E256 - sign_bit)
                    else:
                        res = s1 & (sign_bit - 1)
                else:
                    res = s1

                self.stack.push_uint(res)
                return (5, s0, raws1)

            case Op.ADDRESS:
                self.stack.push_uint(0)
                return (2,)

            case Op.CALLDATACOPY:
                mem_off = self.stack.pop_uint()
                src_off = self.stack.pop_uint()
                size = self.stack.pop_uint()
                if size > 256:
                    raise UnsupportedOpError(op)
                value = self.calldata.load(src_off, size)
                self.memory.store(mem_off, value)
                return (4,)

            case Op.ORIGIN | Op.CALLER:
                self.stack.push_uint(0)
                return (2,)

            case Op.SLOAD:
                self.stack.pop()
                self.stack.push_uint(0)
                return (100,)

            case _:
                raise UnsupportedOpError(op)
