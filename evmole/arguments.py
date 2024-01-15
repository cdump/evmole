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


def function_arguments(code: bytes | str, selector: bytes | str, gas_limit: int = int(1e4)) -> str:
    bytes_selector = to_bytes(selector)
    vm = Vm(code=to_bytes(code), calldata=Element(data=bytes_selector, label='calldata'))
    gas_used = 0
    inside_function = False
    args: dict[int, str] = {}
    while not vm.stopped:
        try:
            ret = vm.step()
            gas_used += ret[1]
            if gas_used > gas_limit:
                # raise Exception(f'gas overflow: {gas_used} > {gas_limit}')
                break

            if inside_function:
                # print(vm, '\n')
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
                vm.stack.push_uint(8192)

            case (Op.CALLDATALOAD, _, Element(Arg() as arg)):
                args[arg.offset] = 'bytes'
                vm.stack.pop()
                vm.stack.push(Element(data=(1).to_bytes(32, 'big'), label=ArgDynamicLength(offset=arg.offset)))

            case (Op.CALLDATALOAD, _, Element(ArgDynamic() as arg)):
                vm.stack.peek().label = Arg(offset=arg.offset, dynamic=True)

            case (Op.CALLDATALOAD, _, Element() as offset):
                off = int.from_bytes(offset.data, 'big')
                if off >= 4 and off < 2**32:
                    vm.stack.peek().label = Arg(offset=off)
                    args[off] = ''

            case (Op.ADD, _, Element(Arg() as arg), Element() as ot) | (Op.ADD, _, Element() as ot, Element(Arg() as arg)):
                vm.stack.peek().label = (
                    Arg(offset=arg.offset) if int.from_bytes(ot.data, 'big') == 4 else ArgDynamic(offset=arg.offset)
                )

            case (Op.ADD, _, Element(ArgDynamic() as arg), _) | (Op.ADD, _, _, Element(ArgDynamic() as arg)):
                vm.stack.peek().label = ArgDynamic(offset=arg.offset)

            case (Op.SHL, _, Element() as ot, Element(ArgDynamicLength() as arg)):
                if int.from_bytes(ot.data, 'big') == 5:
                    args[arg.offset] = 'uint256[]'

            case (
                (Op.MUL, _, Element(ArgDynamicLength() as arg), Element() as ot)
                | (Op.MUL, _, Element() as ot, Element(ArgDynamicLength() as arg))
            ):
                if int.from_bytes(ot.data, 'big') == 32:
                    args[arg.offset] = 'uint256[]'

            case (Op.AND, _, Element(Arg() as arg), Element() as ot) | (Op.AND, _, Element() as ot, Element(Arg() as arg)):
                v = int.from_bytes(ot.data, 'big')
                if v == 0:
                    pass
                elif (v & (v + 1)) == 0:
                    # 0x0000ffff
                    bl = v.bit_length()
                    if bl % 8 == 0:
                        t = 'address' if bl == 160 else f'uint{bl}'
                        args[arg.offset] = f'{t}[]' if arg.dynamic else t
                else:
                    # 0xffff0000
                    v = int.from_bytes(ot.data, 'little')
                    if (v & (v + 1)) == 0:
                        bl = v.bit_length()
                        if bl % 8 == 0:
                            t = f'bytes{bl // 8}'
                            args[arg.offset] = f'{t}[]' if arg.dynamic else t

            case (Op.ISZERO, _, Element(Arg() as arg)):
                vm.stack.peek().label = IsZeroResult(offset=arg.offset, dynamic=arg.dynamic)

            case (Op.ISZERO, _, Element(IsZeroResult() as arg)):
                args[arg.offset] = 'bool[]' if arg.dynamic else 'bool'

            case (Op.SIGNEXTEND, _, s0, Element(Arg() as arg)):
                if s0 < 32:
                    t = f'int{(s0+1)*8}'
                    args[arg.offset] = f'{t}[]' if arg.dynamic else t

            case (Op.BYTE, _, _, Element(Arg() as arg)):
                if args[arg.offset] == '':
                    args[arg.offset] = 'bytes32'

            # case (Op.LT, _, CallDataArgument() as arg, _):
            #     args[arg.offset] = 'uint8' # enum

    return ','.join(v[1] if v[1] != '' else 'uint256' for v in sorted(args.items()))
