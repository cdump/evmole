import Op from './evm/opcodes.js'
import { Vm, UnsupportedOpError } from './evm/vm.js'
import { StackIndexError } from './evm/stack.js'
import Element from './evm/element.js'
import { bigIntToUint8Array, uint8ArrayToBigInt, bigIntBitLength, toUint8Array } from './utils.js'

class Arg {
  constructor(offset, dynamic = false) {
    this.offset = offset
    this.dynamic = dynamic
  }
  toString() {
    return `Arg(${this.offset},${this.dynamic})`
  }
}

class ArgDynamicLength {
  constructor(offset) {
    this.offset = offset
  }
  toString() {
    return `ArgDynamicLength(${this.offset})`
  }
}

class ArgDynamic {
  constructor(offset) {
    this.offset = offset
  }
  toString() {
    return `ArgDynamic(${this.offset})`
  }
}

class IsZeroResult {
  constructor(offset, dynamic) {
    this.offset = offset
    this.dynamic = dynamic
  }
  toString() {
    return `IsZeroResult(${this.offset},${this.dynamic})`
  }
}

export function functionArguments(code, selector, gas_limit = 1e4) {
  const code_arr = toUint8Array(code)
  const selector_arr = toUint8Array(selector)
  const vm = new Vm(code_arr, new Element(selector_arr, 'calldata'))

  let gas_used = 0
  let inside_function = false
  let args = {}

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

      if (inside_function) {
        // console.log(vm.toString())
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

    if (inside_function == false) {
      if (op === Op.EQ || op == Op.XOR || op == Op.SUB) {
        const p = vm.stack.peek().data[31]
        if (p === (op === Op.EQ ? 1 : 0)) {
          const a = ret[2].data.slice(-4)
          inside_function = selector_arr.every((v, i) => v === a[i])
        }
      }

      continue
    }

    switch (op) {
      case Op.CALLDATASIZE:
        vm.stack.pop()
        vm.stack.push_uint(8192n)
        break

      case Op.CALLDATALOAD:
        {
          const v = ret[2]
          if (v.label instanceof Arg) {
            args[v.label.offset] = 'bytes'
            vm.stack.pop()
            vm.stack.push(new Element(bigIntToUint8Array(1n), new ArgDynamicLength(v.label.offset)))
          } else if (v.label instanceof ArgDynamic) {
            vm.stack.peek().label = new Arg(v.label.offset, true)
          } else {
            const off = uint8ArrayToBigInt(v.data)
            if (off >= 4n && off < 2n ** 32n) {
              vm.stack.peek().label = new Arg(Number(off))
              args[off] = ''
            }
          }
        }
        break

      case Op.ADD:
        {
          const [r2, r3] = [ret[2], ret[3]]
          if (r2.label instanceof Arg || r3.label instanceof Arg) {
            const [v, ot] = r2.label instanceof Arg ? [r2.label, r3.data] : [r3.label, r2.data]

            const p = vm.stack.peek()
            if (uint8ArrayToBigInt(ot) === 4n) {
              p.label = new Arg(v.offset, false)
            } else {
              p.label = new ArgDynamic(v.offset)
            }
          } else if (r2.label instanceof ArgDynamic || r3.label instanceof ArgDynamic) {
            const arg = r2.label instanceof ArgDynamic ? r2.label : r3.label
            vm.stack.peek().label = new ArgDynamic(arg.offset)
          }
        }
        break

      case Op.SHL:
        {
          const [r2, arg] = [uint8ArrayToBigInt(ret[2].data), ret[3].label]
          if (r2 == 5n && arg instanceof ArgDynamicLength) {
            args[arg.offset] = 'uint256[]'
          }
        }
        break

      case Op.MUL:
        {
          if (ret[3].label instanceof ArgDynamicLength && uint8ArrayToBigInt(ret[2].data) == 32n) {
            args[ret[3].label.offset] = 'uint256[]'
          } else if (ret[2].label instanceof ArgDynamicLength && uint8ArrayToBigInt(ret[3].data) == 32n) {
            args[ret[2].label.offset] = 'uint256[]'
          }
        }
        break

      case Op.AND:
        {
          const [r2, r3] = [ret[2], ret[3]]
          if (r2.label instanceof Arg || r3.label instanceof Arg) {
            const [arg, ot] = r2.label instanceof Arg ? [r2.label, r3.data] : [r3.label, r2.data]

            const v = uint8ArrayToBigInt(ot)
            if (v === 0n) {
              // pass
            } else if ((v & (v + 1n)) === 0n) {
              // 0x0000ffff
              const bl = bigIntBitLength(v)
              if (bl % 8 === 0) {
                const t = bl === 160 ? 'address' : `uint${bl}`
                args[arg.offset] = arg.dynamic ? `${t}[]` : t
              }
            } else {
              // 0xffff0000
              const v = BigInt(uint8ArrayToBigInt(ot.slice().reverse()))
              if ((v & (v + 1n)) === 0n) {
                const bl = bigIntBitLength(v)
                if (bl % 8 == 0) {
                  const t = `bytes${bl / 8}`
                  args[arg.offset] = arg.dynamic ? `${t}[]` : t
                }
              }
            }
          }
        }
        break

      case Op.ISZERO:
        {
          const v = ret[2].label
          if (v instanceof Arg) {
            vm.stack.peek().label = new IsZeroResult(v.offset, v.dynamic)
          } else if (v instanceof IsZeroResult) {
            args[v.offset] = v.dynamic ? 'bool[]' : 'bool'
          }
        }
        break

      case Op.SIGNEXTEND:
        {
          const v = ret[3].label
          if (v instanceof Arg && ret[2] < 32n) {
            const t = `int${(Number(ret[2]) + 1) * 8}`
            args[v.offset] = v.dynamic ? `${t}[]` : t
          }
        }
        break

      case Op.BYTE:
        {
          const v = ret[3].label
          if (v instanceof Arg) {
            if (args[v.offset] === '') {
              args[v.offset] = 'bytes32'
            }
          }
        }
        break
    }
  }

  var collator = new Intl.Collator([], { numeric: true })
  return Object.entries(args)
    .sort((a, b) => collator.compare(a, b))
    .map((v) => (v[1] !== '' ? v[1] : 'uint256'))
    .join(',')
}
