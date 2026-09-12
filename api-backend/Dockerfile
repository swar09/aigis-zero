# syntax=docker/dockerfile:1.7

# ==============================================================================
# Stage 1: Build (Rust 1.85 Bookworm Slim with BuildKit Cache Mounts)
# ==============================================================================
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libpq-dev \
    protobuf-compiler \
    cmake \
    g++ \
    make \
    libcurl4-openssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy Cargo workspace manifests and sources (filtered by .dockerignore)
COPY . .

# Build with persistent cargo registry & target cache mounts
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release -p edr-api-backend && \
    cp /build/target/release/edr-api-backend /usr/local/bin/edr-api-backend

# ==============================================================================
# Stage 2: Hardened Runtime (Debian Bookworm Slim, Non-Root)
# ==============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Run as dedicated unprivileged user
RUN groupadd -g 1001 edr && \
    useradd -u 1001 -g edr -s /usr/sbin/nologin -M edr

COPY --from=builder --chmod=0755 /usr/local/bin/edr-api-backend /usr/local/bin/edr-api-backend

USER edr:edr
EXPOSE 8080

# Production health check with startup grace period to avoid flapping on DB connection
HEALTHCHECK --interval=10s --timeout=5s --start-period=10s --retries=3 \
  CMD ["curl", "-f", "http://127.0.0.1:8080/healthz"]

ENTRYPOINT ["/usr/local/bin/edr-api-backend"]
