# Application library reference

Use a native EVMole binding when the user wants to integrate bytecode analysis
into application code. Reserve the MCP tool and agent JSON adapter for
agent-driven analysis, and use the JSON CLI in an application only when the
user explicitly wants a subprocess or no native binding is available.

## Rust

- Package: `evmole`
- Install: `cargo add evmole`
- Main entry point: `evmole::contract_info` with
  `evmole::ContractInfoArgs`
- Documentation: <https://docs.rs/evmole/latest/evmole/>

## JavaScript and TypeScript

- Package: `evmole`
- Install: `npm install evmole`
- Main entry point: `contractInfo`
- Repository documentation:
  <https://github.com/cdump/evmole/tree/master/javascript>

The default package supports Node.js and modern browser bundlers. Consult the
JavaScript documentation for CommonJS, top-level-await, and bundler-specific
entry points instead of creating a new WASM loader.

## Python

- Package: `evmole`
- Install: `python -m pip install --upgrade evmole`
- Main entry point: `evmole.contract_info`
- Repository documentation:
  <https://github.com/cdump/evmole/tree/master/python>

## Go

- Module: `github.com/cdump/evmole/go`
- Install: `go get github.com/cdump/evmole/go`
- Main entry points: `evmole.ContractInfo` for one analysis or
  `evmole.NewAnalyzer` for efficient reuse
- Documentation: <https://pkg.go.dev/github.com/cdump/evmole/go>

## Shared interpretation

All bindings analyze deployed/runtime bytecode. Creation bytecode is not
executed or stripped automatically. Treat inferred arguments, state mutability,
storage types, and storage labels as inferred rather than verified source-level
facts.
