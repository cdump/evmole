import Op from './opcodes.js'
import Stack from './stack.js'
import Memory from './memory.js'
import { toBigInt, modExp, bigIntBitLength } from '../utils.js'

const E256 = 2n ** 256n
const E256M1 = E256 - 1n
const E255M1 = 2n ** 255n - 1n

export class BadJumpDestError extends Error {}
export class BlacklistedOpError extends Error {}
export class UnsupportedOpError extends Error {}

export class Vm {
  constructor(code, calldata, blacklisted_ops, clone = false) {
    if (clone) {
      return
    }
    this.code = code
    this.pc = 0
    this.stack = new Stack()
    this.memory = new Memory()
    this.stopped = false
    this.calldata = calldata
    this.blacklisted_ops =
      blacklisted_ops !== undefined ? blacklisted_ops : new Set()
  }

  toString() {
    let r = 'Vm:\n'
    r += ` .pc = 0x${this.pc.toString(16)} | ${this.current_op().name}\n`
    r += ` .stack = ${this.stack}\n`
    r += ` .memory = ${this.memory}\n`
    return r
  }

  clone() {
    const c = new Vm(undefined, undefined, undefined, true)
    c.code = this.code
    c.pc = this.pc
    c.stack = new Stack()
    c.stack._data = [...this.stack._data]
    c.memory = new Memory()
    c.memory._seq = this.memory._seq
    c.memory._data = [...this.memory._data]
    c.stopped = this.stopped
    c.calldata = this.calldata
    c.blacklisted_ops = this.blacklisted_ops
    return c
  }

  current_op() {
    return this.code[this.pc]
  }

  step() {
    const ret = this.#exec_next_opcode()
    const op = ret[0]
    if (ret[1] == -1) {
      throw `Op ${op.name} with unset gas_used`
    }
    if (op != Op.JUMP && op != Op.JUMPI) {
      this.pc += 1
    }
    if (this.pc >= this.code.length) {
      this.stopped = true
    }
    return ret
  }

  #exec_next_opcode() {
    const op = this.current_op()
    if (this.blacklisted_ops.has(op)) {
      throw new BlacklistedOpError(op.name)
    }

    if (op >= Op.PUSH0 && op <= Op.PUSH32) {
      const n = op - Op.PUSH0
      if (n != 0) {
        const args = this.code.subarray(this.pc + 1, this.pc + 1 + n)
        const v = new Uint8Array(32)
        v.set(args, v.length - args.length)
        this.stack.push(v)
        this.pc += n
        return [op, 3]
      } else {
        this.stack.push_uint(0n)
        return [op, 2]
      }
    }
    if (op >= Op.DUP1 && op <= Op.DUP16) {
      this.stack.dup(op - Op.DUP1 + 1)
      return [op, 3]
    }
    if (op >= Op.SWAP1 && op <= Op.SWAP16) {
      this.stack.swap(op - Op.SWAP1 + 1)
      return [op, 3]
    }

