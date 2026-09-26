FROM rust:1.71 as builder
WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN apt-get update && apt-get install -y musl-tools pkg-config libssl-dev ca-certificates
RUN rustup target add x86_64-unknown-linux-musl || true

RUN cargo build --release

FROM debian:buster-slim
COPY --from=builder /app/target/release/SSO /usr/local/bin/SSO
EXPOSE 3000
CMD ["/usr/local/bin/SSO"]
