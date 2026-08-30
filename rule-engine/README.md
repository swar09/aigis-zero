# rule-engine

Stream-processing detection microservice. Evaluates normalized endpoint events against pure-Rust YARA-X detection rules, enriches detections with MITRE ATT&CK tactics, suppresses duplicate alerts, and dual-sinks alerts to PostgreSQL (`edr_alerts`) and Kafka (`aigis.alerts`).

## features

- Pure-Rust YARA-X engine evaluation with zero C dependencies
- In-memory MITRE ATT&CK taxonomy indexing technique descriptions, tactics, and threat scores
- 16-bucket sharded LRU deduplicator preventing SOC alert flooding during high-volume event bursts
- Atomic SIGHUP hot-reload for updating detection rules without service restart
- Dual alert sinks: transactional persistence to PostgreSQL via `diesel-async` and broadcast to Kafka topic `aigis.alerts`
- Axum HTTP health probes and Prometheus metrics exporter on port `8081`

## rule directory layout

Rules are organized in `/etc/aigis/rules` or the local `rules/` directory:

```
rules/
  ├── custom/      # Proprietary or site-specific YARA rules
  ├── mitre/       # MITRE Enterprise ATT&CK STIX taxonomy JSON
  └── open-source/ # Downloaded community YARA signatures
```

To download community signatures and the latest MITRE STIX database:

```bash
./scripts/fetch-rules.sh
```

## configuration

```bash
DATABASE_URL=postgres://edr:edr_dev_password@localhost:5434/edr_alerts
KAFKA_BROKERS=localhost:29092
KAFKA_TOPICS=aigis.events.process,aigis.events.network,aigis.events.file,aigis.events.auth
KAFKA_ALERTS_TOPIC=aigis.alerts
RULES_DIR=./rules
PORT=8081
RUST_LOG=info
```

## running locally

```bash
# Start rule engine service
cargo run -p edr-rule-engine

# Trigger hot-reload of rules
kill -HUP $(pgrep edr-rule-engine)
```

## health & metrics endpoints

- `GET /health/live` or `GET /healthz`: returns `OK` (200)
- `GET /health/ready` or `GET /readyz`: returns `READY` (200)
- `GET /metrics`: exposes Prometheus metrics:
  - `aigis_events_consumed_total`
  - `aigis_events_scanned_total`
  - `aigis_alerts_generated_total`
  - `aigis_alerts_suppressed_total`
  - `aigis_alerts_persisted_total`
