FROM rust:1.75
WORKDIR /app
COPY . .
RUN cargo install --path .
ENTRYPOINT ["main"]
