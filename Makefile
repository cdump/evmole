.PHONY: wasm
wasm:
	cargo build --target wasm32-unknown-unknown --features wasm --release
	cp target/wasm32-unknown-unknown/release/evmole.wasm go/evmole.wasm
