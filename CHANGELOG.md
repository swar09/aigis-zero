# Changelog

All notable changes to the Aigis-Zero EDR project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to Semantic Versioning.

---

## [Unreleased]

### Added

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

- **agent**: Switched fleet transport and offline buffer serialization from Protobuf to JSON
- **infra**: Consolidated all scattered configuration files into a single root `.env` and `.env.example`
- **infra**: Updated PostgreSQL logs database port mapping to 5435 to avoid host port conflicts


### Fixed

- **api-backend**: Added missing native `libcurl4-openssl-dev` dependency required for rdkafka static builds in Docker
- **agent**: Resolved SQLite thread-safety comments and added unit tests for FleetClient identity handling
- **kafka-pipeline**: Corrected doc comments in `kafka-admin` and consumer metrics modules

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
