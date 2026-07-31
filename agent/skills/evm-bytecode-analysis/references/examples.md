# Routing examples

## Supported requests

For “Recover likely ABI entries from this deployed bytecode,” request
`selectors`, `arguments`, and `stateMutability`. Read the combined result from
`analysis.functions`, and report selectors, inferred argument lists, dispatch
kind, and inferred mutability without inventing names.

For “Which storage slots does this runtime touch?”, request `storage` and
`transientStorage`. Distinguish the two domains and qualify type labels as
inferred.

For “Extract the compiler metadata,” request `metadata`. A `null` result means
no valid terminal metadata was decoded; it does not identify the compiler.

For “Explain the control-flow structure,” begin with `basicBlocks` and
`controlFlowGraph`. Add `disassembly` only when instruction-level evidence is
needed. Summarize branches and destinations instead of dumping all blocks.

## Malformed input

For empty, odd-length, or non-hexadecimal input, relay the canonical error and
ask for valid deployed runtime bytecode. An address is not bytecode.

## Unsupported requests

- Calldata or transaction decoding requires a calldata/ABI decoder.
- Fetching an address requires an authorized RPC or chain-data tool.
- Solidity source verification requires a compiler and verification workflow.
- Exact source reconstruction or full decompilation is outside EVMole's scope.
- Non-EVM binaries require a matching architecture-specific analyzer.
