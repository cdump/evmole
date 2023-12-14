from .utils import to_bytes
from .evm.vm import Vm, UnsupportedOpError
from .evm.opcodes import Op

from .selectors import CallData


class CallDataArgument(bytes):
    offset: int
    dynamic: bool

    def __new__(cls, *, offset: int, dynamic: bool = False, val: bytes = b'\x00' * 32):
        v = super().__new__(cls, val)
        v.dynamic = dynamic
        v.offset = offset
        return v

    def __repr__(self):
        return f'arg({self.offset},{self.dynamic})'


class CallDataArgumentDynamicLength(bytes):
    offset: int

    def __new__(cls, *, offset: int):
        v = super().__new__(cls, (1).to_bytes(32, 'big'))
        v.offset = offset
        return v

    def __repr__(self):
        return f'dlen({self.offset})'


class CallDataArgumentDynamic(bytes):
    offset: int

    def __new__(cls, *, offset: int, val: bytes = b'\x00' * 32):
        v = super().__new__(cls, val)
        v.offset = offset
        return v

    def __repr__(self):
        return f'darg({self.offset})'


def function_arguments(code: bytes | str, selector: bytes | str, gas_limit: int = int(1e4)) -> str:
    bytes_selector = to_bytes(selector)
    vm = Vm(code=to_bytes(code), calldata=CallData(bytes_selector))
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
        except UnsupportedOpError:
            break

        if inside_function is False:
            if ret[0] in {Op.EQ, Op.XOR, Op.SUB}:
                p = int.from_bytes(vm.stack.peek(), 'big')
                if p == (1 if ret[0] == Op.EQ else 0):
                    inside_function = bytes(ret[2]).endswith(bytes_selector)
            continue

        # print(ret)
        match ret:
            case (Op.CALLDATASIZE, _):
                vm.stack.pop()
                vm.stack.push_uint(8192)

            case (Op.CALLDATALOAD, _, CallDataArgument() as arg):
                args[arg.offset] = 'bytes'
                vm.stack.pop()
                v = CallDataArgumentDynamicLength(offset=arg.offset)
                vm.stack.push(v)

            case (Op.CALLDATALOAD, _, CallDataArgumentDynamic() as arg):
                vm.stack.pop()
                v = CallDataArgument(offset=arg.offset, dynamic=True)
                vm.stack.push(v)

            case (Op.CALLDATALOAD, _, bytes() as offset):
                off = int.from_bytes(offset, 'big')
                if off >= 4:
                    vm.stack.pop()
                    vm.stack.push(CallDataArgument(offset=off))
                    args[off] = ''

            case (Op.ADD, _, CallDataArgument() as cd, bytes() as ot) | (Op.ADD, _, bytes() as ot, CallDataArgument() as cd):
                v = vm.stack.pop()
                if int.from_bytes(ot, 'big') == 4:
                    vm.stack.push(CallDataArgument(offset=cd.offset, val=v))
                else:
                    vm.stack.push(CallDataArgumentDynamic(offset=cd.offset))

            case (Op.ADD, _, CallDataArgumentDynamic() as cd, _) | (Op.ADD, _, _, CallDataArgumentDynamic() as cd):
                v = vm.stack.pop()
                v = CallDataArgumentDynamic(offset=cd.offset, val=v)
                vm.stack.push(v)

            case (Op.SHL, _, bytes() as ot, CallDataArgumentDynamicLength() as arg) if int.from_bytes(ot, 'big') == 5:
                args[arg.offset] = 'uint256[]'

            # fmt: off
            case (Op.MUL, _, CallDataArgumentDynamicLength() as arg, bytes() as ot) | \
                 (Op.MUL, _, bytes() as ot, CallDataArgumentDynamicLength() as arg) if int.from_bytes(ot, 'big') == 32:
            # fmt: on
                args[arg.offset] = 'uint256[]'

            case (Op.AND, _, CallDataArgument() as arg, bytes() as ot) | (Op.AND, _, bytes() as ot, CallDataArgument() as arg):
                # 0x0000ffff
                v = int.from_bytes(ot, 'big')
                if (v & (v + 1)) == 0:
                    bl = v.bit_length()
                    t = 'address' if bl == 160 else f'uint{bl}'
                    args[arg.offset] = f'{t}[]' if arg.dynamic else t
                else:
                    # 0xffff0000
                    v = int.from_bytes(ot, 'little')
                    if (v & (v + 1)) == 0:
                        bl = v.bit_length() // 8
                        t = f'bytes{bl}'
                        args[arg.offset] = f'{t}[]' if arg.dynamic else t

            case (Op.ISZERO, _, CallDataArgument() as arg):
                args[arg.offset] = 'bool[]' if arg.dynamic else 'bool'

            case (Op.SIGNEXTEND, _, s0, CallDataArgument() as arg):
                t = f'int{(s0+1)*8}'
                args[arg.offset] = f'{t}[]' if arg.dynamic else t

            case (Op.BYTE, _, _, CallDataArgument() as arg):
                if args[arg.offset] == '':
                    args[arg.offset] = 'bytes32'

            # case (Op.LT, _, CallDataArgument() as arg, _):
            #     args[arg.offset] = 'uint8' # enum

    return ','.join(v[1] if v[1] != '' else 'uint256' for v in sorted(args.items()))
