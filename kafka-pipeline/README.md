# kafka-pipeline

Stream normalization and topic routing service. Consumes raw telemetry from `aigis.events.raw`, inspects event types, and fans records out into typed topics for detection scanning and long-term storage.

For the complete topic routing matrix, DLQ inspection, and deployment runbooks, see the [Kafka Pipeline guide](guide.md).

## Topic topology

```
aigis.events.raw (12 partitions)
       |
       +---> aigis.events.process (12 partitions) -> rule-engine
       +---> aigis.events.network (12 partitions) -> rule-engine
       +---> aigis.events.file    (8 partitions)  -> rule-engine
       +---> aigis.events.auth    (8 partitions)  -> rule-engine
       +---> aigis.events.norm    (12 partitions) -> long-term storage
       +---> aigis.events.dlq     (4 partitions)  -> poison pill records
```

## Features

- Type-aware routing based on `event_type` and query metadata
- LZ4 compression with 5ms micro-batching on producer output
- Asynchronous message offset commits preventing replay loops
- Dead-letter-queue error routing with attached Kafka headers (`x-error-reason`, `x-source-topic`, `x-original-partition`, `x-original-offset`)
- Axum HTTP health probes and Prometheus metrics exporter on port `8082`

## Configuration

```bash
# Use localhost:9092 for host processes, or kafka:29092 inside Docker
KAFKA_BROKERS=localhost:9092
HEALTH_PORT=8082
RUST_LOG=info
```

## Running locally

```bash
# Provision Kafka topics if needed
./scripts/infra.sh up

# Start the pipeline router
cargo run -p edr-kafka-pipeline

# Run topic administration utility
cargo run -p edr-kafka-pipeline --bin kafka-admin -- list
```

## Health & metrics endpoints

- `GET /health/live` or `GET /healthz`: returns `OK` (200)
- `GET /health/ready` or `GET /readyz`: returns `READY` (200)
- `GET /metrics`: exposes Prometheus metrics:
  - `aigis_pipeline_events_consumed_total`
  - `aigis_pipeline_events_routed_total{topic="..."}`
  - `aigis_pipeline_routing_errors_total`

## Additional resources

- [Kafka Pipeline operations guide](guide.md): In-depth setup, routing simulation, and troubleshooting.
- [Topic provisioning script](../infra/scripts/create-topics.sh): Partition definitions and retention policies.
