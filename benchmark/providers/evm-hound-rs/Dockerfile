FROM rust:1.74
WORKDIR /app
COPY . .
RUN cargo install --path .
ENTRYPOINT ["main"]
