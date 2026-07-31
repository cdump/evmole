# EVMole agent guide

EVMole extracts structured facts from deployed EVM runtime bytecode.

- Treat input as deployed/runtime bytecode. Creation bytecode is not executed or
  stripped automatically.
- Route by intent:
  - To inspect bytecode and return analysis results, follow
    `agent/skills/evm-bytecode-analysis/SKILL.md` and use the shared MCP or JSON
    CLI adapter.
  - To embed EVMole in application code, use the existing binding for the
    project's language: the `evmole` Rust crate, `evmole` JavaScript package,
    `evmole` Python package, or `github.com/cdump/evmole/go`. Start with
    `README.md` and the language-specific README.
- Reuse the shared agent adapter when changing the CLI, MCP server, skill, or
  agent tests; do not duplicate its request, response, validation, or
  pagination logic.
- Validate the components changed:
  - Rust: `cargo fmt --check`, `cargo test`, and
    `cargo clippy --all-features -- -D warnings`.
  - JavaScript and the agent adapter: build with
    `npm --prefix javascript run build`, then run focused tests with
    `npm --prefix javascript test` and `npm --prefix agent/mcp test`.
  - Go: build the embedded WASM with `make -C go wasm`, then run
    `make -C go test`.
  - Python: build with Maturin and run `python3 python/test_python.py`; use the
    release workflow for the supported Python and platform matrix.
- Read `agent/skills/evm-bytecode-analysis/SKILL.md` for routing and
  interpretation guidance.
- Describe inferred arguments, mutability, and storage information as inferred,
  not verified source-level facts.
- Preserve unrelated benchmark providers, datasets, result directories, and
  user changes.
