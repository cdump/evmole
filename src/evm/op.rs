#![allow(dead_code)]

pub type OpCode = u8;

#[rustfmt::skip]
const NAMES: [&str; 256] = ["STOP","ADD","MUL","SUB","DIV","SDIV","MOD","SMOD","ADDMOD","MULMOD","EXP","SIGNEXTEND","?","?","?","?","LT","GT","SLT","SGT","EQ","ISZERO","AND","OR","XOR","NOT","BYTE","SHL","SHR","SAR","?","?","KECCAK256","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","ADDRESS","BALANCE","ORIGIN","CALLER","CALLVALUE","CALLDATALOAD","CALLDATASIZE","CALLDATACOPY","CODESIZE","CODECOPY","GASPRICE","EXTCODESIZE","EXTCODECOPY","RETURNDATASIZE","RETURNDATACOPY","EXTCODEHASH","BLOCKHASH","COINBASE","TIMESTAMP","NUMBER","PREVRANDAO","GASLIMIT","CHAINID","SELFBALANCE","BASEFEE","BLOBHASH","BLOBBASEFEE","?","?","?","?","?","POP","MLOAD","MSTORE","MSTORE8","SLOAD","SSTORE","JUMP","JUMPI","PC","MSIZE","GAS","JUMPDEST","TLOAD","TSTORE","MCOPY","PUSH0","PUSH1","PUSH2","PUSH3","PUSH4","PUSH5","PUSH6","PUSH7","PUSH8","PUSH9","PUSH10","PUSH11","PUSH12","PUSH13","PUSH14","PUSH15","PUSH16","PUSH17","PUSH18","PUSH19","PUSH20","PUSH21","PUSH22","PUSH23","PUSH24","PUSH25","PUSH26","PUSH27","PUSH28","PUSH29","PUSH30","PUSH31","PUSH32","DUP1","DUP2","DUP3","DUP4","DUP5","DUP6","DUP7","DUP8","DUP9","DUP10","DUP11","DUP12","DUP13","DUP14","DUP15","DUP16","SWAP1","SWAP2","SWAP3","SWAP4","SWAP5","SWAP6","SWAP7","SWAP8","SWAP9","SWAP10","SWAP11","SWAP12","SWAP13","SWAP14","SWAP15","SWAP16","LOG0","LOG1","LOG2","LOG3","LOG4","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","?","CREATE","CALL","CALLCODE","RETURN","DELEGATECALL","CREATE2","?","?","?","?","STATICCALL","?","?","REVERT","INVALID","SELFDESTRUCT"];

pub fn name(op: OpCode) -> &'static str {
    NAMES[op as usize]
}

pub const STOP: OpCode = 0x00;
pub const ADD: OpCode = 0x01;
pub const MUL: OpCode = 0x02;
pub const SUB: OpCode = 0x03;
pub const DIV: OpCode = 0x04;
pub const SDIV: OpCode = 0x05;
pub const MOD: OpCode = 0x06;
pub const SMOD: OpCode = 0x07;
pub const ADDMOD: OpCode = 0x08;
pub const MULMOD: OpCode = 0x09;
pub const EXP: OpCode = 0x0A;
pub const SIGNEXTEND: OpCode = 0x0B;

pub const LT: OpCode = 0x10;
pub const GT: OpCode = 0x11;
pub const SLT: OpCode = 0x12;
pub const SGT: OpCode = 0x13;
pub const EQ: OpCode = 0x14;
pub const ISZERO: OpCode = 0x15;
pub const AND: OpCode = 0x16;
pub const OR: OpCode = 0x17;
pub const XOR: OpCode = 0x18;
pub const NOT: OpCode = 0x19;
pub const BYTE: OpCode = 0x1A;
pub const SHL: OpCode = 0x1B;
pub const SHR: OpCode = 0x1C;
pub const SAR: OpCode = 0x1D;

pub const KECCAK256: OpCode = 0x20;

pub const ADDRESS: OpCode = 0x30;
pub const BALANCE: OpCode = 0x31;
pub const ORIGIN: OpCode = 0x32;
pub const CALLER: OpCode = 0x33;
pub const CALLVALUE: OpCode = 0x34;
pub const CALLDATALOAD: OpCode = 0x35;
pub const CALLDATASIZE: OpCode = 0x36;
pub const CALLDATACOPY: OpCode = 0x37;
pub const CODESIZE: OpCode = 0x38;
pub const CODECOPY: OpCode = 0x39;
pub const GASPRICE: OpCode = 0x3A;
pub const EXTCODESIZE: OpCode = 0x3B;
pub const EXTCODECOPY: OpCode = 0x3C;
pub const RETURNDATASIZE: OpCode = 0x3D;
pub const RETURNDATACOPY: OpCode = 0x3E;
pub const EXTCODEHASH: OpCode = 0x3F;

pub const BLOCKHASH: OpCode = 0x40;
pub const COINBASE: OpCode = 0x41;
pub const TIMESTAMP: OpCode = 0x42;
pub const NUMBER: OpCode = 0x43;
pub const PREVRANDAO: OpCode = 0x44;
pub const GASLIMIT: OpCode = 0x45;
pub const CHAINID: OpCode = 0x46;
pub const SELFBALANCE: OpCode = 0x47;
pub const BASEFEE: OpCode = 0x48;
pub const BLOBHASH: OpCode = 0x49;
pub const BLOBBASEFEE: OpCode = 0x4A;

