# Stage 1: Build
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
RUN cargo build --release -p edr-rule-engine

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    libssl3 libpq5 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1001 -s /bin/bash edr

COPY --from=builder /build/target/release/edr-rule-engine /usr/local/bin/
RUN chmod +x /usr/local/bin/edr-rule-engine

USER edr

ENTRYPOINT ["edr-rule-engine"]
