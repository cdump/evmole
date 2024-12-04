# syntax=docker/dockerfile:1

FROM rust:1.83
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=./.cargo \
    --mount=type=cache,target=./target \
    CARGO_HOME=/app/.cargo \
    cargo install --locked --root=. --path .
ENTRYPOINT ["./bin/main"]
