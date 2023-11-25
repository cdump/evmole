import copy

from .evm.vm import Vm
from .evm.opcodes import Op


class CallData(bytes):
    def load(self, offset: int, size: int = 32):
        val = self[offset : min(offset + size, len(self))]
        return CallData(val + b'\x00' * max(0, size - len(val)))


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

            case (Op.LT | Op.GT, *_):
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

            case (Op.MLOAD, _, list() as used):
                for u in used:
                    if isinstance(u, CallData):
                        p = vm.stack.peek()
                        if p is not None and p[-4:] == vm.calldata[:4]:
                            v = vm.stack.pop()
                            vm.stack.push(CallDataSignature(v))
                            break

    return selectors, gas_used


def function_selectors(code: bytes | str, gas_limit: int = int(1e6)) -> list[str]:
    if isinstance(code, str):
        code = bytes.fromhex(code[2:] if code.startswith('0x') else code)
    else:
        assert isinstance(code, bytes), '`code` arg must be hex-string or bytes'
    vm = Vm(code=code, calldata=CallData(b'\xaa\xbb\xcc\xdd'))
    selectors, _ = process(vm, gas_limit)
    return [s.hex().zfill(8) for s in selectors]
