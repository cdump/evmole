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

class ArgsResult {
  constructor() {
    this.args = {}
  }
  set(offset, atype) {
    this.args[offset] = atype
  }

  setIf(offset, if_val, atype) {
    const v = this.args[offset]
    if (v !== undefined) {
      if (v === if_val) {
        this.args[offset] = atype
      }
    } else if (atype === '') {
      this.args[offset] = atype
    }
  }

  joinToString() {
    const collator = new Intl.Collator([], { numeric: true })
    return Object.entries(this.args)
      .sort((a, b) => collator.compare(a, b))
      .map((v) => (v[1] !== '' ? v[1] : 'uint256'))
      .join(',')
  }
}

export function functionArguments(code, selector, gas_limit = 1e4) {
  const code_arr = toUint8Array(code)
  const selector_arr = toUint8Array(selector)
  const vm = new Vm(code_arr, new Element(selector_arr, 'calldata'))

  let gas_used = 0
  let inside_function = false
  let args = new ArgsResult()

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
        vm.stack.push_uint(131072n)
        break

      case Op.CALLDATALOAD:
        {
          const v = ret[2]
          if (v.label instanceof Arg) {
            args.set(v.label.offset, 'bytes')
            vm.stack.pop()
            vm.stack.push(new Element(bigIntToUint8Array(1n), new ArgDynamicLength(v.label.offset)))
          } else if (v.label instanceof ArgDynamic) {
            vm.stack.pop()
            vm.stack.push(new Element(bigIntToUint8Array(0n), new Arg(v.label.offset, true)))
          } else {
            const off = uint8ArrayToBigInt(v.data)
            if (off >= 4n && off < 131072n - 1024n) {
              vm.stack.pop()
              vm.stack.push(new Element(bigIntToUint8Array(0n), new Arg(Number(off))))

              args.setIf(off, '', '')
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
          if (arg instanceof ArgDynamicLength) {
            if (r2 === 5n) {
              args.set(arg.offset, 'uint256[]')
            } else if (r2 === 1n) {
              args.set(arg.offset, 'string')
            }
          }
        }
        break

      case Op.MUL:
        {
          if (ret[2].label instanceof ArgDynamicLength) {
            const n = uint8ArrayToBigInt(ret[3].data);
            if (n === 32n) {
              args.set(ret[2].label.offset, 'uint256[]')
            } else if (n === 2n) {
              args.set(ret[2].label.offset, 'string')
            }
          } else if (ret[3].label instanceof ArgDynamicLength) {
            const n = uint8ArrayToBigInt(ret[2].data);
            if (n === 32n) {
              args.set(ret[3].label.offset, 'uint256[]')
            } else if (n === 2n) {
              args.set(ret[3].label.offset, 'string')
            }
          } else if (ret[2].label instanceof Arg) {
            args.setIf(ret[2].label.offset, 'bool', '')
          } else if (ret[3].label instanceof Arg) {
            args.setIf(ret[3].label.offset, 'bool', '')
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
                args.set(arg.offset, arg.dynamic ? `${t}[]` : t)
              }
            } else {
              // 0xffff0000
              const v = BigInt(uint8ArrayToBigInt(ot.slice().reverse()))
              if ((v & (v + 1n)) === 0n) {
                const bl = bigIntBitLength(v)
                if (bl % 8 == 0) {
                  const t = `bytes${bl / 8}`
                  args.set(arg.offset, arg.dynamic ? `${t}[]` : t)
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
            // Detect check for 0 in DIV, it's not bool in that case: ISZERO, ISZERO, PUSH off, JUMPI, JUMPDEST, DIV
            let is_bool = true
            const op = vm.code[vm.pc]
            if (op >= Op.PUSH1 && op <= Op.PUSH4) {
              const n = op - Op.PUSH0
              if (vm.code[vm.pc + n + 1] === Op.JUMPI) {
                const jumpdest = vm.code.subarray(vm.pc + 1, vm.pc + 1 + n).reduce((acc, b) => acc * 256 + b, 0)
                if (jumpdest + 1 < vm.code.length && vm.code[jumpdest] === Op.JUMPDEST && vm.code[jumpdest + 1] === Op.DIV) {
                  is_bool = false
                }
              }
            }
            if (is_bool) {
              args.set(v.offset, v.dynamic ? 'bool[]' : 'bool')
            }
          }
        }
        break

      case Op.SIGNEXTEND:
        {
          const v = ret[3].label
          if (v instanceof Arg && ret[2] < 32n) {
            const t = `int${(Number(ret[2]) + 1) * 8}`
            args.set(v.offset, v.dynamic ? `${t}[]` : t)
          }
        }
        break

      case Op.BYTE:
        {
          const v = ret[3].label
          if (v instanceof Arg) {
            args.setIf(v.offset, '', 'bytes32')
          }
        }
        break
    }
  }

  return args.joinToString()
}
