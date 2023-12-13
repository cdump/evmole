import copy

from .evm.vm import Vm
from .evm.opcodes import Op
from .utils import to_bytes


class CallData(bytes):
    def load(self, offset: int, size: int = 32):
        val = self[offset : min(offset + size, len(self))]
        return CallData(val.ljust(size, b'\x00'))


class CallDataSignature(bytes):
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
                raise Exception(f'gas overflow: {gas_used} > {gas_limit}')
        except Exception as ex:
            _ = ex
            # print(ex)
            # raise ex
            break

        match ret:
            # fmt: off
            case ((Op.XOR | Op.EQ as op, _, bytes() as s1, CallDataSignature())) | \
                 ((Op.XOR | Op.EQ as op, _, CallDataSignature(), bytes() as s1)
                ):
            #fmt: on
                selectors.append(s1[-4:])
                vm.stack.pop()
                vm.stack.push_uint(1 if op == Op.XOR else 0)

            # fmt: off
            case (Op.SUB, _, CallDataSignature(), bytes() as s1) | \
                 (Op.SUB, _, bytes() as s1, CallDataSignature()
                ):
            #fmt: on
                selectors.append(s1[-4:])

            case (Op.LT | Op.GT, _, CallDataSignature(), _) | (Op.LT | Op.GT, _, _, CallDataSignature()):
                cloned_vm = copy.copy(vm)
                s, g = process(cloned_vm, gas_limit // 2)
                selectors += s
                gas_used += g
                v = vm.stack.pop_uint()
                vm.stack.push_uint(1 if v == 0 else 0)

            # fmt: off
            case (Op.SHR, _, _, CallDataSignature() | CallData()) | \
                 (Op.AND, _, CallDataSignature() | CallData(), _) | \
                 (Op.AND, _, _, CallDataSignature() | CallData()) | \
                 (Op.DIV, _, CallDataSignature() | CallData(), _
                ):
            # fmt: on
                v = vm.stack.peek()
                assert v is not None
                if v[-4:] == vm.calldata[:4]:
                    v = vm.stack.pop()
                    vm.stack.push(CallDataSignature(v))

            case (Op.ISZERO, _, CallDataSignature()):
                selectors.append(b'\x00\x00\x00\x00')

            case (Op.MLOAD, _, set() as used):
                for u in used:
                    if isinstance(u, CallData):
                        p = vm.stack.peek()
                        if p is not None and p[-4:] == vm.calldata[:4]:
                            v = vm.stack.pop()
                            vm.stack.push(CallDataSignature(v))
                            break

    return selectors, gas_used


def function_selectors(code: bytes | str, gas_limit: int = int(5e5)) -> list[str]:
    # we don't need this OPs for function selector extraction, so blacklist them to exit the vm loop early
    blacklisted_ops = set([Op.NOT, Op.SHL, Op.MUL])
    vm = Vm(code=to_bytes(code), calldata=CallData(b'\xaa\xbb\xcc\xdd'), blacklisted_ops=blacklisted_ops)
    selectors, _ = process(vm, gas_limit)
    return [s.hex().zfill(8) for s in selectors]
