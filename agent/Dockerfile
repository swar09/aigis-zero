# Agent Dockerfile — for testing only, real deployment is a static binary
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev cmake clang llvm \
    linux-headers-generic \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
RUN cargo build --release --bin agent-core
