# Rule Engine deployment and operations guide

This guide covers the architecture, YARA-X detection rules, MITRE ATT&CK taxonomy enrichment, LRU alert deduplication, and production operations for the Aigis-Zero Rule Engine (`edr-rule-engine`).

## 1. System architecture

The Rule Engine evaluates normalized endpoint telemetry streams against compiled YARA-X rules, enriches detections with MITRE ATT&CK metadata, deduplicates alert storms, and writes alerts to both PostgreSQL and Kafka.

```mermaid
graph TD
    subgraph Inputs ["Inbound Telemetry Topics"]
        T_Proc["aigis.events.process"]
        T_Net["aigis.events.network"]
        T_File["aigis.events.file"]
        T_Auth["aigis.events.auth"]
    end

    subgraph Engine ["edr-rule-engine (:8081)"]
        Consumer["Kafka StreamConsumer (Group: aigis-rule-engine)"]
        Parser["JSON Event Normalizer"]
        YARA["YARA-X Scanner Engine<br/>(Pure-Rust in-memory rules)"]
        Taxonomy["MITRE ATT&CK STIX Indexer<br/>(enterprise-attack-linux.json)"]
        Dedup["16-Bucket Sharded LRU Deduplicator<br/>(60s time window)"]
        Sink["Dual Alert Sink Dispatcher"]
        Health["Axum Health & Metrics (:8081)"]
    end

    subgraph Sinks ["Alert Persistence & Broadcast"]
        DB_Alerts[("PostgreSQL edr_alerts :5434")]
        K_Alerts{{"Kafka topic: aigis.alerts"}}
    end

    Inputs --> Consumer
    Consumer --> Parser
    Parser --> YARA
    YARA -->|"Rule match + tags"| Taxonomy
    Taxonomy -->|"Enriched alert"| Dedup
    Dedup -->|"Non-duplicate alerts"| Sink
    Sink -->|"diesel-async INSERT"| DB_Alerts
    Sink -->|"rdkafka produce"| K_Alerts
```

### Key capabilities

1. Pure-Rust scanning: Uses the official YARA-X engine, eliminating C library dependencies and memory safety issues in detection scanning.
2. In-memory MITRE ATT&CK indexing: Loads `enterprise-attack-linux.json` into memory on boot. Enriches every rule match with formal technique names, tactic categories, and severity ratings.
3. Sharded LRU deduplication: Uses a 16-bucket lock-sharded LRU cache to suppress duplicate alerts from recurring event loops within a 60-second window.
4. Atomic hot-reload: Listens for `SIGHUP` signals. Compiles new rules from disk in the background and atomically swaps the rule pointer with zero dropped events and zero process downtime.
5. Dual alert sinks: Persists every alert into PostgreSQL `edr_alerts` for historical querying while broadcasting to Kafka `aigis.alerts` for WebSocket live streams.

## 2. Configuration reference

Set environment variables in `.env` or systemd environment files:

```bash
# PostgreSQL alert database
DATABASE_URL=postgres://edr:edrpassword_change_in_production@localhost:5434/edr_alerts
DB_POOL_MAX_SIZE=16

# Kafka brokers and topics
KAFKA_BROKERS=localhost:9092
KAFKA_GROUP_ID=aigis-rule-engine
KAFKA_TOPICS=aigis.events.process,aigis.events.network,aigis.events.file,aigis.events.auth
ALERTS_TOPIC=aigis.alerts
DLQ_TOPIC=aigis.events.dlq

# Rule definitions and MITRE taxonomy
RULES_DIR=./rule-engine/rules
MITRE_TAXONOMY_PATH=./rule-engine/rules/mitre/enterprise-attack-linux.json

# Performance and deduplication parameters
SCANNER_WORKERS=8
CHANNEL_CAPACITY=10000
DEDUP_CAPACITY=100000
DEDUP_WINDOW_SECS=60
BATCH_MAX_SIZE=500
BATCH_FLUSH_MS=100

# Health probe and Prometheus metrics port
HEALTH_PORT=8081

# Observability
RUST_LOG=info,edr_rule_engine=debug
```

## 3. Rule authoring and MITRE ATT&CK mapping

Rules are stored in `rule-engine/rules/`:
- `rules/custom/`: Site-specific and proprietary detection rules.
- `rules/mitre/`: Enterprise ATT&CK Linux STIX taxonomy JSON.
- `rules/open-source/`: Community detection signatures downloaded via `./scripts/fetch-rules.sh`.

### Authoring a YARA-X detection rule

Rules match on the serialized JSON representation of incoming events. Example rule detecting reverse shell execution:

