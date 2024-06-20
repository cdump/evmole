import Op from './opcodes.js'
import Stack from './stack.js'
import Memory from './memory.js'
import Element from './element.js'
import { toBigInt, modExp, bigIntBitLength } from '../utils.js'

const E256 = 2n ** 256n
const E256M1 = E256 - 1n
const E255M1 = 2n ** 255n - 1n

export class UnsupportedOpError extends Error {}

export class Vm {
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
    r += ` .pc = 0x${this.pc.toString(16)} | ${Op.name(this.current_op())}\n`
    r += ` .stack = ${this.stack}\n`
    r += ` .memory = ${this.memory}\n`
    return r
  }

  clone() {
    const c = new Vm(undefined, undefined, true)
    c.code = this.code
    c.pc = this.pc
    c.stack = new Stack()
    c.stack.data = [...this.stack.data]
    c.memory = new Memory()
    c.memory.data = [...this.memory.data]
    c.stopped = this.stopped
    c.calldata = this.calldata
    return c
  }

  current_op() {
    return this.code[this.pc]
  }

  step() {
    const op = this.current_op()
    const ret = this.#exec_opcode(op)
    if (op != Op.JUMP && op != Op.JUMPI) {
      this.pc += 1
    }
    if (this.pc >= this.code.length) {
      this.stopped = true
    }
    return [op, ...ret]
  }

  #bop(cb) {
    const raws0 = this.stack.pop()
    const raws1 = this.stack.pop()

    const s0 = toBigInt(raws0.data)
    const s1 = toBigInt(raws1.data)

    const [gas_used, res] = cb(raws0, s0, raws1, s1)

    this.stack.push_uint(res)
    return [gas_used, raws0, raws1]
  }

  #exec_opcode(op) {
    if (op >= Op.PUSH0 && op <= Op.PUSH32) {
      const n = op - Op.PUSH0
      if (n != 0) {
        const args = this.code.subarray(this.pc + 1, this.pc + 1 + n)
        const v = new Uint8Array(32)
        v.set(args, v.length - args.length)
        this.stack.push(new Element(v))
        this.pc += n
        return [3]
      } else {
        this.stack.push_uint(0n)
        return [2]
      }
    }
    if (op >= Op.DUP1 && op <= Op.DUP16) {
      this.stack.dup(op - Op.DUP1 + 1)
      return [3]
    }
    if (op >= Op.SWAP1 && op <= Op.SWAP16) {
      this.stack.swap(op - Op.SWAP1 + 1)
      return [3]
    }

    switch (op) {
      case Op.JUMP:
      case Op.JUMPI: {
        const s0 = Number(this.stack.pop_uint())
        if (op == Op.JUMPI) {
          const s1 = this.stack.pop_uint()
          if (s1 == 0n) {
            this.pc += 1
            return [10]
          }
        }
        if (s0 >= this.code.length || this.code[s0] != Op.JUMPDEST) {
          throw new UnsupportedOpError(op)
        }
        this.pc = s0
        return [op === Op.JUMP ? 8 : 10]
      }

      case Op.JUMPDEST:
        return [1]

      case Op.REVERT:
      case Op.STOP:
      case Op.RETURN:
        // skip stack pop()s
        this.stopped = true
        return [4]

      case Op.EQ:
        return this.#bop((raws0, s0, raws1, s1) => [3, s0 == s1 ? 1n : 0n])

      case Op.LT:
        return this.#bop((raws0, s0, raws1, s1) => [3, s0 < s1 ? 1n : 0n])

      case Op.GT:
        return this.#bop((raws0, s0, raws1, s1) => [3, s0 > s1 ? 1n : 0n])

      case Op.SUB:
        return this.#bop((raws0, s0, raws1, s1) => [3, (s0 - s1) & E256M1])

      case Op.ADD:
        return this.#bop((raws0, s0, raws1, s1) => [3, (s0 + s1) & E256M1])

      case Op.DIV:
        return this.#bop((raws0, s0, raws1, s1) => [5, s1 != 0n ? s0 / s1 : 0n])

      case Op.MOD:
        return this.#bop((raws0, s0, raws1, s1) => [5, s1 != 0n ? s0 % s1 : 0n])

      case Op.MUL:
        return this.#bop((raws0, s0, raws1, s1) => [5, (s0 * s1) & E256M1])

      case Op.EXP:
        return this.#bop((raws0, s0, raws1, s1) => [
          50 * (1 + Math.floor(bigIntBitLength(s1) / 8)), // ~approx
          modExp(s0, s1, E256),
        ])

      case Op.XOR:
        return this.#bop((raws0, s0, raws1, s1) => [3, s0 ^ s1])

      case Op.AND:
        return this.#bop((raws0, s0, raws1, s1) => [3, s0 & s1])

      case Op.OR:
        return this.#bop((raws0, s0, raws1, s1) => [3, s0 | s1])

      case Op.SHR:
        return this.#bop((raws0, s0, raws1, s1) => [3, s0 >= 256n ? 0n : (s1 >> s0) & E256M1])

      case Op.SHL:
        return this.#bop((raws0, s0, raws1, s1) => [3, s0 >= 256n ? 0n : (s1 << s0) & E256M1])

      case Op.SLT:
        return this.#bop((raws0, s0, raws1, s1) => {
          // unsigned to signed
          const a = s0 <= E255M1 ? s0 : s0 - E256
          const b = s1 <= E255M1 ? s1 : s1 - E256
          return [3, a < b ? 1n : 0n]
        })

      case Op.SGT:
        return this.#bop((raws0, s0, raws1, s1) => {
          // unsigned to signed
          const a = s0 <= E255M1 ? s0 : s0 - E256
          const b = s1 <= E255M1 ? s1 : s1 - E256
          return [3, a > b ? 1n : 0n]
        })

      case Op.BYTE:
        return this.#bop((raws0, s0, raws1) => [3, s0 >= 32n ? 0n : BigInt(raws1.data[s0])])

      case Op.ISZERO: {
        const raw = this.stack.pop()
        const v = toBigInt(raw.data)
        this.stack.push_uint(v === 0n ? 1n : 0n)
        return [3, raw]
      }

      case Op.POP:
        this.stack.pop()
        return [2]

      case Op.CALLVALUE:
        this.stack.push_uint(0n) // msg.value == 0
        return [2]

      case Op.CALLDATALOAD: {
        const raws0 = this.stack.pop()
        const offset = Number(toBigInt(raws0.data))
        this.stack.push(this.calldata.load(offset))
        return [3, raws0]
      }

      case Op.CALLDATASIZE:
        this.stack.push_uint(BigInt(this.calldata.length))
        return [2]

      case Op.MSIZE:
        this.stack.push_uint(BigInt(this.memory.size()))
        return [2]

      case Op.MSTORE8: {
        const offset = Number(this.stack.pop_uint())
        const v = this.stack.pop()
        const el = new Element(
          v.data.subarray(v.length - 1), // v[31]
          v.label,
        )
        this.memory.store(offset, el)
        return [3]
      }

      case Op.MSTORE: {
        const offset = Number(this.stack.pop_uint())
        const v = this.stack.pop()
        this.memory.store(offset, v)
        return [3]
      }

      case Op.MLOAD: {
        const offset = Number(this.stack.pop_uint())
        const [val, used] = this.memory.load(offset)
        this.stack.push(val)
        return [4, used]
      }

      case Op.NOT: {
        const s0 = this.stack.pop_uint()
        this.stack.push_uint(E256M1 - s0)
        return [3]
      }

      case Op.SIGNEXTEND: {
        const s0 = this.stack.pop_uint()
        const raws1 = this.stack.pop()
        const s1 = toBigInt(raws1.data)
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
        return [5, s0, raws1]
      }

      case Op.ADDRESS:
      case Op.ORIGIN:
      case Op.CALLER:
        this.stack.push_uint(0n)
        return [2]

      case Op.CALLDATACOPY: {
        const mem_off = Number(this.stack.pop_uint())
        const src_off = Number(this.stack.pop_uint())
        const size = Number(this.stack.pop_uint())
        if (size > 512) {
          throw new UnsupportedOpError(op)
        }
        const value = this.calldata.load(src_off, size)
        this.memory.store(mem_off, value)
        return [4]
      }

      case Op.CODECOPY: {
        const mem_off = Number(this.stack.pop_uint())
        const src_off = Number(this.stack.pop_uint())
        const size = Number(this.stack.pop_uint())
        if (src_off + size > this.code.length) {
          throw new UnsupportedOpError(op)
        }
        const value = this.code.subarray(src_off, src_off + size)
        this.memory.store(mem_off, new Element(value))
        return [3]
      }

      case Op.SLOAD: {
        const slot = this.stack.pop()
        this.stack.push_uint(0n)
        return [100, slot]
      }

      case Op.SSTORE: {
        const slot = this.stack.pop()
        const sval = this.stack.pop()
        return [100, slot, sval]
      }

      case Op.BALANCE:
        this.stack.pop()
        this.stack.push_uint(1n)
        return [100]

      case Op.SELFBALANCE:
        this.stack.push_uint(1n)
        return [5]

      case Op.GAS:
        this.stack.push_uint(1_000_000n)
        return [2]

      case Op.CALL:
      case Op.DELEGATECALL:
      case Op.STATICCALL: {
        this.stack.pop()
        const p1 = this.stack.pop()
        const p2 = this.stack.pop()
        this.stack.pop()
        this.stack.pop()
        this.stack.pop()

        if (op === Op.CALL) {
          this.stack.pop()
        }

        this.stack.push_uint(0n) // failure

        if (op === Op.CALL) {
          return [100, p1, p2]
        }
        return [100, p1]
      }

      default:
        throw new UnsupportedOpError(op)
    }
  }
}
