import copy

from .evm.opcodes import Op
from .evm.stack import StackIndexError
from .evm.vm import CallData, UnsupportedOpError, Vm
from .utils import to_bytes


class Signature(bytes):
    pass


def process(vm: Vm, gas_limit: int) -> tuple[list[bytes], int]:
    selectors = []
    gas_used = 0

    while not vm.stopped:
        # print(vm, '\n')
        try:
            ret = vm.step()
            gas_used += ret[1]
            if gas_used > gas_limit:
                # raise Exception(f'gas overflow: {gas_used} > {gas_limit}')
                break
        except (StackIndexError, UnsupportedOpError):
            break

        match ret:
            # fmt: off
            case ((Op.XOR | Op.EQ as op, _, bytes() as s1, Signature())) | \
                 ((Op.XOR | Op.EQ as op, _, Signature(), bytes() as s1)
                ):
            #fmt: on
                selectors.append(s1[-4:])
                vm.stack.pop()
                vm.stack.push_uint(1 if op == Op.XOR else 0)

            # fmt: off
            case (Op.SUB, _, Signature(), bytes() as s1) | \
                 (Op.SUB, _, bytes() as s1, Signature()
                ):
            #fmt: on
                selectors.append(s1[-4:])

            case (Op.LT | Op.GT, _, Signature(), _) | (Op.LT | Op.GT, _, _, Signature()):
                cloned_vm = copy.copy(vm)
                s, g = process(cloned_vm, (gas_limit - gas_used) // 2)
                selectors += s
                gas_used += g
                v = vm.stack.pop_uint()
                vm.stack.push_uint(1 if v == 0 else 0)

            # fmt: off
            case (Op.SHR, _, _, CallData())  | \
                 (Op.AND, _, Signature(), _) | \
                 (Op.AND, _, _, Signature()) | \
                 (Op.DIV, _, CallData(), _
                ):
            # fmt: on
                v = vm.stack.peek()
                if v[-4:] == vm.calldata[:4]:
                    v = vm.stack.pop()
                    vm.stack.push(Signature(v))

            case (Op.AND, _, CallData(), _) | (Op.AND, _, _, CallData()):
                v = vm.stack.pop()
                vm.stack.push(CallData(v))

            case (Op.ISZERO, _, Signature()):
                selectors.append(b'\x00\x00\x00\x00')

            case (Op.MLOAD, _, set() as used):
                for u in used:
                    if isinstance(u, CallData):
                        v = vm.stack.pop()
                        if v[-4:] == vm.calldata[:4]:
                            vm.stack.push(Signature(v))
                        else:
                            vm.stack.push(CallData(v))
                        break

    return selectors, gas_used


def function_selectors(code: bytes | str, gas_limit: int = int(5e5)) -> list[str]:
    vm = Vm(code=to_bytes(code), calldata=CallData(b'\xaa\xbb\xcc\xdd'))
    selectors, _ = process(vm, gas_limit)
    return [s.hex() for s in selectors]
