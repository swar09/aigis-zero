FROM rust:1.85-slim AS builder
WORKDIR /usr/src/app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    cmake \
    g++ \
    make \
    libcurl4-openssl-dev \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release -p edr-kafka-pipeline


FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/edr-kafka-pipeline /usr/local/bin/edr-kafka-pipeline
COPY --from=builder /usr/src/app/target/release/kafka-admin /usr/local/bin/kafka-admin
RUN groupadd --system edr && useradd --system --gid edr edr
USER edr
CMD ["edr-kafka-pipeline"]
