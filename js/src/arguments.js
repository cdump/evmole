import Op from './evm/opcodes.js'
import { Vm, UnsupportedOpError } from './evm/vm.js'
import { StackIndexError } from './evm/stack.js'
import Element from './evm/element.js'
import { bigIntToUint8Array, uint8ArrayToBigInt, bigIntBitLength, toUint8Array } from './utils.js'

class Arg {
  offset
  path
  add_val
  and_mask
  constructor(properties) {
    Object.preventExtensions(this)
    Object.assign(this, properties)
  }
  toString() {
    return `Arg(off=${this.offset},path=${this.path},add_val=${this.add_val},and_mask=${this.and_mask})`
  }
}

class IsZeroResult {
  offset
  path
  add_val
  and_mask
  constructor(properties) {
    Object.preventExtensions(this)
    Object.assign(this, properties)
  }
  toString() {
    return `IsZeroResult(off=${this.offset},path=${this.path},add_val=${this.add_val},and_mask=${this.and_mask})`
  }
}

class InfoValDynamic {
  n_elements
  constructor(n) {
    this.n_elements = n
  }
  toString() {
    return `Dynamic(${this.n_elements})`
  }
}

class InfoValArray {
  n_elements
  constructor(n) {
    this.n_elements = n
  }
  toString() {
    return `Array(${this.n_elements})`
  }
}

class Info {
  constructor() {
    this.tinfo = null
    this.tname = null
    this.children = new Map()
  }
  toStr(isRoot = false) {
    if (this.tname !== null) {
      const [name] = this.tname
      if (name === 'bytes') {
        if (
          this.tinfo === null ||
          (this.tinfo instanceof InfoValArray && this.tinfo.n_elements === 0) ||
          (this.tinfo instanceof InfoValDynamic && this.tinfo.n_elements === 1)
        ) {
          return name
        }
      } else if (this.children.size === 0) {
        if (this.tinfo === null || this.tinfo instanceof InfoValDynamic) {
          return name
        }
      }
    }
    let startKey = this.tinfo instanceof InfoValArray ? 32 : 0
    let endKey = this.children.size > 0 ? Math.max(...this.children.keys()) : 0

    if (this.tinfo instanceof InfoValArray || this.tinfo instanceof InfoValDynamic) {
      endKey = Math.max(endKey, this.tinfo.n_elements * 32)
    }

    const q = []
    for (let k = startKey; k <= endKey; k += 32) {
      q.push(this.children.has(k) ? this.children.get(k).toStr(false) : 'uint256')
    }

    let c = q.length > 1 && !isRoot ? `(${q.join(',')})` : q.join(',')

    if (this.tinfo instanceof InfoValArray) {
      return `${c}[]`
    }
    if (this.tinfo instanceof InfoValDynamic) {
      if (endKey === 0 && this.children.size === 0) {
        return 'bytes'
      }
      if (endKey === 32) {
        if (this.children.size === 0) {
          return 'uint256[]'
        }
        if (this.children.size === 1 && this.children.values().next().value.tinfo === null) {
          return `${q[1]}[]`
        }
      }
    }
    return c
  }
}

class ArgsResult {
  constructor() {
    this.data = new Info()
    this.notBool = new Set()
  }

  getOrCreate(path) {
    return path.reduce((node, key) => {
      if (!node.children.has(key)) {
        node.children.set(key, new Info())
      }
      return node.children.get(key)
    }, this.data)
  }

  get(path) {
    return path.reduce((node, key) => {
      return node?.children.get(key)
    }, this.data)
  }

  markNotBool(path, offset) {
    const fullPath = [...path, offset]
    const el = this.get(fullPath)
    if (el && el.tname && el.tname[0] === 'bool') {
      el.tname = null
    }
    this.notBool.add(fullPath.join(','))
  }

  setTname(path, offset, tname, confidence) {
    const fullPath = offset !== null ? [...path, offset] : path
    if (tname === 'bool' && this.notBool.has(fullPath.join(','))) {
      return
    }
    const el = this.getOrCreate(fullPath)
    if (el.tname !== null && confidence <= el.tname[1]) {
      return
    }
    el.tname = [tname, confidence]
  }

  arrayInPath(path) {
    let el = this.data
    return path.map((p) => {
      if (el === undefined) return false
      el = el.children.get(p)
      return el && el.tinfo instanceof InfoValArray
    })
  }

  setInfo(path, tinfo) {
    if (path.length === 0) {
      // root
      return
    }

    const el = this.getOrCreate(path)
    if (tinfo instanceof InfoValDynamic) {
      if (el.tinfo instanceof InfoValDynamic && el.tinfo.n_elements > tinfo.n_elements) {
        return
      }
      if (el.tinfo instanceof InfoValArray) {
        return
      }
    }
    if (el.tinfo instanceof InfoValArray && tinfo instanceof InfoValArray) {
      if (tinfo.n_elements < el.tinfo.n_elements) {
        return
      }
    }
    el.tinfo = tinfo
  }

