import Op from './evm/opcodes.js'
import { CallData, Vm, UnsupportedOpError } from './evm/vm.js'
import { StackIndexError } from './evm/stack.js'
import { toUint8Array, uint8ArrayToBigInt } from './utils.js'

class Signature extends Uint8Array {}

function process(vm, gas_limit) {
  let selectors = []
  let gas_used = 0

  while (!vm.stopped) {
    // console.log(vm.toString());
    let ret
    try {
      ret = vm.step()
      gas_used += ret[1]
      if (gas_used > gas_limit) {
        // throw `gas overflow: ${gas_used} > ${gas_limit}`
        break
      }
    } catch (e) {
      if (e instanceof StackIndexError || e instanceof UnsupportedOpError) {
        // console.log(e)
        break
      } else {
        throw e
      }
    }
    const op = ret[0]

    switch (op) {
      case Op.EQ:
      case Op.XOR:
        if (ret[2] instanceof Signature) {
          selectors.push(uint8ArrayToBigInt(ret[3]))
          vm.stack.pop()
          vm.stack.push_uint(op == Op.XOR ? 1n : 0n)
        } else if (ret[3] instanceof Signature) {
          selectors.push(uint8ArrayToBigInt(ret[2]))
          vm.stack.pop()
          vm.stack.push_uint(op == Op.XOR ? 1n : 0n)
        }
        break

      case Op.SUB:
        if (ret[2] instanceof Signature) {
          selectors.push(uint8ArrayToBigInt(ret[3]))
        } else if (ret[3] instanceof Signature) {
          selectors.push(uint8ArrayToBigInt(ret[2]))
        }
        break

      case Op.LT:
      case Op.GT:
        if (ret[2] instanceof Signature || ret[3] instanceof Signature) {
          const cloned_vm = vm.clone()
          const [s, gas] = process(cloned_vm, Math.trunc((gas_limit - gas_used) / 2))
          selectors.push(...s)
          gas_used += gas
          const v = vm.stack.pop_uint()
          vm.stack.push_uint(v === 0n ? 1n : 0n)
        }
        break

      case Op.SHR:
        {
          if (ret[3] instanceof CallData) {
            if (vm.stack.peek().slice(-4).every((v, i) => v === vm.calldata[i])) {
              const v = vm.stack.pop()
              vm.stack.push(new Signature(v))
            }
          }
        }
        break

      case Op.AND:
        {
          if (ret[2] instanceof Signature || ret[3] instanceof Signature) {
            if (vm.stack.peek().slice(-4).every((v, i) => v === vm.calldata[i])) {
              const v = vm.stack.pop()
              vm.stack.push(new Signature(v))
            }
          } else if (ret[2] instanceof CallData || ret[3] instanceof CallData) {
              const v = vm.stack.pop()
              vm.stack.push(new CallData(v))
          }
        }
        break

      case Op.DIV:
        {
          if (ret[2] instanceof CallData) {
            if (vm.stack.peek().slice(-4).every((v, i) => v === vm.calldata[i])) {
              const v = vm.stack.pop()
              vm.stack.push(new Signature(v))
            }
          }
        }
        break

      case Op.ISZERO:
        if (ret[2] instanceof Signature) {
          selectors.push(0n)
        }
        break

      case Op.MLOAD:
        {
          const used = ret[2]
          for (const u of used) {
            if (u instanceof CallData) {
              const p = vm.stack.pop()
              if (p.slice(-4).every((v, i) => v === vm.calldata[i])) {
                vm.stack.push(new Signature(p))
              } else {
                vm.stack.push(new CallData(p))
              }
              break
            }
          }
        }
        break
    }
  }
  return [selectors, gas_used]
}

export function functionSelectors(code, gas_limit = 5e5) {
  const code_arr = toUint8Array(code)
  const vm = new Vm(code_arr, new CallData([0xaa, 0xbb, 0xcc, 0xdd]))
  const [selectors] = process(vm, gas_limit)
  return selectors.map((x) => x.toString(16).padStart(8, '0'))
}
