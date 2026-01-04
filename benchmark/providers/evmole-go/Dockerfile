# syntax=docker/dockerfile:1

FROM rust:1.92 AS build-wasm
RUN rustup target add wasm32-unknown-unknown
COPY ./rust /rust
RUN cargo build --target wasm32-unknown-unknown --release --features wasm --manifest-path /rust/Cargo.toml

FROM golang:1.25
WORKDIR /app
COPY . .
COPY --from=build-wasm /rust/target/wasm32-unknown-unknown/release/evmole.wasm /tmp/evmole.wasm
RUN gzip -c /tmp/evmole.wasm > evmole/wasm/evmole.wasm.gz
RUN go mod tidy && go build -o main .
ENTRYPOINT ["./main"]