pub const POP: OpCode = 0x50;
pub const MLOAD: OpCode = 0x51;
pub const MSTORE: OpCode = 0x52;
pub const MSTORE8: OpCode = 0x53;
pub const SLOAD: OpCode = 0x54;
pub const SSTORE: OpCode = 0x55;
pub const JUMP: OpCode = 0x56;
pub const JUMPI: OpCode = 0x57;
pub const PC: OpCode = 0x58;
pub const MSIZE: OpCode = 0x59;
pub const GAS: OpCode = 0x5A;
pub const JUMPDEST: OpCode = 0x5B;
pub const TLOAD: OpCode = 0x5C;
pub const TSTORE: OpCode = 0x5D;
pub const MCOPY: OpCode = 0x5E;
pub const PUSH0: OpCode = 0x5F;

pub const PUSH1: OpCode = 0x60;
pub const PUSH2: OpCode = 0x61;
pub const PUSH3: OpCode = 0x62;
pub const PUSH4: OpCode = 0x63;
pub const PUSH5: OpCode = 0x64;
pub const PUSH6: OpCode = 0x65;
pub const PUSH7: OpCode = 0x66;
pub const PUSH8: OpCode = 0x67;
pub const PUSH9: OpCode = 0x68;
pub const PUSH10: OpCode = 0x69;
pub const PUSH11: OpCode = 0x6A;
pub const PUSH12: OpCode = 0x6B;
pub const PUSH13: OpCode = 0x6C;
pub const PUSH14: OpCode = 0x6D;
pub const PUSH15: OpCode = 0x6E;
pub const PUSH16: OpCode = 0x6F;

pub const PUSH17: OpCode = 0x70;
pub const PUSH18: OpCode = 0x71;
pub const PUSH19: OpCode = 0x72;
pub const PUSH20: OpCode = 0x73;
pub const PUSH21: OpCode = 0x74;
pub const PUSH22: OpCode = 0x75;
pub const PUSH23: OpCode = 0x76;
pub const PUSH24: OpCode = 0x77;
pub const PUSH25: OpCode = 0x78;
pub const PUSH26: OpCode = 0x79;
pub const PUSH27: OpCode = 0x7A;
pub const PUSH28: OpCode = 0x7B;
pub const PUSH29: OpCode = 0x7C;
pub const PUSH30: OpCode = 0x7D;
pub const PUSH31: OpCode = 0x7E;
pub const PUSH32: OpCode = 0x7F;

pub const DUP1: OpCode = 0x80;
pub const DUP2: OpCode = 0x81;
pub const DUP3: OpCode = 0x82;
pub const DUP4: OpCode = 0x83;
pub const DUP5: OpCode = 0x84;
pub const DUP6: OpCode = 0x85;
pub const DUP7: OpCode = 0x86;
pub const DUP8: OpCode = 0x87;
pub const DUP9: OpCode = 0x88;
pub const DUP10: OpCode = 0x89;
pub const DUP11: OpCode = 0x8A;
pub const DUP12: OpCode = 0x8B;
pub const DUP13: OpCode = 0x8C;
pub const DUP14: OpCode = 0x8D;
pub const DUP15: OpCode = 0x8E;
pub const DUP16: OpCode = 0x8F;

pub const SWAP1: OpCode = 0x90;
pub const SWAP2: OpCode = 0x91;
pub const SWAP3: OpCode = 0x92;
pub const SWAP4: OpCode = 0x93;
pub const SWAP5: OpCode = 0x94;
pub const SWAP6: OpCode = 0x95;
pub const SWAP7: OpCode = 0x96;
pub const SWAP8: OpCode = 0x97;
pub const SWAP9: OpCode = 0x98;
pub const SWAP10: OpCode = 0x99;
pub const SWAP11: OpCode = 0x9A;
pub const SWAP12: OpCode = 0x9B;
pub const SWAP13: OpCode = 0x9C;
pub const SWAP14: OpCode = 0x9D;
pub const SWAP15: OpCode = 0x9E;
pub const SWAP16: OpCode = 0x9F;

pub const LOG0: OpCode = 0xA0;
pub const LOG1: OpCode = 0xA1;
pub const LOG2: OpCode = 0xA2;
pub const LOG3: OpCode = 0xA3;
pub const LOG4: OpCode = 0xA4;

pub const CREATE: OpCode = 0xF0;
pub const CALL: OpCode = 0xF1;
pub const CALLCODE: OpCode = 0xF2;
pub const RETURN: OpCode = 0xF3;
pub const DELEGATECALL: OpCode = 0xF4;
pub const CREATE2: OpCode = 0xF5;

pub const STATICCALL: OpCode = 0xFA;
pub const REVERT: OpCode = 0xFD;
pub const INVALID: OpCode = 0xFE;
pub const SELFDESTRUCT: OpCode = 0xFF;
