FROM rust:1.76
WORKDIR /app
COPY . .
RUN cargo install --path .
ENTRYPOINT ["main"]
