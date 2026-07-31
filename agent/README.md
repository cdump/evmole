# EVMole agent integration

EVMole gives coding agents deterministic, structured facts from deployed EVM
runtime bytecode. Use the JSON CLI for scripts and one-shot analysis, install
the portable skill for automatic routing and interpretation guidance, use the
local MCP server for typed tool calls.

Node.js 22 or newer is supported.

## JSON CLI

Analyze a command-line value, standard input, or one explicitly named file:

```bash
npx -y evmole@latest analyze --bytecode 0x6001600055
echo '0x6001600055' | npx -y evmole@latest analyze
npx -y evmole@latest analyze --file runtime.hex --include selectors,arguments,stateMutability,storage
```

Explicit `--bytecode` and `--file` input does not consume standard input. When
neither option is present, the CLI reads piped stdin.

Use `--offset` and `--limit` to page list-like output, `--pretty` for
human-readable JSON, `evmole schema` for the version 1 contracts, and
`evmole --help` for all options. Exit code 0 means success, 2 means invalid CLI
usage, 3 means invalid bytecode/request data, 4 means an input or output bound
was exceeded, and 1 means an unexpected analysis failure. Analyze failures are
written as one JSON error document on stdout.

Supported features are `selectors`, `arguments`, `stateMutability`, `storage`,
`transientStorage`, `metadata`, `disassembly`, `basicBlocks`, and
`controlFlowGraph`. Arguments and state mutability imply selectors; requested
function data is combined into `analysis.functions` records. The default
requests selectors, arguments, state mutability, both storage domains, and
metadata. Disassembly and control-flow output are opt-in because they can be
large. Input must be deployed/runtime bytecode; EVMole does not execute or
strip creation bytecode.

The canonical contracts are
[request v1](schemas/request-v1.schema.json),
[response v1](schemas/response-v1.schema.json), and
[error v1](schemas/error-v1.schema.json).

## Portable skill

List or install the canonical repository skill:

```bash
npx skills add cdump/evmole --list
npx skills add cdump/evmole --skill evm-bytecode-analysis -g
```

Run the same install command again to update it. Remove it with:

```bash
npx skills remove evm-bytecode-analysis -g -y
```

## Local MCP server

Start the stdio server with:

```bash
npx -y evmole-mcp@latest
```

Codex CLI and the Codex desktop app share MCP configuration. Add the local
server with:

```bash
codex mcp add evmole -- npx -y evmole-mcp@latest
codex mcp list
```

The equivalent `config.toml` entry is:

```toml
[mcp_servers.evmole]
command = "npx"
args = ["-y", "evmole-mcp@latest"]
```

Claude Code can add the same stdio command with:

```bash
claude mcp add --transport stdio evmole -- npx -y evmole-mcp@latest
```

OpenCode V2 uses this local-server entry:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "evmole": {
        "type": "local",
        "command": ["npx", "-y", "evmole-mcp@latest"]
      }
    }
  }
}
```

The generic entry for other clients that use the common JSON shape is:

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

Pi users should install the portable skill and use its CLI fallback; Pi does
not currently publish a built-in MCP client configuration. MCP input always
contains bytecode directly; the server does not accept file paths or addresses.
Remove standalone MCP entries with the relevant client's MCP removal command.

## Interpretation and limitations

Selectors are bytecode evidence, but arguments, mutability, and storage types
can be inferred and must not be presented as verified source-level facts.
EVMole does not fetch contract addresses, decode calldata, resolve signature
names, verify source, or reconstruct exact source code. Check every response's
`warnings` and `pagination` objects before summarizing it.

EVMole runs locally. The CLI and MCP package do not send bytecode to an
EVMole-operated service. Your chosen agent or model provider may still receive
prompts and tool results according to that provider's configuration.

If an executable is missing, confirm Node.js 22+, npm access, and the exact
package name. If input is rejected, remove whitespace, confirm an even number
of hexadecimal digits, and verify that the value is runtime bytecode. Report
issues at <https://github.com/cdump/evmole/issues>. Package and registry links
are:

- [`evmole` on npm](https://www.npmjs.com/package/evmole)
- [`evmole-mcp` on npm](https://www.npmjs.com/package/evmole-mcp)
- [Official MCP Registry](https://registry.modelcontextprotocol.io/)
- [`evm-bytecode-analysis` on skills.sh](https://skills.sh/cdump/evmole)
- [GitHub repository](https://github.com/cdump/evmole)
- [Benchmark evidence and reproduction](../README.md#benchmark)
