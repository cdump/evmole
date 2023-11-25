import Op from './opcodes.js'
import Stack from './stack.js'
import Memory from './memory.js'
import { uint8ArrayToBigInt, hexToUint8Array, modExp } from '../utils.js'

const E256 = 2n ** 256n
const E256M1 = E256 - 1n

function toBigInt(v) {
  if (typeof v === 'bigint') return v
  if (typeof v.toBigInt === 'function') return v.toBigInt()
  if (!(v instanceof Uint8Array)) throw `Not uint8array instance`
  return uint8ArrayToBigInt(v)
}

export default class Vm {
  constructor(code, calldata, clone = false) {
    if (clone) {
      return
    }
    this.code = code
    this.pc = 0
    this.stack = new Stack()
    this.memory = new Memory()
    this.stopped = false
    this.calldata = calldata
  }

  toString() {
    let r = 'Vm:\n'
    r += `  .pc = 0x${this.pc.toString(16)} | ${this.current_op().name}\n`
    r += `  .stack = ${this.stack}\n`
    return r
  }

  clone() {
    const c = new Vm(0, 0, true)
    c.code = this.code
    c.pc = this.pc
    c.stack = new Stack()
    c.stack._data = [...this.stack._data]
    c.memory = new Memory()
    c.memory._data = [...this.memory._data]
    c.stopped = this.stopped
    c.calldata = this.calldata
    return c
  }

  current_op() {
    return Op.parse(this.code[this.pc])
  }

  step() {
    const ret = this.#exec_next_opcode()
    const op = ret[0]
    if (ret[1] == -1) {
      throw `Op ${op.name} with unset gas_used`
    }

    if (op != Op.JUMP && op != Op.JUMPI) {
      this.pc += op.blen
    }
    if (this.pc >= this.code.length) {
      this.stopped = true
    }
    return ret
  }

  #exec_next_opcode() {
    const op = this.current_op()
    let gas_used = op.gas !== undefined ? op.gas : -1

    if (op >= Op.PUSH0 && op <= Op.PUSH32) {
      const n = op - Op.PUSH0
      if (n != 0) {
        const args = this.code.subarray(this.pc + 1, this.pc + 1 + n)
        this.stack.push(uint8ArrayToBigInt(args))
      } else {
        this.stack.push(0n)
      }
      return [op, gas_used]
    }
    if (op >= Op.DUP1 && op <= Op.DUP16) {
      this.stack.dup(op - Op.DUP1 + 1)
      return [op, gas_used]
    }
    if (op >= Op.SWAP1 && op <= Op.SWAP16) {
      this.stack.swap(op - Op.SWAP1 + 1)
      return [op, gas_used]
    }

    switch (op) {
      case Op.JUMP:
      case Op.JUMPI: {
        const s0 = Number(this.stack.pop())
        if (this.code[s0] != Op.JUMPDEST.code) {
          throw 'jump to not JUMPDEST'
        }
        if (op == Op.JUMPI) {
          const s1 = this.stack.pop()
          if (s1 == 0n) {
            this.pc += 1
            return [op, gas_used]
          }
        }
        this.pc = Number(s0)
        return [op, gas_used]
      }

      case Op.JUMPDEST:
        return [op, gas_used]

      case Op.REVERT:
        this.stack.pop()
        this.stack.pop()
        this.stopped = true
        return [op, 4]

      case Op.ISZERO: {
        const raw = this.stack.pop()
        const v = toBigInt(raw)
        this.stack.push(v === 0n ? 1n : 0n)
        return [op, gas_used, raw]
      }

      case Op.POP:
        this.stack.pop()
        return [op]

      case Op.LT:
      case Op.GT:
      case Op.EQ:
      case Op.SUB:
      case Op.DIV:
      case Op.EXP:
      case Op.XOR:
      case Op.AND:
      case Op.SHR: {
        const raws0 = this.stack.pop()
        const raws1 = this.stack.pop()

        const s0 = toBigInt(raws0)
        const s1 = toBigInt(raws1)

        let res
        switch (op) {
          case Op.EQ:
            res = s0 == s1 ? 1n : 0n
            break
          case Op.GT:
            res = s0 > s1 ? 1n : 0n
            break
          case Op.LT:
            res = s0 < s1 ? 1n : 0n
            break
          case Op.SUB:
            res = (s0 - s1) & E256M1
            break
          case Op.DIV:
            res = s1 != 0n ? s0 / s1 : 0n
            break
          case Op.EXP:
            res = modExp(s0, s1, E256)
            gas_used = 50 * (1 + Math.floor(s1.toString(2).length / 8))  // ~approx
            break
          case Op.XOR:
            res = s0 ^ s1
            break
          case Op.AND:
            res = s0 & s1
            break
          case Op.SHR:
            res = (s1 >> s0) & E256M1
            break
        }
        this.stack.push(res)
        return [op, gas_used, raws0, raws1]
      }

      case Op.CALLVALUE:
        this.stack.push(0n) // msg.value == 0
        return [op, gas_used]

      case Op.CALLDATALOAD: {
        const offset = Number(this.stack.pop())
        this.stack.push(this.calldata.load(offset))
        return [op, gas_used]
      }

      case Op.CALLDATASIZE:
        this.stack.push(BigInt(this.calldata.length))
        return [op, gas_used]

      case Op.MSTORE: {
        const offset = Number(this.stack.pop())
        const raw = this.stack.pop()
        const v =
          typeof raw === 'bigint' ? hexToUint8Array(raw.toString(16)) : raw
        this.memory.store(offset, v)
        return [op, 3]
      }

      case Op.MLOAD: {
        const offset = Number(this.stack.pop())
        const [val, used] = this.memory.load(offset)
        this.stack.push(uint8ArrayToBigInt(val))
        return [op, 4, used]
      }

      case Op.CALLDATACOPY: {
        const mem_off = Number(this.stack.pop())
        const src_off = Number(this.stack.pop())
        const size = Number(this.stack.pop())
        const value = this.calldata.load(src_off, size)
        this.memory.store(mem_off, value)
        return [op, 4]
      }

      default:
        throw `unknown op ${op.name}`
    }
  }
}
