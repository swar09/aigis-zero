# Changelog

All notable changes to the Aigis-Zero EDR project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to Semantic Versioning.

---

## [Unreleased]

### Added

- **agents**: Virtual engineering team skill suite and orchestration framework in `.agents/engineering-team/` with subagent delegation protocols and automation tooling
- **rule-engine**: Stream-processing detection microservice consuming typed Kafka topics with YARA-X rule matching
- **rule-engine**: In-memory MITRE ATT&CK taxonomy loader providing sub-15ns technique enrichment and threat scoring
- **rule-engine**: Sharded 16-bucket LRU deduplicator to prevent SOC alert flooding during high-volume event bursts
- **rule-engine**: Dual alert sink persisting to PostgreSQL via diesel-async and broadcasting to Kafka topic `aigis.alerts`
- **rule-engine**: Dead letter queue producer routing malformed or unparsable event payloads to `aigis.events.dlq`
- **rule-engine**: SIGHUP rule hot-reload supporting zero-downtime rule updates via atomic pointer swapping
- **rule-engine**: Prometheus metrics and Axum health check endpoints for liveness and readiness monitoring
- **scripts**: Automated rule provisioning script (`scripts/fetch-rules.sh`) to download MITRE STIX data and community YARA signatures on demand
- **scripts**: Cross-platform system and development dependency installation in `scripts/setup.sh` supporting macOS (Homebrew) and Linux distributions (apt, dnf, pacman, apk)
- **api-backend**: REST endpoints for node inventory, alert triage, and telemetry search in Axum 0.8
- **api-backend**: Bearer JWT authentication and Argon2id password verification on operator routes
- **api-backend**: Multi-database connection pools with diesel-async for `edr_nodes` (5433), `edr_alerts` (5434), and `edr_logs` (5435)
- **api-backend**: Real-time event streaming over WebSockets via background rdkafka consumer and Tokio broadcast channels
- **api-backend**: Host isolation and un-isolation command dispatch to the Fleet Server
- **fleet-server**: Tonic gRPC controller handling node registration, health heartbeats, and event streams
- **fleet-server**: Idempotent node enrollment transactions in PostgreSQL using machine identity from `/etc/machine-id`
- **fleet-server**: 24-hour HMAC-SHA256 token generation and validation for agent connections
- **fleet-server**: Kafka event publisher bridge routing agent telemetry into `aigis.events.raw`
- **kafka-pipeline**: Event router dividing raw telemetry into typed topics (`process`, `network`, `file`, `auth`)
- **kafka-pipeline**: Topic administration tool (`kafka-admin`) to provision partitions and retention policies
- **agent**: osquery Thrift client using Unix domain sockets with differential query snapshotting
- **agent**: SQLite WAL buffer for offline telemetry storage during network disconnects
- **agent**: Host quarantine management using Linux nftables packet-filtering rules
- **agent**: Hardware-stable machine ID extraction and OS release parsing from `/etc/os-release`
- **infra**: Single-command startup script (`./scripts/infra.sh up`) that initializes databases, seeds test data, and provisions Kafka topics
- **infra**: Automated DDL schema and mock fixtures for `edr_nodes`, `edr_alerts`, and `edr_logs`
- **frontend**: React and TypeScript operator console for viewing nodes, triaging alerts, and searching logs

### Changed

- **fleet-server**: Migrated database layer from sqlx to diesel-async with deadpool connection pooling for non-blocking offline compilation and unified PostgreSQL ORM architecture
- **workspace**: Consolidated shared dependencies (diesel, diesel-async, deadpool-diesel, yara-x, arc-swap, lru, num_cpus, dotenvy, futures-util, clap, metrics, tempfile) into root workspace dependencies across all crate manifests
- **rule-engine**: Configured gitignore to exclude downloaded external YARA signatures and STIX JSON files while preserving custom rules in `rules/custom/`
- **agent**: Switched fleet transport and offline buffer serialization from Protobuf to JSON
- **infra**: Consolidated all scattered configuration files into a single root `.env` and `.env.example`
- **infra**: Updated PostgreSQL logs database port mapping to 5435 to avoid host port conflicts

### Removed

- **workspace**: Removed sqlx from workspace dependencies following the fleet-server diesel-async migration
- **workspace**: Removed unused `sled` and `http-body` dependencies from root Cargo.toml
- **kafka-pipeline**: Removed unused `sqlx` dependency from `kafka-pipeline/Cargo.toml`


### Fixed

- **rule-engine**: Corrected invalid librdkafka configuration key `fetch.max.wait.ms` to `fetch.wait.max.ms` to prevent consumer startup panic
- **kafka-pipeline**: Corrected invalid librdkafka configuration key `fetch.max.wait.ms` to `fetch.wait.max.ms`
- **fleet-server**: Serialized incoming agent events into structured TelemetryEvent JSON envelopes before publishing to `aigis.events.raw` to preserve event type and node metadata
- **kafka-pipeline**: Added message offset commits after processing and routed unclassified event types to `aigis.events.dlq` to prevent infinite reprocessing loops
- **rule-engine**: Added asynchronous consumer message offset commits and expanded payload buffer extraction to parse nested osquery row arrays
- **infra**: Added missing canonical Kafka topics (process, network, file, auth, dlq, heartbeats) to Docker Compose kafka-init, infra.sh, and create-topics.sh
- **infra**: Mounted host rule directory and MITRE taxonomy in rule-engine Docker Compose service and copied rules into container build stage
- **api-backend**: Connected FleetClient to Fleet Server gRPC control plane for host containment dispatch
- **api-backend**: Added missing native `libcurl4-openssl-dev` dependency required for rdkafka static builds in Docker
- **agent**: Resolved SQLite thread-safety comments and added unit tests for FleetClient identity handling
- **kafka-pipeline**: Corrected doc comments in `kafka-admin` and consumer metrics modules
- **scripts**: Added macOS Homebrew libpq discovery and nightly toolchain verification in development and CI scripts

---

## [1.0.0-beta.2] - 2026-06-15

### Added

- **agent**: Cross-platform Linux release builds for x86_64 and aarch64
- **agent**: Configuration file watcher for `agent.toml`
- **agent**: Periodic heartbeat reporting for agent status and buffer backlog metrics

---

## [1.0.0-beta.1] - 2026-06-01

### Added

- **workspace**: Scaffolding for crates (`agent`, `fleet-server`, `kafka-pipeline`, `rule-engine`, `sdk`, `frontend`)
- **sdk**: Shared Protobuf schemas and gRPC contracts in `sdk/proto/`
- **infra**: Docker Compose definitions for Kafka, Zookeeper, and PostgreSQL
