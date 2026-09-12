# syntax=docker/dockerfile:1.7
FROM rust:1-slim-bookworm AS builder

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
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release -p edr-api-backend && \
    cp /build/target/release/edr-api-backend /usr/local/bin/edr-api-backend

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1001 edr && \
    useradd -u 1001 -g edr -s /usr/sbin/nologin -M edr

COPY --from=builder --chmod=0755 /usr/local/bin/edr-api-backend /usr/local/bin/edr-api-backend

USER edr:edr
EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://127.0.0.1:8080/healthz || exit 1

ENTRYPOINT ["/usr/local/bin/edr-api-backend"]
