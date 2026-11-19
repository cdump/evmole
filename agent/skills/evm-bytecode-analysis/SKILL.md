---
name: evm-bytecode-analysis
description: Analyze supplied deployed EVM runtime bytecode with EVMole or guide a separate application in integrating a published EVMole Rust, Go, Python, or JavaScript binding. Use for unverified-contract inspection, ABI reconstruction from runtime code, selector discovery, storage-access analysis, proxy detection, EVM control-flow inspection, and external application integration. Do not use for implementing, debugging, refactoring, or reviewing EVMole's own source code unless the task specifically targets the CLI, MCP server, agent adapter, schemas, or this skill. Do not use for calldata decoding, source verification, creation-bytecode execution, or full source-code decompilation.
---

# EVM Bytecode Analysis

Use EVMole for deterministic, local extraction from deployed EVM runtime
bytecode.

## Route by intent

- To answer a question from supplied runtime bytecode, use the analysis workflow
  below. Prefer the MCP tool when available and otherwise use the JSON CLI.
- To add published EVMole to a separate application's code, use the native
  library for the project's language instead of invoking the MCP server or
  agent JSON adapter from product code. Read
  [references/libraries.md](references/libraries.md) for package names, entry
  points, and documentation.
- To implement, debug, refactor, or review EVMole itself, follow that
  repository's contributor instructions and inspect its source. Do not use a
  published package, MCP server, or JSON CLI as a substitute for understanding
  the implementation. Read this skill only when the repository task targets
  the CLI, MCP server, agent adapter, schemas, interpretation behavior, or the
  skill itself.
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
   - `proxy` for where a forwarding proxy's implementation address comes from.
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

- Treat call arguments, mutability, storage types, storage labels, and proxy
  sources as inferred. Never present them as verified source-level facts.
- For `Address`, report the embedded implementation address. For `Eip7702`,
  report the delegate address encoded in the designator. For `Storage`,
  `Call`, and `PerSelector`, describe the slot, lookup call, or dependence on
  the incoming selector. The other sections describe the supplied runtime.
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
