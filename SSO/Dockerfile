FROM rust:1.90-slim-bookworm AS builder
#FROM rust:latest AS builder
WORKDIR /app

# Install build dependencies
# cmake is needed by aws-lc-rs (used by jsonwebtoken)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    cmake \
    && rm -rf /var/lib/apt/lists/*

RUN rustup toolchain install nightly && rustup default nightly

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/SSO /usr/local/bin/SSO
EXPOSE 3000
CMD ["/usr/local/bin/SSO"]