  joinToString() {
    return this.data.children.size === 0 ? '' : this.data.toStr(true)
  }
}

function andMaskToType(mask) {
  if (mask === 0n) {
    return null
  }
  if ((mask & (mask + 1n)) === 0n) {
    // 0x0000ffff
    const bl = bigIntBitLength(mask)
    if (bl % 8 === 0) {
      return bl === 160 ? 'address' : `uint${bl}`
    }
  } else {
    // 0xffff0000
    const m = BigInt(uint8ArrayToBigInt(bigIntToUint8Array(mask).slice().reverse()))
    if ((m & (m + 1n)) === 0n) {
      const bl = bigIntBitLength(m)
      if (bl % 8 == 0) {
        return `bytes${bl / 8}`
      }
    }
  }
  return null
}

export function functionArguments(code, selector, gas_limit = 5e4) {
  const code_arr = toUint8Array(code)
  const selector_arr = toUint8Array(selector)
  const vm = new Vm(code_arr, new Element(selector_arr, 'calldata'))

  let gas_used = 0
  let inside_function = false
  let args = new ArgsResult()

  while (!vm.stopped) {
    let ret
    try {
      if (inside_function) {
        // console.log('args:', args.joinToString());
        // console.log('not_bool:', args.notBool);
        // console.dir(args.data, { depth: null, colors: true })
        // console.log(vm.toString())
      }
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

    const [op, , r0, r1] = ret

    if (inside_function == false) {
      if (op === Op.EQ || op == Op.XOR || op == Op.SUB) {
        const p = vm.stack.peek().data[31]
        if (p === (op === Op.EQ ? 1 : 0)) {
          const a = r0.data.slice(-4)
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
      case Op.CALLDATACOPY:
        {
          if (r0.label instanceof Arg) {
            const { offset, path, add_val } = r0.label
            if (add_val >= 4 && (add_val - 4) % 32 === 0) {
              let po = 0
              if (add_val != 4) {
                const a = args.arrayInPath(path).reduce((s, v) => s + 32 * v, 0)
                if (a <= add_val - 4) {
                  po = a
                }
              }

              const fullPath = [...path, offset]
              const new_off = add_val - 4 - po

              args.setInfo(fullPath, new InfoValDynamic(new_off / 32))

              if (new_off === 0 && args.arrayInPath(fullPath).pop() === true) {
                const d = bigIntToUint8Array(1n)
                if (op == Op.CALLDATALOAD) {
                  vm.stack.peek().data = d
                } else {
                  const mem_off = uint8ArrayToBigInt(r1.data)
                  vm.memory.get(mem_off).data = d
                }
              }

              const new_label = new Arg({ offset: new_off, path: fullPath, add_val: 0, and_mask: null })
              if (op == Op.CALLDATALOAD) {
                vm.stack.peek().label = new_label
              } else {
                const mem_off = uint8ArrayToBigInt(r1.data)
                vm.memory.get(mem_off).label = new_label
                args.setTname(path, offset, 'bytes', 10)
              }
            }
          } else {
            const off = uint8ArrayToBigInt(r0.data)
            if (off >= 4n && off < 131072n - 1024n) {
              // -1024: cut 'trustedForwarder'
              args.getOrCreate([Number(off) - 4])

              const new_label = new Arg({ offset: Number(off) - 4, path: [], add_val: 0, and_mask: null })
              if (op == Op.CALLDATALOAD) {
                vm.stack.peek().label = new_label
              } else {
                const mem_off = uint8ArrayToBigInt(r1.data)
                vm.memory.get(mem_off).label = new_label
              }
            }
          }
        }
        break

      case Op.ADD:
        {
          const [l0, l1] = [r0.label, r1.label]
          if (l0 instanceof Arg && l1 instanceof Arg) {
            args.markNotBool(l0.path, l0.offset)
            args.markNotBool(l1.path, l1.offset)

            vm.stack.peek().label =
              l0.path.length > l1.path.length
                ? new Arg({
                    offset: l0.offset,
                    path: l0.path,
                    add_val: l0.add_val + l1.add_val,
                    and_mask: l0.and_mask,
                  })
                : new Arg({
                    offset: l1.offset,
                    path: l1.path,
                    add_val: l0.add_val + l1.add_val,
                    and_mask: l1.and_mask,
                  })
          } else if (l0 instanceof Arg || l1 instanceof Arg) {
            const [r, otd] = l0 instanceof Arg ? [r0, r1.data] : [r1, r0.data]
            const rl = r.label

            args.markNotBool(rl.path, rl.offset)

            const ot_val = uint8ArrayToBigInt(otd)

            const E256M1 = (1n << 256n) - 1n

            if (
              rl.offset == 0 &&
              rl.add_val == 0 &&
              rl.path.length != 0 &&
              uint8ArrayToBigInt(r.data) === 0n &&
              ot_val == E256M1
            ) {
              vm.stack.peek().data = bigIntToUint8Array(0n)
            }
            const add = (ot_val + BigInt(rl.add_val)) & E256M1
            if (add < 1n << 32n) {
              vm.stack.peek().label = new Arg({ offset: rl.offset, path: rl.path, add_val: Number(add), and_mask: rl.and_mask })
            }
          }
        }
        break

      case Op.MUL:
      case Op.SHL:
        {
          const [l0, l1] = [r0.label, r1.label]

          if ((op === Op.MUL && (l0 instanceof Arg || l1 instanceof Arg)) || (op === Op.SHL && l1 instanceof Arg)) {
            const [rl, ot] = l1 instanceof Arg ? [l1, r0] : [l0, r1]

            args.markNotBool(rl.path, rl.offset)
            if (ot.label instanceof Arg) {
              args.markNotBool(ot.label.path, ot.label.offset)
            }
            if (rl.offset === 0 && rl.add_val === 0) {
              if (rl.path.length != 0) {
                let mult = uint8ArrayToBigInt(ot.data)
                if (op === Op.SHL) {
                  mult = 1n << mult
                }
                if (mult === 1n) {
                  args.setTname(rl.path, null, 'bytes', 10)
                } else if (mult == 2n) {
                  args.setTname(rl.path, null, 'string', 20)
                } else if (mult % 32n === 0n && 32n <= mult && mult <= 3200n) {
                  args.setInfo(rl.path, new InfoValArray(Number(mult / 32n)))

                  const shouldUpdate = (v) => v.offset == 0 && v.path == rl.path && v.add_val == 0

                  vm.stack.data.forEach((el) => {
                    if (el.label instanceof Arg && shouldUpdate(el.label)) {
                      el.data = bigIntToUint8Array(1n)
                    }
                  })

                  vm.memory.data.forEach((el) => {
                    if (el.label instanceof Arg && shouldUpdate(el.label[1])) {
                      el.data = bigIntToUint8Array(1n)
                    }
                  })

                  vm.stack.peek().data = ot.data // ==bigIntToUint8Array(mult)
                }
              }
            }
          }
        }
        break

      case Op.GT:
      case Op.LT:
        {
          const [rl, ot] = op === Op.LT ? [r1.label, r0] : [r0.label, r1]
          if (ot.label instanceof Arg) {
            args.markNotBool(ot.label.path, ot.label.offset)
          }
          if (rl instanceof Arg) {
            args.markNotBool(rl.path, rl.offset)
            if (rl.offset === 0 && rl.add_val === 0 && rl.and_mask === null) {
              // 0 < arr.len || arr.len > 0
              const v = uint8ArrayToBigInt(ot.data)
              if (v === 0n || v === 31n) {
                vm.stack.peek().data = bigIntToUint8Array(1n)
              }
            }
          }
        }
        break

      case Op.AND:
        {
          const [l0, l1] = [r0.label, r1.label]
          if (l0 instanceof Arg || l1 instanceof Arg) {
            const [rl, otd] = l0 instanceof Arg ? [l0, r1.data] : [l1, r0.data]

            const { path, offset, add_val } = rl
            args.markNotBool(path, offset)

            const mask = uint8ArrayToBigInt(otd)
            let t = andMaskToType(mask)
            if (t !== null) {
              args.setTname(path, offset, t, 5)
              vm.stack.peek().label = new Arg({ offset, path, add_val, and_mask: mask })
            }
          }
        }
        break

      case Op.EQ:
        {
          const [l0, l1] = [r0.label, r1.label]
          if (l0 instanceof Arg && l1 instanceof Arg) {
            if (l0.offset === l1.offset && l0.path === l1.path && l0.add_val === l1.add_val) {
              let mask = null
              if (l0.and_mask === null && l1.and_mask != null) {
                mask = l1.and_mask
              } else if (l0.and_mask != null && l1.and_mask === null) {
                mask = l0.and_mask
              }
              if (mask !== null) {
                let t = andMaskToType(mask)
                if (t !== null) {
                  args.setTname(l0.path, l0.offset, t, 20)
                }
              }
            }
          }
        }
        break

      case Op.ISZERO:
        {
          const rl = r0.label
          if (rl instanceof Arg) {
            vm.stack.peek().label = new IsZeroResult(rl)
          } else if (rl instanceof IsZeroResult) {
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
              args.setTname(rl.path, rl.offset, 'bool', 5)
            }
          }
        }
        break

      case Op.SIGNEXTEND:
        {
          const rl = r1.label
          if (rl instanceof Arg && r0 < 32n) {
            args.setTname(rl.path, rl.offset, `int${(Number(r0) + 1) * 8}`, 20)
          }
        }
        break

      case Op.BYTE:
        {
          const rl = r1.label
          if (rl instanceof Arg) {
            args.setTname(rl.path, rl.offset, 'bytes32', 4)
          }
        }
        break
    }
  }

  return args.joinToString()
}
