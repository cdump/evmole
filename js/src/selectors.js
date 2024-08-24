import Op from './evm/opcodes.js'
import { Vm, UnsupportedOpError } from './evm/vm.js'
import { StackIndexError } from './evm/stack.js'
import Element from './evm/element.js'
import { toUint8Array, uint8ArrayToBigInt } from './utils.js'

function process(vm, gasLimit) {
  let selectors = new Set()
  let gasUsed = 0

  while (!vm.stopped) {
    // console.log('selectors', selectors)
    // console.log(vm.toString())
    let ret
    try {
      ret = vm.step()
      gasUsed += ret[1]
      if (gasUsed > gasLimit) {
        // throw `gas overflow: ${gasUsed} > ${gasLimit}`
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

    const [op, , r0, r1] = ret

    switch (op) {
      case Op.XOR:
      case Op.EQ:
      case Op.SUB:
        if (r0.label === 'signature' || r1.label === 'signature') {
          selectors.add(uint8ArrayToBigInt(r0.label === 'signature' ? r1.data : r0.data))
          vm.stack.pop()
          vm.stack.push_uint(op === Op.EQ ? 0n : 1n)
        }
        break

      case Op.LT:
      case Op.GT:
        if (r0.label === 'signature' || r1.label === 'signature') {
          const clonedVm = vm.clone()
          const [newSelectors, gas] = process(clonedVm, Math.trunc((gasLimit - gasUsed) / 2))
          newSelectors.forEach((v) => selectors.add(v))
          gasUsed += gas
          const v = vm.stack.pop_uint()
          vm.stack.push_uint(v === 0n ? 1n : 0n)
        }
        break

      case Op.MUL:
        if (r0.label === 'signature' || r1.label === 'signature') {
          vm.stack.peek().label = 'mulsig'
        }
        break

      // Vyper _selector_section_dense()/_selector_section_sparse()
      // (sig MOD n_buckets) or (sig AND (n_buckets-1))
      case Op.MOD:
      case Op.AND:
        {
          const p = vm.stack.peek()
          if (
            (op === Op.AND && (r0.label === 'signature' || r1.label === 'signature')) ||
            (op === Op.MOD && (r0.label === 'mulsig' || r0.label === 'signature'))
          ) {
            const otd = op === Op.AND && r1.label == 'signature' ? r0.data : r1.data
            const rawMa = uint8ArrayToBigInt(otd)
            if (op === Op.AND && rawMa === 0xffffffffn) {
              p.label = 'signature'
            } else {
              if (rawMa < 256n) {
                const ma = Number(rawMa)
                vm.stack.pop()
                const to = op === Op.MOD ? ma : ma + 1
                for (let m = 1; m < to && gasUsed < gasLimit; m++) {
                  const clonedVm = vm.clone()
                  clonedVm.stack.push_uint(BigInt(m))
                  const [newSelectors, gas] = process(clonedVm, Math.trunc((gasLimit - gasUsed) / ma))
                  newSelectors.forEach((v) => selectors.add(v))
                  gasUsed += gas
                }
                vm.stack.push_uint(0n)
              }
            }
          } else if (op === Op.AND && (r0.label === 'calldata' || r1.label === 'calldata')) {
            p.label = 'calldata'
          }
        }
        break

      case Op.SHR:
        if (r1.label === 'calldata') {
          const p = vm.stack.peek()
          if (p.data.slice(-4).every((v, i) => v === vm.calldata.data[i])) {
            p.label = 'signature'
          } else {
            p.label = 'mulsig'
          }
        } else if (r1.label === 'mulsig') {
          const p = vm.stack.peek()
          p.label = 'mulsig'
        }
        break

      case Op.DIV:
        if (r0.label === 'calldata') {
          const p = vm.stack.peek()
          if (p.data.slice(-4).every((v, i) => v === vm.calldata.data[i])) {
            p.label = 'signature'
          }
        }
        break

      case Op.ISZERO:
        if (r0.label === 'signature') {
          selectors.add(0n)
        }
        break

      case Op.MLOAD:
        {
          const p = vm.stack.peek()
          if (r0.has('calldata') && p.data.slice(-4).every((v, i) => v === vm.calldata.data[i])) {
            p.label = 'signature'
          }
        }
        break
    }
  }
  return [selectors, gasUsed]
}

export function functionSelectors(code, gasLimit = 5e5) {
  const codeArr = toUint8Array(code)
  const vm = new Vm(codeArr, new Element(new Uint8Array([0xaa, 0xbb, 0xcc, 0xdd]), 'calldata'))
  const [selectors] = process(vm, gasLimit)
  return [...selectors.values()].map((x) => x.toString(16).padStart(8, '0'))
}