```yara
rule Suspicious_Netcat_Reverse_Shell
{
    meta:
        description = "Detects netcat executed with interactive shell redirection"
        severity = "critical"
        mitre_tactic = "Execution"
        mitre_technique = "Command and Scripting Interpreter"
        technique_id = "T1059.004"
        author = "Aigis-Zero Security Operations"

    strings:
        $nc1 = "nc -e /bin/sh" ascii wide
        $nc2 = "nc -e /bin/bash" ascii wide
        $nc3 = "nc.traditional -e" ascii wide
        $nc4 = "ncat -e" ascii wide

    condition:
        any of ($nc*)
}
```

When this rule matches:
1. YARA-X extracts the matched rule name and metadata tags.
2. The engine resolves `technique_id = "T1059.004"` against the in-memory MITRE index, attaching tactic definitions and description summaries.
3. An alert payload is formed and passed to the deduplicator.

## 4. Atomic SIGHUP rule hot-reload

To update or add detection rules without restarting the service or disconnecting Kafka consumers:

1. Add or edit `.yar` files in the configured `RULES_DIR`.
2. Send `SIGHUP` to the process:
   ```bash
   kill -HUP $(pgrep edr-rule-engine)
   ```
3. The engine parses and compiles the new rule files into a new `yara_x::Rules` instance.
4. If compilation succeeds, the active pointer is atomically updated and a log line confirms: `Successfully reloaded YARA-X detection rules`.
5. If compilation fails, the engine logs the syntax error and continues scanning with the previous rule set.

## 5. Health probes and Prometheus metrics

The service exposes HTTP endpoints on port `8081`:

- `GET /healthz` or `GET /health/live`: Process liveness probe. Returns HTTP 200 `OK`.
- `GET /readyz` or `GET /health/ready`: Readiness probe verifying active database and Kafka connections. Returns HTTP 200 `READY`.
- `GET /metrics`: Exposes Prometheus metrics:
  - `aigis_events_consumed_total`: Messages consumed from typed topics.
  - `aigis_events_scanned_total`: Individual evaluations executed by worker threads.
  - `aigis_alerts_generated_total`: Detections triggered before deduplication.
  - `aigis_alerts_suppressed_total`: Repeated alerts suppressed by the LRU window.
  - `aigis_alerts_persisted_total`: Alerts committed to PostgreSQL and Kafka.

Check readiness:
```bash
curl -s http://localhost:8081/readyz
```

## 6. Production deployment

### Docker Compose deployment

```bash
cd infra
docker compose up -d rule-engine
```

### Pulling and running pre-built Docker image

Pull the published image from GitHub Container Registry:

```bash
docker pull ghcr.io/swar09/aigis-rule-engine:latest
```

Run standalone container with rule mount and environment file:

```bash
docker run -d \
  --name edr-rule-engine \
  --restart unless-stopped \
  -p 8081:8081 \
  -v ./rule-engine/rules:/etc/aigis/rules:ro \
  --env-file .env \
  ghcr.io/swar09/aigis-rule-engine:latest
```

### Systemd deployment

Create `/etc/systemd/system/edr-rule-engine.service`:

```ini
[Unit]
Description=Aigis-Zero YARA-X Rule Engine
After=network.target

[Service]
Type=simple
User=edr
Group=edr
EnvironmentFile=/etc/aigis-zero/rule-engine.env
ExecStart=/usr/local/bin/edr-rule-engine
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=5
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now edr-rule-engine
sudo systemctl status edr-rule-engine
```

To reload rules via systemd:
```bash
sudo systemctl reload edr-rule-engine
```

## 7. Operational troubleshooting

### Rule compilation failure on startup
- Log message: `failed to compile rules in directory`.
- Cause: YARA rule syntax error in one of the `.yar` files.
- Fix: Validate rules locally by running `yara-x compile <rule_file>` or checking service logs for the exact line number and error message.

### High alert suppression count
- Symptom: `aigis_alerts_suppressed_total` increases rapidly while SOC analysts report missing alerts.
- Cause: `DEDUP_WINDOW_SECS` is too high for testing, or an endpoint has a runaway process that triggers repeatedly.
- Fix: Default window is 60 seconds. Lower to 10 seconds in development or tune specific noisy rules.

### Database insert timeouts
- Symptom: Warnings in log `failed to persist alert to edr_alerts`.
- Cause: Database connection pool checkout timeout.
- Fix: Check PostgreSQL container on port 5434. Verify `DATABASE_URL` credentials and check query latency with `pg_stat_activity`.
