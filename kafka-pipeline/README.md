# kafka-pipeline

Stream normalization and topic routing service. Consumes raw telemetry from `aigis.events.raw`, inspects event types, and fans records out into typed topics for detection scanning and long-term storage.

## topic topology

```
aigis.events.raw (12 partitions)
       |
       +---> aigis.events.process (12 partitions) -> rule-engine
       +---> aigis.events.network (12 partitions) -> rule-engine
       +---> aigis.events.file    (12 partitions) -> rule-engine
       +---> aigis.events.auth    (12 partitions) -> rule-engine
       +---> aigis.events.dlq     (4 partitions)  -> poison pill records
```

## features

- Type-aware routing based on `event_type` and query metadata
- LZ4 compression with 5ms micro-batching on producer output
- Asynchronous message offset commits preventing replay loops
- Dead-letter-queue error routing with attached Kafka headers (`x-error-reason`, `x-source-topic`, `x-original-partition`, `x-original-offset`)
- Axum HTTP health probes and Prometheus metrics exporter on port `8082`

## configuration

```bash
KAFKA_BROKERS=localhost:29092
HEALTH_PORT=8082
RUST_LOG=info
```

## running locally

```bash
# Provision Kafka topics if needed
./scripts/infra.sh up

# Start the pipeline router
cargo run -p edr-kafka-pipeline

# Run topic administration utility
cargo run -p edr-kafka-pipeline --bin kafka-admin -- list
```

## health & metrics endpoints

- `GET /health/live` or `GET /healthz`: returns `OK` (200)
- `GET /health/ready` or `GET /readyz`: returns `READY` (200)
- `GET /metrics`: exposes Prometheus metrics:
  - `aigis_pipeline_events_consumed_total`
  - `aigis_pipeline_events_routed_total{topic="..."}`
  - `aigis_pipeline_routing_errors_total`
