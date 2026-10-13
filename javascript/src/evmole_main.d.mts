// Types for the package root ("evmole"), which only exports contractInfo.
// dist/evmole.d.ts also declares the wasm init functions used by "evmole/no_tla".
export { contractInfo } from "./evmole.js";
export type { Block, DynamicJump, ControlFlowGraph, Contract, ContractFunction, StorageRecord, CborMetadata, CborEntry, CborValue } from "./evmole.js";
