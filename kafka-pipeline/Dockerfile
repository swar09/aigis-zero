FROM rust:1.80 as builder
WORKDIR /usr/src/app
RUN apt-get update && apt-get install -y cmake g++
COPY . .
RUN cd kafka-pipeline && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/edr-kafka-pipeline /usr/local/bin/edr-kafka-pipeline
COPY --from=builder /usr/src/app/target/release/kafka-admin /usr/local/bin/kafka-admin
CMD ["edr-kafka-pipeline"]
