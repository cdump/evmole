.PHONY: build test bench clean wasm

# Build WASM module and copy to go/wasm/
wasm:
	cargo build --release --target wasm32-unknown-unknown --features wasm --manifest-path ../Cargo.toml
	gzip -9 -c ../target/wasm32-unknown-unknown/release/evmole.wasm > wasm/evmole.wasm.gz

# Run tests
test:
	go test -v ./...

# Run benchmarks
bench:
	go test -bench=. -benchmem ./...

# Build Go package (verify compilation)
build:
	go build ./...

# Clean build artifacts
clean:
	rm -f wasm/evmole.wasm.gz
	go clean ./...

# Full rebuild: build WASM and run tests
all: wasm test
