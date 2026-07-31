# evmole-mcp

`evmole-mcp` is the local MCP server for
[EVMole](https://github.com/cdump/evmole), a structured analyzer for deployed
EVM runtime bytecode. It exposes one tool, `analyze_evm_bytecode`, for use by
MCP-compatible agents and clients.

## Start the server

```bash
npx -y evmole-mcp@latest
```

Node.js 22 or newer is required.

## Configure an MCP client

```json
{
  "mcpServers": {
    "evmole": {
      "command": "npx",
      "args": ["-y", "evmole-mcp@latest"]
    }
  }
}
```

The server communicates over stdio.

## Analyze runtime bytecode

The `analyze_evm_bytecode` tool accepts hexadecimal `bytecode`, optional
`include` feature names, and optional `offset` and `limit` values. It can
extract:

- function selectors and dispatch locations;
- inferred arguments and state mutability;
- inferred persistent and transient storage access;
- compiler metadata;
- disassembly, basic blocks, and a control-flow graph.

Function selectors can be requested on their own with `selectors`. Requesting
`arguments` or `stateMutability` also enables selector extraction, and the
requested function information is combined in `analysis.functions`. Other
requested analyses are returned in their corresponding `analysis` fields.

If `include` is omitted, the tool analyzes selectors, arguments, state
mutability, both storage domains, and metadata. Disassembly and control-flow
features are opt-in because their output can be large. The tool returns the
canonical version 1 EVMole response as structured content plus a JSON text
fallback. Check `warnings` and `pagination` before interpreting the result.

## Privacy and limitations

The server uses stdio only, makes no network requests, requires no credentials,
does not fetch contract addresses, and writes no files. It does not decode
calldata, verify source, execute creation bytecode, or fully decompile source.
Arguments, mutability, and storage descriptions are inferred rather than
verified source-level facts.

Each `evmole-mcp` release depends on the matching exact `evmole` version.

See <https://github.com/cdump/evmole/tree/master/agent> for full setup and
privacy guidance. Report problems at <https://github.com/cdump/evmole/issues>.
