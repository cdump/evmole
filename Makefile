.PHONY: staticbuild
staticbuild: staticbuild-darwin staticbuild-windows staticbuild-linux

.PHONY: staticbuild-darwin
staticbuild-darwin:
	rustup target add x86_64-apple-darwin
	cargo zigbuild --release --target x86_64-apple-darwin --features c_api

	rustup target add aarch64-apple-darwin
	cargo zigbuild --release --target aarch64-apple-darwin --features c_api

.PHONY: staticbuild-windows
staticbuild-windows:
	rustup target add x86_64-pc-windows-gnu
	cargo zigbuild --release --target x86_64-pc-windows-gnu --features c_api

.PHONY: staticbuild-linux
staticbuild-linux:
	rustup target add x86_64-unknown-linux-musl
	cargo zigbuild --release --target x86_64-unknown-linux-musl --features c_api

	rustup target add aarch64-unknown-linux-musl
	cargo zigbuild --release --target aarch64-unknown-linux-musl --features c_api

.PHONY: gobuild
gobuild: staticbuild
	mkdir -p go/staticlibs/linux-amd64
	mkdir -p go/staticlibs/linux-arm64
	cp target/x86_64-unknown-linux-musl/release/libevmole.a go/staticlibs/linux-amd64/libevmole.a
	cp target/aarch64-unknown-linux-musl/release/libevmole.a go/staticlibs/linux-arm64/libevmole.a

	mkdir -p go/staticlibs/darwin-amd64
	mkdir -p go/staticlibs/darwin-arm64
	cp target/x86_64-apple-darwin/release/libevmole.a go/staticlibs/darwin-amd64/libevmole.a
	cp target/aarch64-apple-darwin/release/libevmole.a go/staticlibs/darwin-arm64/libevmole.a

	mkdir -p go/staticlibs/windows-amd64
	cp target/x86_64-pc-windows-gnu/release/libevmole.a go/staticlibs/windows-amd64/libevmole.a
