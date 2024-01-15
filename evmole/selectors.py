import copy

from .evm.element import Element
from .evm.opcodes import Op
from .evm.stack import StackIndexError
from .evm.vm import UnsupportedOpError, Vm
from .utils import to_bytes


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
            case (
                ((Op.XOR | Op.EQ as op, _, Element() as s1, Element('signature')))
                | ((Op.XOR | Op.EQ as op, _, Element('signature'), Element() as s1))
            ):
                selectors.append(s1.data[-4:])
                vm.stack.pop()
                vm.stack.push_uint(1 if op == Op.XOR else 0)

            case (Op.SUB, _, Element('signature'), Element() as s1) | (Op.SUB, _, Element() as s1, Element('signature')):
                selectors.append(s1.data[-4:])

            case (Op.LT | Op.GT, _, Element('signature'), _) | (Op.LT | Op.GT, _, _, Element('signature')):
                cloned_vm = copy.copy(vm)
                s, g = process(cloned_vm, (gas_limit - gas_used) // 2)
                selectors += s
                gas_used += g
                v = vm.stack.pop_uint()
                vm.stack.push_uint(1 if v == 0 else 0)

            case (
                (Op.SHR, _, _, Element('calldata'))
                | (Op.AND, _, Element('signature'), _)
                | (Op.AND, _, _, Element('signature'))
                | (Op.DIV, _, Element('calldata'), _)
            ):
                v = vm.stack.peek()
                if v.data[-4:] == vm.calldata.data[:4]:
                    vm.stack.peek().label = 'signature'

            case (Op.AND, _, Element('calldata'), _) | (Op.AND, _, _, Element('calldata')):
                vm.stack.peek().label = 'calldata'

            case (Op.ISZERO, _, Element('signature')):
                selectors.append(b'\x00\x00\x00\x00')

            case (Op.MLOAD, _, set() as used):
                if 'calldata' in used:
                    v = vm.stack.peek()
                    v.label = 'signature' if v.data[-4:] == vm.calldata.data[:4] else 'calldata'

    return selectors, gas_used


def function_selectors(code: bytes | str, gas_limit: int = int(5e5)) -> list[str]:
    vm = Vm(code=to_bytes(code), calldata=Element(data=b'\xaa\xbb\xcc\xdd', label='calldata'))
    selectors, _ = process(vm, gas_limit)
    return [s.hex() for s in selectors]
