import copy

from .evm.element import Element
from .evm.opcodes import Op
from .evm.stack import StackIndexError
from .evm.vm import UnsupportedOpError, Vm
from .utils import to_bytes


def process(vm: Vm, gas_limit: int) -> tuple[set[bytes], int]:
    selectors: set[bytes] = set()
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
                (Op.XOR | Op.EQ | Op.SUB as op, _, Element() as s1, Element('signature'))
                | (Op.XOR | Op.EQ | Op.SUB as op, _, Element('signature'), Element() as s1)
            ):
                selectors.add(s1.data[-4:])
                vm.stack.pop()
                vm.stack.push_uint(0 if op == Op.EQ else 1)

            case (Op.LT | Op.GT, _, Element('signature'), _) | (Op.LT | Op.GT, _, _, Element('signature')):
                cloned_vm = copy.copy(vm)
                s, g = process(cloned_vm, (gas_limit - gas_used) // 2)
                selectors.update(s)
                gas_used += g
                v = vm.stack.pop_uint()
                vm.stack.push_uint(1 if v == 0 else 0)

            case (Op.MUL, _, Element('signature'), _) | (Op.MUL, _, _, Element('signature')) | (Op.SHR, _, _, Element('mulsig')):
                vm.stack.peek().label = 'mulsig'

            # Vyper _selector_section_dense()/_selector_section_sparse()
            # (sig MOD n_buckets) or (sig AND (n_buckets-1))
            case (
                (Op.MOD as op, _, Element('mulsig') | Element('signature'), Element() as s1)
                | (Op.AND as op, _, Element('signature'), Element() as s1)
                | (Op.AND as op, _, Element() as s1, Element('signature'))
            ):
                if op == Op.AND and s1.data == b'\x00' * 28 + b'\xff\xff\xff\xff':
                    vm.stack.peek().label = 'signature'
                else:
                    ma = int.from_bytes(s1.data, 'big')
                    if ma < 256:
                        to = ma if op == Op.MOD else ma + 1
                        for m in range(1, to):
                            cloned_vm = copy.copy(vm)
                            cloned_vm.stack.peek().data = m.to_bytes(32, 'big')
                            s, g = process(cloned_vm, (gas_limit - gas_used) // ma)
                            selectors.update(s)
                            gas_used += g
                            if gas_used > gas_limit:
                                break
                        vm.stack.peek().data = (0).to_bytes(32, 'big')

            case (Op.SHR, _, _, Element('calldata')) | (Op.DIV, _, Element('calldata'), _):
                v = vm.stack.peek()
                if v.data[-4:] == vm.calldata.data[:4]:
                    vm.stack.peek().label = 'signature'

            case (Op.AND, _, Element('calldata'), _) | (Op.AND, _, _, Element('calldata')):
                vm.stack.peek().label = 'calldata'

            case (Op.ISZERO, _, Element('signature')):
                selectors.add(b'\x00\x00\x00\x00')

            case (Op.MLOAD, _, set() as used):
                v = vm.stack.peek()
                if 'calldata' in used and v.data[-4:] == vm.calldata.data[:4]:
                    v.label = 'signature'

    return selectors, gas_used


def function_selectors(code: bytes | str, gas_limit: int = int(5e5)) -> list[str]:
    vm = Vm(code=to_bytes(code), calldata=Element(data=b'\xaa\xbb\xcc\xdd', label='calldata'))
    selectors, _ = process(vm, gas_limit)
    return [s.hex() for s in selectors]
