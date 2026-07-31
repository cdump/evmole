# CLI reference

## Requirements and invocation

Use Node.js 22 or newer. A global installation is optional:

```bash
npm install -g evmole@latest
evmole analyze --bytecode 0x6001600055
```

Prefer the latest published release when the executable is not installed:

```bash
npx -y evmole@latest analyze --bytecode 0x6001600055
echo '0x6001600055' | npx -y evmole@latest analyze
npx -y evmole@latest analyze --file runtime.hex --include selectors,arguments,stateMutability,storage
```

Use `--bytecode` or `--file` for explicit input. When neither option is present,
the CLI reads piped stdin. Explicit input does not consume stdin, and the file
option reads only the explicitly supplied local file.

## Options and features

- `--include <comma-separated-features>` selects `selectors`, `arguments`,
  `stateMutability`, `storage`, `transientStorage`, `metadata`, `disassembly`,
  `basicBlocks`, or `controlFlowGraph`.
- `arguments` and `stateMutability` imply selectors. Requested function data
  is returned in the combined `analysis.functions` array.
- `--offset <integer>` starts list-like sections at an offset.
- `--limit <integer>` returns at most that many items per requested list-like
  section. The default is 1,000 and maximum is 5,000.
- `--pretty` formats JSON for humans.
- `evmole schema` prints the version 1 request, response, and error schemas.
- `evmole --version` and `evmole --help` report package information.

The default includes selectors, arguments, state mutability, persistent
storage, transient storage, and metadata. Disassembly and control-flow
features are opt-in.

## Output and recovery

An analysis writes exactly one JSON document to stdout. Exit codes are:

- `0`: success.
- `2`: invalid CLI syntax or ambiguous input.
- `3`: invalid bytecode or request.
- `4`: bytecode or configured output limit exceeded.
- `1`: unexpected analysis/internal failure.

If the executable is missing, verify Node.js 22+, npm registry access, and the
package name `evmole`. If bytecode is invalid, remove surrounding whitespace,
verify an even number of hexadecimal digits, and confirm the value is deployed
runtime bytecode rather than an address. Parse the JSON error's `code`,
`message`, and optional `details`; do not expose local dependency exceptions.