    switch (op) {
      case Op.JUMP:
      case Op.JUMPI: {
        const s0 = Number(this.stack.pop_uint())
        if (s0 >= this.code.length || this.code[s0] != Op.JUMPDEST) {
          throw new BadJumpDestError(`pos ${s0}`)
        }
        if (op == Op.JUMPI) {
          const s1 = this.stack.pop_uint()
          if (s1 == 0n) {
            this.pc += 1
            return [op, 10]
          }
        }
        this.pc = Number(s0)
        return [op, op === Op.JUMP ? 8 : 10]
      }

      case Op.JUMPDEST:
        return [op, 1]

      case Op.REVERT:
        this.stack.pop()
        this.stack.pop()
        this.stopped = true
        return [op, 4]

      case Op.EQ:
      case Op.LT:
      case Op.GT:
      case Op.SUB:
      case Op.ADD:
      case Op.DIV:
      case Op.MUL:
      case Op.EXP:
      case Op.XOR:
      case Op.AND:
      case Op.OR:
      case Op.SHR:
      case Op.SHL:
      case Op.BYTE: {
        const raws0 = this.stack.pop()
        const raws1 = this.stack.pop()

        const s0 = toBigInt(raws0)
        const s1 = toBigInt(raws1)

        let res
        let gas_used = 3
        switch (op) {
          case Op.EQ:
            res = s0 == s1 ? 1n : 0n
            break
          case Op.LT:
            res = s0 < s1 ? 1n : 0n
            break
          case Op.GT:
            res = s0 > s1 ? 1n : 0n
            break
          case Op.SUB:
            res = (s0 - s1) & E256M1
            break
          case Op.ADD:
            res = (s0 + s1) & E256M1
            break
          case Op.DIV:
            res = s1 != 0n ? s0 / s1 : 0n
            gas_used = 5
            break
          case Op.MUL:
            res = (s0 * s1) & E256M1
            gas_used = 5
            break
          case Op.EXP:
            res = modExp(s0, s1, E256)
            gas_used = 50 * (1 + Math.floor(bigIntBitLength(s1) / 8)) // ~approx
            break
          case Op.XOR:
            res = s0 ^ s1
            break
          case Op.AND:
            res = s0 & s1
            break
          case Op.OR:
            res = s0 | s1
            break
          case Op.SHR:
            res = s0 >= 256n ? 0n : (s1 >> s0) & E256M1
            break
          case Op.SHL:
            res = s0 >= 256n ? 0n : (s1 << s0) & E256M1
            break
          case Op.BYTE:
            res = s0 >= 32n ? 0n : BigInt(raws1[s0])
            break
        }
        this.stack.push_uint(res)
        return [op, gas_used, raws0, raws1]
      }

      case Op.SLT:
      case Op.SGT: {
        let s0 = this.stack.pop_uint()
        let s1 = this.stack.pop_uint()

        // unsigned to signed
        s0 = s0 <= E255M1 ? s0 : s0 - E256
        s1 = s1 <= E255M1 ? s1 : s1 - E256
        let res
        if (op === Op.SLT) {
          res = s0 < s1 ? 1n : 0n
        } else {
          res = s0 > s1 ? 1n : 0n
        }
        this.stack.push_uint(res)
        return [op, 3]
      }

      case Op.ISZERO: {
        const raw = this.stack.pop()
        const v = toBigInt(raw)
        this.stack.push_uint(v === 0n ? 1n : 0n)
        return [op, 3, raw]
      }

      case Op.POP:
        this.stack.pop()
        return [op, 2]

      case Op.CALLVALUE:
        this.stack.push_uint(0n) // msg.value == 0
        return [op, 2]

      case Op.CALLDATALOAD: {
        const raws0 = this.stack.pop()
        const offset = Number(toBigInt(raws0))
        this.stack.push(this.calldata.load(offset))
        return [op, 3, raws0]
      }

      case Op.CALLDATASIZE:
        this.stack.push_uint(BigInt(this.calldata.length))
        return [op, 2]

      case Op.MSTORE: {
        const offset = Number(this.stack.pop_uint())
        const v = this.stack.pop()
        this.memory.store(offset, v)
        return [op, 3]
      }

      case Op.MLOAD: {
        const offset = Number(this.stack.pop_uint())
        const [val, used] = this.memory.load(offset)
        this.stack.push(val)
        return [op, 4, used]
      }

      case Op.NOT: {
        const s0 = this.stack.pop_uint()
        this.stack.push_uint(E256M1 - s0)
        return [op, 3]
      }

      case Op.SIGNEXTEND: {
        const s0 = this.stack.pop_uint()
        const raws1 = this.stack.pop()
        const s1 = toBigInt(raws1)
        let res = s1
        if (s0 <= 31) {
          const sign_bit = 1n << (s0 * 8n + 7n)
          if (s1 & sign_bit) {
            res = s1 | (E256 - sign_bit)
          } else {
            res = s1 & (sign_bit - 1n)
          }
        }
        this.stack.push_uint(res)
        return [op, 5, s0, raws1]
      }

      case Op.ADDRESS: {
        this.stack.push_uint(1n)
        return [op, 2]
      }

      case Op.CALLDATACOPY: {
        const mem_off = Number(this.stack.pop_uint())
        const src_off = Number(this.stack.pop_uint())
        const size = Number(this.stack.pop_uint())
        const value = this.calldata.load(src_off, size)
        this.memory.store(mem_off, value)
        return [op, 4]
      }

      default:
        throw new UnsupportedOpError(op.name)
    }
  }
}
