# syntax=docker/dockerfile:1.7

# Stage 1: Build
FROM rust:1.85-slim AS builder

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

RUN cargo build --release -p edr-rule-engine

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1001 -s /bin/bash edr

RUN mkdir -p /etc/aigis/rules && chown -R edr:edr /etc/aigis
COPY --chown=edr:edr rule-engine/rules /etc/aigis/rules

COPY --from=builder /build/target/release/edr-rule-engine /usr/local/bin/edr-rule-engine
RUN chmod +x /usr/local/bin/edr-rule-engine

USER edr
EXPOSE 8081

HEALTHCHECK --interval=10s --timeout=5s --retries=3 \
  CMD ["curl", "-f", "http://localhost:8081/health/ready"]

ENTRYPOINT ["/usr/local/bin/edr-rule-engine"]
