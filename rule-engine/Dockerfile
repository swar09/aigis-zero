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
    python3 \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN if [ ! -f rule-engine/rules/mitre/enterprise-attack-linux.json ]; then \
      ./scripts/fetch-rules.sh; \
    fi

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release -p edr-rule-engine && \
    cp /build/target/release/edr-rule-engine /usr/local/bin/edr-rule-engine

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1001 edr && \
    useradd -u 1001 -g edr -s /usr/sbin/nologin -M edr

COPY --chown=edr:edr rule-engine/rules /etc/aigis/rules
COPY --from=builder --chmod=0755 /usr/local/bin/edr-rule-engine /usr/local/bin/edr-rule-engine

USER edr:edr
EXPOSE 8081

HEALTHCHECK --interval=10s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://127.0.0.1:8081/healthz || exit 1

ENTRYPOINT ["/usr/local/bin/edr-rule-engine"]
