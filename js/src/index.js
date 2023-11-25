import Op from './evm/opcodes.js'
import Vm from './evm/vm.js'

import {
  hexToUint8Array,
  bigIntToUint8Array,
  uint8ArrayToBigInt,
} from './utils.js'

class CallData extends Uint8Array {
  load(offset, size = 32) {
    const v = new CallData(32)
    v.set(this.subarray(offset, offset + size))
    return v
  }
  toBigInt() {
    return uint8ArrayToBigInt(this)
  }
}

class CallDataSignature extends Uint8Array {
  toBigInt() {
    return uint8ArrayToBigInt(this)
  }
}

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
        throw `gas overflow: ${gas_used} > ${gas_limit}`
      }
    } catch (err) {
      // console.log(err);
      // throw err;
      break
    }
    const op = ret[0]

    if (op == Op.EQ || op == Op.XOR) {
      if (ret[2] instanceof CallDataSignature) {
        selectors.push(ret[3])
        vm.stack.pop()
        vm.stack.push(op == Op.XOR ? 1n : 0n)
      } else if (ret[3] instanceof CallDataSignature) {
        selectors.push(ret[2])
        vm.stack.pop()
        vm.stack.push(op == Op.XOR ? 1n : 0n)
      }
      continue
    }

    if (op == Op.SUB) {
      if (ret[2] instanceof CallDataSignature) {
        selectors.push(ret[3])
      } else if (ret[3] instanceof CallDataSignature) {
        selectors.push(ret[2])
      }
      continue
    }

    if (op == Op.LT || op == Op.GT) {
      const cloned_vm = vm.clone()
      const [s, gas] = process(cloned_vm, gas_limit / 2)
      selectors.push(...s)
      gas_used += gas
      const v = vm.stack.pop()
      vm.stack.push(v === 0n ? 1n : 0n)
      continue
    }

    if (op == Op.SHR || op == Op.AND || op == Op.DIV) {
      const x = vm.stack.peek()
      if (x === undefined) throw 'stack peek failed'
      if ((x & 0xffffffffn) == vm.calldata.toBigInt()) {
        const v = vm.stack.pop()
        vm.stack.push(new CallDataSignature(bigIntToUint8Array(v)))
      }
      continue
    }

    if (op == Op.ISZERO) {
      if (ret[2] instanceof CallDataSignature) {
        selectors.push(0n)
      }
      continue
    }

    if (op == Op.MLOAD) {
      const used = ret[2]
      for (const u of used) {
        if (u instanceof CallData) {
          const v = vm.stack.peek()
          if ((v & 0xffffffffn) == vm.calldata.toBigInt()) {
            vm.stack.push(
              new CallDataSignature(bigIntToUint8Array(vm.stack.pop())),
            )
            break
          }
        }
      }
      continue
    }
  }

  return [selectors, gas_used]
}

export function functionSelectors(code_hex_string, gas_limit = 1e6) {
  const code = hexToUint8Array(code_hex_string)
  const vm = new Vm(code, new CallData([0xaa, 0xbb, 0xcc, 0xdd]))
  const [selectors] = process(vm, gas_limit)
  return selectors.map((x) => x.toString(16).padStart(8, '0'))
}
