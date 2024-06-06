from dataclasses import dataclass

from .evm.element import Element
from .evm.opcodes import Op
from .evm.stack import StackIndexError
from .evm.vm import UnsupportedOpError, Vm
from .utils import to_bytes


@dataclass(frozen=True)
class Arg:
    offset: int
    dynamic: bool = False


@dataclass(frozen=True)
class ArgDynamic:
    offset: int


@dataclass(frozen=True)
class ArgDynamicLength:
    offset: int


@dataclass(frozen=True)
class IsZeroResult:
    offset: int
    dynamic: bool = False


class ArgsResult:
    args: dict[int, str]

    def __init__(self):
        self.args = {}

    def set(self, offset: int, atype: str):
        self.args[offset] = atype

    def set_if(self, offset: int, if_val: str, atype: str):
        v = self.args.get(offset)
        if v is not None:
            if v == if_val:
                self.args[offset] = atype
        elif atype == '':
            self.args[offset] = atype

    def join_to_string(self) -> str:
        return ','.join(v[1] if v[1] != '' else 'uint256' for v in sorted(self.args.items()))


def function_arguments(code: bytes | str, selector: bytes | str, gas_limit: int = int(1e4)) -> str:
    bytes_selector = to_bytes(selector)
    vm = Vm(code=to_bytes(code), calldata=Element(data=bytes_selector, label='calldata'))
    gas_used = 0
    inside_function = False
    args = ArgsResult()
    while not vm.stopped:
        try:
            ret = vm.step()
            gas_used += ret[1]
            if gas_used > gas_limit:
                # raise Exception(f'gas overflow: {gas_used} > {gas_limit}')
                break

            if inside_function:
                # print(vm, '\n', sep='')
                # print(ret)
                pass
        except (StackIndexError, UnsupportedOpError) as ex:
            _ = ex
            # print(ex)
            break

        if inside_function is False:
            if ret[0] in {Op.EQ, Op.XOR, Op.SUB}:
                p = int.from_bytes(vm.stack.peek().data, 'big')
                if p == (1 if ret[0] == Op.EQ else 0):
                    inside_function = ret[2].data.endswith(bytes_selector)
            continue

        match ret:
            case (Op.CALLDATASIZE, _):
                vm.stack.pop()
                vm.stack.push_uint(131072)

            case (Op.CALLDATALOAD, _, Element(Arg() as arg)):
                args.set(arg.offset, 'bytes')
                vm.stack.pop()
                vm.stack.push(Element(data=(1).to_bytes(32, 'big'), label=ArgDynamicLength(offset=arg.offset)))

            case (Op.CALLDATALOAD, _, Element(ArgDynamic() as arg)):
                vm.stack.pop()
                vm.stack.push(Element(data=(0).to_bytes(32, 'big'), label=Arg(offset=arg.offset, dynamic=True)))

            case (Op.CALLDATALOAD, _, Element() as offset):
                off = int.from_bytes(offset.data, 'big')
                if off >= 4 and off < (131072 - 1024):
                    vm.stack.pop()
                    vm.stack.push(Element(data=(0).to_bytes(32, 'big'), label=Arg(offset=off)))
                    args.set_if(off, '', '')

            case (Op.MUL, _, Element(Arg() as arg), Element()) | (Op.MUL, _, Element(), Element(Arg() as arg)):
                args.set_if(arg.offset, 'bool', '')

            case (Op.ADD, _, Element(Arg() as arg), Element() as ot) | (Op.ADD, _, Element() as ot, Element(Arg() as arg)):
                vm.stack.peek().label = (
                    Arg(offset=arg.offset) if int.from_bytes(ot.data, 'big') == 4 else ArgDynamic(offset=arg.offset)
                )

            case (Op.ADD, _, Element(ArgDynamic() as arg), _) | (Op.ADD, _, _, Element(ArgDynamic() as arg)):
                vm.stack.peek().label = ArgDynamic(offset=arg.offset)

            case (Op.SHL, _, Element() as ot, Element(ArgDynamicLength() as arg)):
                v = int.from_bytes(ot.data, 'big')
                if v == 5:
                    args.set(arg.offset, 'uint256[]')
                elif v == 1:
                    args.set(arg.offset, 'string')

            case (
                (Op.MUL, _, Element(ArgDynamicLength() as arg), Element() as ot)
                | (Op.MUL, _, Element() as ot, Element(ArgDynamicLength() as arg))
            ):
                v = int.from_bytes(ot.data, 'big')
                if v == 32:
                    args.set(arg.offset, 'uint256[]')
                elif v == 2:
                    args.set(arg.offset, 'string')

            case (Op.AND, _, Element(Arg() as arg), Element() as ot) | (Op.AND, _, Element() as ot, Element(Arg() as arg)):
                v = int.from_bytes(ot.data, 'big')
                if v == 0:
                    pass
                elif (v & (v + 1)) == 0:
                    # 0x0000ffff
                    bl = v.bit_length()
                    if bl % 8 == 0:
                        t = 'address' if bl == 160 else f'uint{bl}'
                        args.set(arg.offset, f'{t}[]' if arg.dynamic else t)
                else:
                    # 0xffff0000
                    v = int.from_bytes(ot.data, 'little')
                    if (v & (v + 1)) == 0:
                        bl = v.bit_length()
                        if bl % 8 == 0:
                            t = f'bytes{bl // 8}'
                            args.set(arg.offset, f'{t}[]' if arg.dynamic else t)

            case (Op.ISZERO, _, Element(Arg() as arg)):
                vm.stack.peek().label = IsZeroResult(offset=arg.offset, dynamic=arg.dynamic)

            case (Op.ISZERO, _, Element(IsZeroResult() as arg)):
                # Detect check for 0 in DIV, it's not bool in that case: ISZERO, ISZERO, PUSH off, JUMPI, JUMPDEST, DIV
                is_bool = True
                op = vm.code[vm.pc]
                if op >= Op.PUSH1 and op <= Op.PUSH4:
                    n = op - Op.PUSH0
                    if vm.code[vm.pc + n + 1] == Op.JUMPI:
                        jumpdest = int.from_bytes(vm.code[(vm.pc + 1) : (vm.pc + 1 + n)], signed=False)
                        if jumpdest + 1 < len(vm.code) and vm.code[jumpdest] == Op.JUMPDEST and vm.code[jumpdest + 1] == Op.DIV:
                            is_bool = False
                if is_bool:
                    args.set(arg.offset, 'bool[]' if arg.dynamic else 'bool')

            case (Op.SIGNEXTEND, _, s0, Element(Arg() as arg)):
                if s0 < 32:
                    t = f'int{(s0+1)*8}'
                    args.set(arg.offset, f'{t}[]' if arg.dynamic else t)

            case (Op.BYTE, _, _, Element(Arg() as arg)):
                args.set_if(arg.offset, '', 'bytes32')

            # case (Op.LT, _, CallDataArgument() as arg, _):
            #     args[arg.offset] = 'uint8' # enum

    return args.join_to_string()
