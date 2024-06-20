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

    def __init__(self, op) -> None:
        self.op = op

    def __str__(self) -> str:
        return f'{self.__class__.__name__}({opcode2name(self.op)})'


class Vm:
    def __init__(self, *, code: bytes, calldata: Element) -> None:
        self.code = code
        self.pc = 0
        self.stack = Stack()
        self.memory = Memory()
        self.stopped = len(code) == 0
        self.calldata = calldata

    def __str__(self) -> str:
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
        obj.memory.data = self.memory.data[:]
        obj.stack.data = self.stack.data[:]
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

            case op if op >= Op.DUP1 and op <= Op.DUP16:
                self.stack.dup(op - Op.DUP1 + 1)
                return (3,)

            case op if op >= Op.SWAP1 and op <= Op.SWAP16:
                self.stack.swap(op - Op.SWAP1 + 1)
                return (3,)

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

            case Op.JUMPDEST:
                return (1,)

            case Op.ADD:
                return self._bop(lambda raws0, s0, raws1, s1: (3, (s0 + s1) & E256M1))

            case Op.MUL:
                return self._bop(lambda raws0, s0, raws1, s1: (5, (s0 * s1) & E256M1))

            case Op.SUB:
                return self._bop(lambda raws0, s0, raws1, s1: (3, (s0 - s1) & E256M1))

            case Op.DIV:
                return self._bop(lambda raws0, s0, raws1, s1: (5, 0 if s1 == 0 else s0 // s1))

            case Op.MOD:
                return self._bop(lambda raws0, s0, raws1, s1: (5, 0 if s1 == 0 else s0 % s1))

            case Op.EXP:
                return self._bop(
                    lambda raws0, s0, raws1, s1: (50 * (1 + (s1.bit_length() // 8)), pow(s0, s1, E256))
                )  # ~approx gas

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

            case Op.LT:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 1 if s0 < s1 else 0))

            case Op.GT:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 1 if s0 > s1 else 0))

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

            case Op.EQ:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 1 if s0 == s1 else 0))

            case Op.ISZERO:
                raws0 = self.stack.pop()
                s0 = int.from_bytes(raws0.data, 'big', signed=False)
                res = 0 if s0 else 1
                self.stack.push_uint(res)
                return (3, raws0)

            case Op.AND:
                return self._bop(lambda raws0, s0, raws1, s1: (3, s0 & s1))

            case Op.OR:
                return self._bop(lambda raws0, s0, raws1, s1: (3, s0 | s1))

            case Op.XOR:
                return self._bop(lambda raws0, s0, raws1, s1: (3, s0 ^ s1))

            case Op.NOT:
                s0 = self.stack.pop_uint()
                self.stack.push_uint(E256M1 - s0)
                return (3,)

            case Op.BYTE:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 0 if s0 >= 32 else raws1.data[s0]))

            case Op.SHL:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 0 if s0 >= 256 else (s1 << s0) & E256M1))

            case Op.SHR:
                return self._bop(lambda raws0, s0, raws1, s1: (3, 0 if s0 >= 256 else (s1 >> s0) & E256M1))

            case Op.KECCAK256:
                self.stack.pop()
                self.stack.pop()
                self.stack.push_uint(1)
                return (30,)

            case (
                Op.ADDRESS
                | Op.ORIGIN
                | Op.CALLER
                | Op.COINBASE
                | Op.CALLVALUE
                | Op.TIMESTAMP
                | Op.NUMBER
                | Op.PREVRANDAO
                | Op.GASLIMIT
                | Op.CHAINID
                | Op.BASEFEE
                | Op.BLOBBASEFEE
                | Op.GASPRICE
            ):
                self.stack.push_uint(0)
                return (2,)

            case Op.BALANCE:
                self.stack.pop()
                self.stack.push_uint(0)
                return (100,)

            case Op.CALLDATALOAD:
                raws0 = self.stack.pop()
                offset = int.from_bytes(raws0.data, 'big', signed=False)
                self.stack.push(self.calldata.load(offset))
                return (3, raws0)

            case Op.CALLDATASIZE:
                self.stack.push_uint(len(self.calldata))
                return (2,)

            case Op.CALLDATACOPY:
                raws0 = self.stack.pop()
                mem_off = int.from_bytes(raws0.data, 'big', signed=False)

                raws1 = self.stack.pop()
                src_off = int.from_bytes(raws1.data, 'big', signed=False)

                size = self.stack.pop_uint()
                if size > 512:
                    raise UnsupportedOpError(op)
                value = self.calldata.load(src_off, size)
                self.memory.store(mem_off, value)
                return (4, raws1, raws0)  # src_off first, like in calldataload

            case Op.CODESIZE:
                self.stack.push_uint(len(self.code))
                return (2,)

            case Op.CODECOPY:
                mem_off = self.stack.pop_uint()
                src_off = self.stack.pop_uint()
                size = self.stack.pop_uint()
                if size > 32768:
                    raise UnsupportedOpError(op)
                data = self.code[src_off : src_off + size].ljust(size, b'\x00')
                self.memory.store(mem_off, Element(data=data))
                return (3,)

            case Op.EXTCODESIZE | Op.EXTCODEHASH:
                self.stack.pop()
                self.stack.push_uint(1)
                return (100,)

            case Op.RETURNDATASIZE:
                self.stack.push_uint(1024)
                return (2,)

            case Op.RETURNDATACOPY:
                mem_off = self.stack.pop_uint()
                self.stack.pop()
                size = self.stack.pop_uint()
                if size > 1024:
                    raise UnsupportedOpError(op)
                data = b'\x00' * size
                self.memory.store(mem_off, Element(data, None))
                return (3,)

            case Op.BLOCKHASH:
                self.stack.pop()
                self.stack.push_uint(1)
                return (20,)

            case Op.SELFBALANCE:
                self.stack.push_uint(0)
                return (5,)

            case Op.POP:
                self.stack.pop()
                return (2,)

            case Op.MLOAD:
                offset = self.stack.pop_uint()
                val, used = self.memory.load(offset)
                self.stack.push(val)
                return (4, used)

            case Op.MSTORE:
                offset = self.stack.pop_uint()
                value = self.stack.pop()
                self.memory.store(offset, value)
                return (3,)

            case Op.MSTORE8:
                offset = self.stack.pop_uint()
                value = self.stack.pop()
                el = Element(data=value.data[31:32], label=value.label)
                self.memory.store(offset, el)
                return (3,)

            case Op.MSIZE:
                self.stack.push_uint(self.memory.size())
                return (2,)

            case Op.SLOAD:
                slot = self.stack.pop()
                self.stack.push_uint(0)
                return (100, slot)

            case Op.SSTORE:
                slot = self.stack.pop()
                sval = self.stack.pop()
                return (100, slot, sval)

            case Op.GAS:
                self.stack.push_uint(1_000_000)
                return (2,)

            case op if op >= Op.LOG0 and op <= Op.LOG4:
                n = op - Op.LOG0
                for _ in range(n + 2):
                    self.stack.pop()
                return (375 * (n + 1),)

            case Op.CREATE | Op.CREATE2:
                self.stack.pop()
                self.stack.pop()
                self.stack.pop()
                if op == Op.CREATE2:
                    self.stack.pop()
                self.stack.push_uint(0)
                return (32000,)

            case Op.CALL | Op.DELEGATECALL | Op.STATICCALL:
                self.stack.pop()
                p1 = self.stack.pop()
                p2 = self.stack.pop()
                self.stack.pop()
                self.stack.pop()
                self.stack.pop()

                if op == Op.CALL:
                    self.stack.pop()

                self.stack.push_uint(0)  # failure

                if op == Op.CALL:
                    return (100, p1, p2)
                return (100, p1)

            case Op.REVERT | Op.STOP | Op.RETURN | Op.SELFDESTRUCT:
                # skip stack pop()s
                self.stopped = True
                return (5,)

            case _:
                raise UnsupportedOpError(op)
