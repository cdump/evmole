import Op from './evm/opcodes.js'
import { Vm, UnsupportedOpError } from './evm/vm.js'
import { StackIndexError } from './evm/stack.js'
import Element from './evm/element.js'
import { toUint8Array, uint8ArrayToBigInt } from './utils.js'

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
        if (ret[2].label === 'signature') {
          selectors.push(uint8ArrayToBigInt(ret[3].data))
          vm.stack.pop()
          vm.stack.push_uint(op == Op.XOR ? 1n : 0n)
        } else if (ret[3].label === 'signature') {
          selectors.push(uint8ArrayToBigInt(ret[2].data))
          vm.stack.pop()
          vm.stack.push_uint(op == Op.XOR ? 1n : 0n)
        }
        break

      case Op.SUB:
        if (ret[2].label === 'signature') {
          selectors.push(uint8ArrayToBigInt(ret[3].data))
        } else if (ret[3].label === 'signature') {
          selectors.push(uint8ArrayToBigInt(ret[2].data))
        }
        break

      case Op.LT:
      case Op.GT:
        if (ret[2].label === 'signature' || ret[3].label === 'signature') {
          const cloned_vm = vm.clone()
          const [s, gas] = process(cloned_vm, Math.trunc((gas_limit - gas_used) / 2))
          selectors.push(...s)
          gas_used += gas
          const v = vm.stack.pop_uint()
          vm.stack.push_uint(v === 0n ? 1n : 0n)
        }
        break

      case Op.MUL:
        if (ret[2].label === 'signature' || ret[3].label === 'signature') {
          vm.stack.peek().label = 'mulsig'
        }
        break

      // Vyper _selector_section_dense()
      case Op.MOD:
        if (ret[2].label === 'mulsig' || ret[2].label === 'signature') {
          const raw_ma = uint8ArrayToBigInt(ret[3].data)
          if (raw_ma < 128n) {
            const ma = Number(raw_ma)
            vm.stack.pop()
            for (let m = 1; m < ma; m++) {
              const cloned_vm = vm.clone()
              cloned_vm.stack.push_uint(BigInt(m))
              const [s, gas] = process(cloned_vm, Math.trunc((gas_limit - gas_used) / ma))
              selectors.push(...s)
              gas_used += gas
              if (gas_used > gas_limit) {
                break
              }
            }
            vm.stack.push_uint(0n)
          }
        }
        break

      case Op.SHR:
        {
          if (ret[3].label === 'calldata') {
            if (
              vm.stack
                .peek()
                .data.slice(-4)
                .every((v, i) => v === vm.calldata.data[i])
            ) {
              vm.stack.peek().label = 'signature'
            }
          } else if (ret[3].label === 'mulsig') {
            vm.stack.peek().label = 'mulsig'
          }
        }
        break

      case Op.AND:
        {
          if (ret[2].label === 'signature' || ret[3].label === 'signature') {
            if (
              vm.stack
                .peek()
                .data.slice(-4)
                .every((v, i) => v === vm.calldata.data[i])
            ) {
              vm.stack.peek().label = 'signature'
            }
          } else if (ret[2].label === 'calldata' || ret[3].label === 'calldata') {
            vm.stack.peek().label = 'calldata'
          }
        }
        break

      case Op.DIV:
        {
          if (ret[2].label === 'calldata') {
            if (
              vm.stack
                .peek()
                .data.slice(-4)
                .every((v, i) => v === vm.calldata.data[i])
            ) {
              vm.stack.peek().label = 'signature'
            }
          }
        }
        break

      case Op.ISZERO:
        if (ret[2].label === 'signature') {
          selectors.push(0n)
        }
        break

      case Op.MLOAD:
        {
          const used = ret[2]
          const p = vm.stack.peek()
          if (used.has('calldata') && (p.data.slice(-4).every((v, i) => v === vm.calldata.data[i]))) {
            p.label = 'signature'
          }
        }
        break
    }
  }
  return [selectors, gas_used]
}

export function functionSelectors(code, gas_limit = 5e5) {
  const code_arr = toUint8Array(code)
  const vm = new Vm(code_arr, new Element(new Uint8Array([0xaa, 0xbb, 0xcc, 0xdd]), 'calldata'))
  const [selectors] = process(vm, gas_limit)
  return selectors.map((x) => x.toString(16).padStart(8, '0'))
}
