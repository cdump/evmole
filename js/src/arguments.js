import Op from './evm/opcodes.js'
import { CallData, Vm, UnsupportedOpError } from './evm/vm.js'
import { StackIndexError } from './evm/stack.js'
import {
  hexToUint8Array,
  bigIntToUint8Array,
  uint8ArrayToBigInt,
  bigIntBitLength,
} from './utils.js'

class Arg extends Uint8Array {
  constructor(offset, dynamic = false, val) {
    const v = super(val !== undefined ? val : new Uint8Array(32))
    v.offset = offset
    v.dynamic = dynamic
    return v
  }
}

class ArgDynamicLength extends Uint8Array {
  constructor(offset) {
    const v = super(bigIntToUint8Array(1n))
    v.offset = offset
    return v
  }
}

class ArgDynamic extends Uint8Array {
  constructor(offset, val) {
    const v = super(val)
    v.offset = offset
    return v
  }
}

class IsZeroResult extends Uint8Array {
  constructor(offset, dynamic, val) {
    const v = super(val)
    v.offset = offset
    v.dynamic = dynamic
    return v
  }
}

export function functionArguments(
  code_hex_string,
  selector_hex_string,
  gas_limit = 1e4,
) {
  const code = hexToUint8Array(code_hex_string)
  const selector = hexToUint8Array(selector_hex_string)
  const vm = new Vm(code, new CallData(selector))

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
        const p = vm.stack.peek()[31]
        if (p === (op === Op.EQ ? 1 : 0)) {
          const a = ret[2].slice(-4)
          inside_function = selector.every((v, i) => v === a[i])
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
          const arg = ret[2]
          if (arg instanceof Arg) {
            args[arg.offset] = 'bytes'
            vm.stack.pop()
            vm.stack.push(new ArgDynamicLength(arg.offset))
          } else if (arg instanceof ArgDynamic) {
            vm.stack.pop()
            vm.stack.push(new Arg(arg.offset, true))
          } else {
            const off = uint8ArrayToBigInt(arg)
            if (off >= 4n) {
              vm.stack.pop()
              vm.stack.push(new Arg(Number(off)))
              args[off] = ''
            }
          }
        }
        break

      case Op.ADD:
        {
          const [r2, r3] = [ret[2], ret[3]]
          if (r2 instanceof Arg || r3 instanceof Arg) {
            const [arg, ot] = r2 instanceof Arg ? [r2, r3] : [r3, r2]
            const v = vm.stack.pop()
            if (uint8ArrayToBigInt(ot) === 4n) {
              vm.stack.push(new Arg(arg.offset, false, v))
            } else {
              vm.stack.push(new ArgDynamic(arg.offset, v))
            }
          } else if (r2 instanceof ArgDynamic || r3 instanceof ArgDynamic) {
            const v = vm.stack.pop()
            const arg = r2 instanceof ArgDynamic ? r2 : r3
            vm.stack.push(new ArgDynamic(arg.offset, v))
          }
        }
        break

      case Op.SHL:
        {
          const [r2, arg] = [uint8ArrayToBigInt(ret[2]), ret[3]]
          if (r2 == 5n && arg instanceof ArgDynamicLength) {
            args[arg.offset] = 'uint256[]'
          }
        }
        break

      case Op.MUL:
        {
          if (
            ret[3] instanceof ArgDynamicLength &&
            uint8ArrayToBigInt(ret[2]) == 32n
          ) {
            args[ret[3].offset] = 'uint256[]'
          }

          if (
            ret[2] instanceof ArgDynamicLength &&
            uint8ArrayToBigInt(ret[3]) == 32n
          ) {
            args[ret[2].offset] = 'uint256[]'
          }
        }
        break

      case Op.AND:
        {
          const [r2, r3] = [ret[2], ret[3]]
          if (r2 instanceof Arg || r3 instanceof Arg) {
            const [arg, ot] = r2 instanceof Arg ? [r2, r3] : [r3, r2]

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
          const arg = ret[2]
          if (arg instanceof Arg) {
            const v = vm.stack.pop()
            vm.stack.push(new IsZeroResult(arg.offset, arg.dynamic, v))
          } else if (arg instanceof IsZeroResult) {
            args[arg.offset] = arg.dynamic ? 'bool[]' : 'bool'
          }
        }
        break

      case Op.SIGNEXTEND:
        {
          const arg = ret[3]
          if (arg instanceof Arg) {
            const t = `int${(Number(ret[2]) + 1) * 8}`
            args[arg.offset] = arg.dynamic ? `${t}[]` : t
          }
        }
        break

      case Op.BYTE:
        {
          const arg = ret[3]
          if (arg instanceof Arg) {
            if (args[arg.offset] === '') {
              args[arg.offset] = 'bytes32'
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
