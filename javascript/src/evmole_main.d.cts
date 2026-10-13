// CommonJS types for the package root ("evmole"), which only exports contractInfo.
// Type-only references with resolution-mode avoid requiring the ESM declarations.
import type * as evmole from "./evmole.js" with { "resolution-mode": "import" };
export type { Block, DynamicJump, ControlFlowGraph, Contract, ContractFunction, StorageRecord, CborMetadata, CborEntry, CborValue } from "./evmole.js" with { "resolution-mode": "import" };
export declare const contractInfo: typeof evmole.contractInfo;
