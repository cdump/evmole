export default class Op {
  static name(code) {
    return this.#names[code]
  }

  static STOP = 0x00
  static ADD = 0x01
  static MUL = 0x02
  static SUB = 0x03
  static DIV = 0x04
  static SDIV = 0x05
  static MOD = 0x06
  static SMOD = 0x07
  static ADDMOD = 0x08
  static MULMOD = 0x09
  static EXP = 0x0a
  static SIGNEXTEND = 0x0b

  static LT = 0x10
  static GT = 0x11
  static SLT = 0x12
  static SGT = 0x13
  static EQ = 0x14
  static ISZERO = 0x15
  static AND = 0x16
  static OR = 0x17
  static XOR = 0x18
  static NOT = 0x19
  static BYTE = 0x1a
  static SHL = 0x1b
  static SHR = 0x1c
  static SAR = 0x1d

  static KECCAK256 = 0x20

  static ADDRESS = 0x30
  static BALANCE = 0x31
  static ORIGIN = 0x32
  static CALLER = 0x33
  static CALLVALUE = 0x34
  static CALLDATALOAD = 0x35
  static CALLDATASIZE = 0x36
  static CALLDATACOPY = 0x37
  static CODESIZE = 0x38
  static CODECOPY = 0x39
  static GASPRICE = 0x3a
  static EXTCODESIZE = 0x3b
  static EXTCODECOPY = 0x3c
  static RETURNDATASIZE = 0x3d
  static RETURNDATACOPY = 0x3e
  static EXTCODEHASH = 0x3f

  static BLOCKHASH = 0x40
  static COINBASE = 0x41
  static TIMESTAMP = 0x42
  static NUMBER = 0x43
  static DIFFICULTY = 0x44
  static GASLIMIT = 0x45
  static CHAINID = 0x46
  static SELFBALANCE = 0x47
  static BASEFEE = 0x48
  static BLOBHASH = 0x49
  static BLOBBASEFEE = 0x4a

  static POP = 0x50
  static MLOAD = 0x51
  static MSTORE = 0x52
  static MSTORE8 = 0x53
  static SLOAD = 0x54
  static SSTORE = 0x55
  static JUMP = 0x56
  static JUMPI = 0x57
  static PC = 0x58
  static MSIZE = 0x59
  static GAS = 0x5a
  static JUMPDEST = 0x5b
  static TLOAD = 0x5c
  static TSTORE = 0x5d
  static MCOPY = 0x5e
  static PUSH0 = 0x5f

  static PUSH1 = 0x60
  static PUSH2 = 0x61
  static PUSH3 = 0x62
  static PUSH4 = 0x63
  static PUSH5 = 0x64
  static PUSH6 = 0x65
  static PUSH7 = 0x66
  static PUSH8 = 0x67
  static PUSH9 = 0x68
  static PUSH10 = 0x69
  static PUSH11 = 0x6a
  static PUSH12 = 0x6b
  static PUSH13 = 0x6c
  static PUSH14 = 0x6d
  static PUSH15 = 0x6e
  static PUSH16 = 0x6f

  static PUSH17 = 0x70
  static PUSH18 = 0x71
  static PUSH19 = 0x72
  static PUSH20 = 0x73
  static PUSH21 = 0x74
  static PUSH22 = 0x75
  static PUSH23 = 0x76
  static PUSH24 = 0x77
  static PUSH25 = 0x78
  static PUSH26 = 0x79
  static PUSH27 = 0x7a
  static PUSH28 = 0x7b
  static PUSH29 = 0x7c
  static PUSH30 = 0x7d
  static PUSH31 = 0x7e
  static PUSH32 = 0x7f

  static DUP1 = 0x80
  static DUP2 = 0x81
  static DUP3 = 0x82
  static DUP4 = 0x83
  static DUP5 = 0x84
  static DUP6 = 0x85
  static DUP7 = 0x86
  static DUP8 = 0x87
  static DUP9 = 0x88
  static DUP10 = 0x89
  static DUP11 = 0x8a
  static DUP12 = 0x8b
  static DUP13 = 0x8c
  static DUP14 = 0x8d
  static DUP15 = 0x8e
  static DUP16 = 0x8f

  static SWAP1 = 0x90
  static SWAP2 = 0x91
  static SWAP3 = 0x92
  static SWAP4 = 0x93
  static SWAP5 = 0x94
  static SWAP6 = 0x95
  static SWAP7 = 0x96
  static SWAP8 = 0x97
  static SWAP9 = 0x98
  static SWAP10 = 0x99
  static SWAP11 = 0x9a
  static SWAP12 = 0x9b
  static SWAP13 = 0x9c
  static SWAP14 = 0x9d
  static SWAP15 = 0x9e
  static SWAP16 = 0x9f

  static LOG0 = 0xa0
  static LOG1 = 0xa1
  static LOG2 = 0xa2
  static LOG3 = 0xa3
  static LOG4 = 0xa4

  static CREATE = 0xf0
  static CALL = 0xf1
  static CALLCODE = 0xf2
  static RETURN = 0xf3
  static DELEGATECALL = 0xf4
  static CREATE2 = 0xf5

  static STATICCALL = 0xfa
  static REVERT = 0xfd
  static INVALID = 0xfe
  static SELFDESTRUCT = 0xff

  static #names = Array(256)
  static {
    for (const [k, v] of Object.entries(Op)) {
      this.#names[v] = k
    }
  }
}
