# Stage 1: Build
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/edr_*

COPY src ./src
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    libssl3 libpq5 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1001 -s /bin/bash edr

COPY --from=builder /build/target/release/edr-kafka-pipeline /usr/local/bin/
RUN chmod +x /usr/local/bin/edr-kafka-pipeline

USER edr

ENTRYPOINT ["edr-kafka-pipeline"]
