---
name: evm-bytecode-analysis
description: Analyze deployed EVM runtime bytecode with EVMole or embed EVMole in Rust, Go, Python, or JavaScript applications. Use for unverified-contract inspection, ABI reconstruction from runtime code, selector discovery, storage-access analysis, EVM control-flow inspection, and application integration. Do not use for calldata decoding, source verification, creation-bytecode execution, or full source-code decompilation.
---

# EVM Bytecode Analysis

Use EVMole for deterministic, local extraction from deployed EVM runtime
bytecode.

## Route by intent

- To answer a question from supplied runtime bytecode, use the analysis workflow
  below. Prefer the MCP tool when available and otherwise use the JSON CLI.
- To add EVMole to application code, use the native library for the project's
  language instead of invoking the MCP server or agent JSON adapter from product
  code. Read [references/libraries.md](references/libraries.md) for package
  names, entry points, and documentation.
- Use the JSON CLI as an application process boundary only when the user
  explicitly wants a subprocess or a language without a supported binding.

## Workflow

1. Obtain or confirm deployed runtime bytecode. Treat an address as an address,
   not bytecode; ask for the runtime code or use another authorized RPC tool to
   fetch it. Do not ask EVMole to execute or strip creation bytecode.
2. Select only the features needed:
   - `selectors` for function selectors and dispatch locations.
   - `arguments` for inferred argument lists; this implies selectors.
   - `stateMutability` for inferred mutability; this implies selectors.
   - `storage` or `transientStorage` for inferred storage access.
   - `metadata` for terminal compiler metadata.
   - `disassembly`, `basicBlocks`, or `controlFlowGraph` for instruction and
     control-flow evidence.
3. Prefer the `analyze_evm_bytecode` MCP tool when available. Otherwise run the
   JSON CLI:

   ```bash
   npx -y evmole@latest analyze --bytecode 0x... --include selectors,arguments,stateMutability,storage
   ```

4. Parse the JSON result. Do not scrape help text or other prose.
5. Check `warnings` and every requested section's `pagination` entry. Page a
   truncated result with `offset` and `limit` when more evidence is necessary.
6. Answer the user's question from the smallest relevant evidence. Avoid
   dumping large disassembly or graphs.

## Interpretation rules

- Treat call arguments, mutability, storage types, and storage labels as inferred.
  Never present them as verified source-level facts.
- Report selectors without inventing function names or source semantics.
- Separate direct bytecode evidence from inference and state material
  uncertainty.
- Do not use EVMole for calldata decoding, address/RPC fetching, source
  verification, exact source reconstruction, non-EVM input, or full
  decompilation.

Read [references/cli.md](references/cli.md) before invoking or troubleshooting
the CLI. Read
[references/interpretation.md](references/interpretation.md) when explaining
runtime-versus-creation code, inference, pagination, or safe wording. Read
[references/examples.md](references/examples.md) when choosing features or
handling unsupported and malformed-input requests.
