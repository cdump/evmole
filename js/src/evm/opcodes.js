export default class Op {
  constructor(code, blen, gas = undefined) {
    this.code = code
    this.blen = blen
    this.gas = gas
    this.name = '' // will set later in static{}
  }

  valueOf() {
    return this.code
  }

  static parse(code) {
    const op = this.#ops[code]
    if (op === undefined) throw `Unknown op code ${code}`
    return op
  }

  static STOP = new Op(0x00, 1, 0)
  static ADD = new Op(0x01, 1, 3)
  static MUL = new Op(0x02, 1, 5)
  static SUB = new Op(0x03, 1, 3)
  static DIV = new Op(0x04, 1, 5)
  static SDIV = new Op(0x05, 1, 5)
  static MOD = new Op(0x06, 1, 5)
  static SMOD = new Op(0x07, 1, 5)
  static ADDMOD = new Op(0x08, 1, 8)
  static MULMOD = new Op(0x09, 1, 8)
  static EXP = new Op(0x0a, 1)
  static SIGNEXTEND = new Op(0x0b, 1, 5)

  static LT = new Op(0x10, 1, 3)
  static GT = new Op(0x11, 1, 3)
  static SLT = new Op(0x12, 1, 3)
  static SGT = new Op(0x13, 1, 3)
  static EQ = new Op(0x14, 1, 3)
  static ISZERO = new Op(0x15, 1, 3)
  static AND = new Op(0x16, 1, 3)
  static OR = new Op(0x17, 1, 3)
  static XOR = new Op(0x18, 1, 3)
  static NOT = new Op(0x19, 1, 3)
  static BYTE = new Op(0x1a, 1, 3)
  static SHL = new Op(0x1b, 1, 3)
  static SHR = new Op(0x1c, 1, 3)
  static SAR = new Op(0x1d, 1, 3)

  static KECCAK256 = new Op(0x20, 1)

  static ADDRESS = new Op(0x30, 1, 2)
  static BALANCE = new Op(0x31, 1)
  static ORIGIN = new Op(0x32, 1, 2)
  static CALLER = new Op(0x33, 1, 2)
  static CALLVALUE = new Op(0x34, 1, 2)
  static CALLDATALOAD = new Op(0x35, 1, 3)
  static CALLDATASIZE = new Op(0x36, 1, 2)
  static CALLDATACOPY = new Op(0x37, 1)
  static CODESIZE = new Op(0x38, 1, 2)
  static CODECOPY = new Op(0x39, 1)
  static GASPRICE = new Op(0x3a, 1, 2)
  static EXTCODESIZE = new Op(0x3b, 1)
  static EXTCODECOPY = new Op(0x3c, 1)
  static RETURNDATASIZE = new Op(0x3d, 1)
  static RETURNDATACOPY = new Op(0x3e, 1)
  static EXTCODEHASH = new Op(0x3f, 1)

  static BLOCKHASH = new Op(0x40, 1, 20)
  static COINBASE = new Op(0x41, 1, 2)
  static TIMESTAMP = new Op(0x42, 1, 2)
  static NUMBER = new Op(0x43, 1, 2)
  static DIFFICULTY = new Op(0x44, 1, 2)
  static GASLIMIT = new Op(0x45, 1, 2)
  static CHAINID = new Op(0x46, 1, 2)
  static SELFBALANCE = new Op(0x47, 1, 5)
  static BASEFEE = new Op(0x48, 1, 2)

  static POP = new Op(0x50, 1, 2)
  static MLOAD = new Op(0x51, 1)
  static MSTORE = new Op(0x52, 1)
  static MSTORE8 = new Op(0x53, 1)
  static SLOAD = new Op(0x54, 1)
  static SSTORE = new Op(0x55, 1)
  static JUMP = new Op(0x56, 1, 8)
  static JUMPI = new Op(0x57, 1, 10)
  static PC = new Op(0x58, 1, 2)
  static MSIZE = new Op(0x59, 1, 2)
  static GAS = new Op(0x5a, 1, 2)
  static JUMPDEST = new Op(0x5b, 1, 1)
  static PUSH0 = new Op(0x5f, 1, 2)

  static PUSH1 = new Op(0x60, 2, 3)
  static PUSH2 = new Op(0x61, 3, 3)
  static PUSH3 = new Op(0x62, 4, 3)
  static PUSH4 = new Op(0x63, 5, 3)
  static PUSH5 = new Op(0x64, 6, 3)
  static PUSH6 = new Op(0x65, 7, 3)
  static PUSH7 = new Op(0x66, 8, 3)
  static PUSH8 = new Op(0x67, 9, 3)
  static PUSH9 = new Op(0x68, 10, 3)
  static PUSH10 = new Op(0x69, 11, 3)
  static PUSH11 = new Op(0x6a, 12, 3)
  static PUSH12 = new Op(0x6b, 13, 3)
  static PUSH13 = new Op(0x6c, 14, 3)
  static PUSH14 = new Op(0x6d, 15, 3)
  static PUSH15 = new Op(0x6e, 16, 3)
  static PUSH16 = new Op(0x6f, 17, 3)

  static PUSH17 = new Op(0x70, 18, 3)
  static PUSH18 = new Op(0x71, 19, 3)
  static PUSH19 = new Op(0x72, 20, 3)
  static PUSH20 = new Op(0x73, 21, 3)
  static PUSH21 = new Op(0x74, 22, 3)
  static PUSH22 = new Op(0x75, 23, 3)
  static PUSH23 = new Op(0x76, 24, 3)
  static PUSH24 = new Op(0x77, 25, 3)
  static PUSH25 = new Op(0x78, 26, 3)
  static PUSH26 = new Op(0x79, 27, 3)
  static PUSH27 = new Op(0x7a, 28, 3)
  static PUSH28 = new Op(0x7b, 29, 3)
  static PUSH29 = new Op(0x7c, 30, 3)
  static PUSH30 = new Op(0x7d, 31, 3)
  static PUSH31 = new Op(0x7e, 32, 3)
  static PUSH32 = new Op(0x7f, 33, 3)

  static DUP1 = new Op(0x80, 1, 3)
  static DUP2 = new Op(0x81, 1, 3)
  static DUP3 = new Op(0x82, 1, 3)
  static DUP4 = new Op(0x83, 1, 3)
  static DUP5 = new Op(0x84, 1, 3)
  static DUP6 = new Op(0x85, 1, 3)
  static DUP7 = new Op(0x86, 1, 3)
  static DUP8 = new Op(0x87, 1, 3)
  static DUP9 = new Op(0x88, 1, 3)
  static DUP10 = new Op(0x89, 1, 3)
  static DUP11 = new Op(0x8a, 1, 3)
  static DUP12 = new Op(0x8b, 1, 3)
  static DUP13 = new Op(0x8c, 1, 3)
  static DUP14 = new Op(0x8d, 1, 3)
  static DUP15 = new Op(0x8e, 1, 3)
  static DUP16 = new Op(0x8f, 1, 3)

  static SWAP1 = new Op(0x90, 1, 3)
  static SWAP2 = new Op(0x91, 1, 3)
  static SWAP3 = new Op(0x92, 1, 3)
  static SWAP4 = new Op(0x93, 1, 3)
  static SWAP5 = new Op(0x94, 1, 3)
  static SWAP6 = new Op(0x95, 1, 3)
  static SWAP7 = new Op(0x96, 1, 3)
  static SWAP8 = new Op(0x97, 1, 3)
  static SWAP9 = new Op(0x98, 1, 3)
  static SWAP10 = new Op(0x99, 1, 3)
  static SWAP11 = new Op(0x9a, 1, 3)
  static SWAP12 = new Op(0x9b, 1, 3)
  static SWAP13 = new Op(0x9c, 1, 3)
  static SWAP14 = new Op(0x9d, 1, 3)
  static SWAP15 = new Op(0x9e, 1, 3)
  static SWAP16 = new Op(0x9f, 1, 3)

  static LOG0 = new Op(0xa0, 1)
  static LOG1 = new Op(0xa1, 1)
  static LOG2 = new Op(0xa2, 1)
  static LOG3 = new Op(0xa3, 1)
  static LOG4 = new Op(0xa4, 1)

  static CREATE = new Op(0xf0, 1)
  static CALL = new Op(0xf1, 1)
  static CALLCODE = new Op(0xf2, 1)
  static RETURN = new Op(0xf3, 1)
  static DELEGATECALL = new Op(0xf4, 1)
  static CREATE2 = new Op(0xf5, 1)

  static STATICCALL = new Op(0xfa, 1)
  static REVERT = new Op(0xfd, 1)
  static INVALID = new Op(0xfe, 1)
  static SELFDESTRUCT = new Op(0xff, 1)

  static #ops = Array(256)
  static {
    for (const [k, v] of Object.entries(Op)) {
      if (v instanceof Op) {
        v.name = k
        this.#ops[v.code] = v
      }
    }
  }
}
