# syntax=docker/dockerfile:1.7

# Stage 1: build
FROM rust:1.85-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY fleet-server/crates ./fleet-server/crates
COPY sdk ./sdk

# For CI: set SQLX_OFFLINE=true and commit .sqlx/ directory.
# For local dev with live DB: set SQLX_OFFLINE=false.
ARG SQLX_OFFLINE=true
ENV SQLX_OFFLINE=${SQLX_OFFLINE}

RUN cargo build --release -p fleet-server-bin

# Stage 2: runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/fleet-server-bin /usr/local/bin/fleet-server

EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/fleet-server"]
