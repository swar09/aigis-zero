# Changelog

All notable changes to the Aigis-Zero EDR project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to Semantic Versioning.

---

## [Unreleased]

### Added

- **infra**: Added comprehensive operations and deployment guide in `infra/guide.md` covering database partitioning, Kafka topology, KEDA autoscaling, and backup runbooks
- **api-backend**: Added operational guide in `api-backend/guide.md` covering multi-pool database routing, live WebSocket streaming, and quarantine command dispatch
- **fleet-server**: Added operations guide in `fleet-server/guide.md` documenting agent enrollment handshakes, bidirectional gRPC streaming, and heartbeat invariants
- **agent**: Added operational guide in `agent/guide.md` covering eBPF osquery IPC, SQLite WAL event buffering, and nftables network quarantine
- **rule-engine**: Added operational guide in `rule-engine/guide.md` covering YARA-X rule syntax, MITRE ATT&CK indexing, sharded LRU deduplication, and SIGHUP hot-reloading
- **sdk**: Added `sdk/README.md` and `sdk/guide.md` documenting Protocol Buffer contracts, prost code generation, and domain codecs
- **frontend**: Added operational guide in `frontend/guide.md` covering Next.js App Router standalone builds, proxy rewrites, and operator triage workflows
- **infra**: Multi-stage standalone production Dockerfile for Next.js frontend with unprivileged node user and health check probe
- **ci**: GitHub Actions workflow (`.github/workflows/docker-publish.yml`) building and publishing production container images to GitHub Container Registry (`ghcr.io`) on push and release tags
- **infra**: Dedicated production environment template (`.env.production.example`) with container service discovery hostnames and security guidance
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
- **kafka-pipeline**: Axum HTTP health and Prometheus metrics server listening on port 8082 with liveness and readiness probes
- **kafka-pipeline**: Detailed Kafka dead-letter-queue record headers for error tracing across pipeline boundaries
- **agent**: osquery Thrift client using Unix domain sockets with differential query snapshotting
- **agent**: SQLite WAL buffer for offline telemetry storage during network disconnects
- **agent**: Host quarantine management using Linux nftables packet-filtering rules
- **agent**: Hardware-stable machine ID extraction and OS release parsing from `/etc/os-release`
- **infra**: Single-command startup script (`./scripts/infra.sh up`) that initializes databases, seeds test data, and provisions Kafka topics
- **infra**: Automated DDL schema and mock fixtures for `edr_nodes`, `edr_alerts`, and `edr_logs`
- **frontend**: Next.js App Router operator console with Cloudflare light theme tokens, real-time WebSocket telemetry ingestion, and threat hunt telemetry query interface
- **docs**: Reorganized architecture and operational instructions across dedicated READMEs in agent, fleet-server, kafka-pipeline, and rule-engine directories
- **fleet-client**: Added concurrent multi-agent enrollment integration test verifying race-free registration under simultaneous agent load
- **api-backend**: Production micro-batch telemetry persistence pipeline inserting Kafka events into PostgreSQL edr_logs database
- **frontend**: Added dedicated Event Logs (/logs) and Network Activity & Containment (/network) console views
- **frontend**: Next.js auth guard middleware protecting console routes with cookie verification
- **frontend**: Accessible Modal and Drawer primitives with ARIA dialog roles, focus management, and keyboard dismissal
- **frontend**: Reusable LogTable telemetry viewer unifying event inspection across log audit and threat hunt views
- **frontend**: Route-level loading skeleton (loading.tsx), runtime error recovery boundary (error.tsx), and 404 page (not-found.tsx)
- **frontend**: Vitest and React Testing Library automated test suite with 26 unit and component tests across API client, query cache, status badges, metric cards, dialog modals, drawers, and permission guards
- **frontend**: Client query cache in `lib/cache.ts` providing in-flight request deduplication and TTL-based stale-while-revalidate data delivery
- **frontend**: Role-based access control guard component `<Can>` for conditional rendering based on user permission claims
- **frontend**: Client API methods for `getMe`, `getLogById`, and paginated result retrieval with total count preservation
- **frontend**: Strict HTTP response security headers in `next.config.mjs` including X-Frame-Options, X-Content-Type-Options, and Referrer-Policy
- **scripts**: Dedicated frontend quality verification script `scripts/frontend-check.sh` integrated into `scripts/check.sh` and `scripts/ci.sh`
- **scripts**: Enhanced `scripts/check.sh` with target selection supporting frontend only, backend only, specific services (api-backend, fleet-server, agent, kafka-pipeline, rule-engine, sdk), individual crates, and full suite execution by default

### Changed

- **docs**: Updated root `README.md` with an end-to-end lifecycle walkthrough from agent enrollment to cluster deployment, along with a subsystem documentation matrix
- **docs**: Synchronized port configurations across `infra/README.md`, `api-backend/README.md`, and `frontend/README.md` to match actual service bindings
- **infra**: Hardened microservice Dockerfiles (api-backend, fleet-server, kafka-pipeline, rule-engine, agent) with BuildKit cache mounts and single-layer permission handling
- **fleet-server**: Updated Dockerfile to run under an unprivileged user (`edr:1001`) instead of root, and added TCP socket health checking
- **kafka-pipeline**: Added `EXPOSE 8082` and HTTP health check probe to Dockerfile
- **rule-engine**: Added start-period grace window to container readiness probe to prevent false restarts during rule compilation
- **agent**: Modernized osquery repository GPG key management in agent Dockerfile using dearmored keyrings and added capability-hardened runtime guidance
- **agent**: Enabled DNS hostname resolution in `parse_endpoint` to allow connecting to remote or containerized fleet servers
- **infra**: Hardened Docker Compose port mappings to bind to loopback (`127.0.0.1`) by default, protecting databases and Kafka brokers from direct internet exposure
- **frontend**: Aligned sidebar icons and page title headers across all console navigation tabs with Threat Hunt included
- **frontend**: Replaced fragile WebSocket host matching with configurable URLs, exponential backoff with jitter, and keep-alive ping heartbeats
- **frontend**: Removed direct DOM style mutations in sidebar links in favor of pure CSS pseudo-classes
- **frontend**: Fixed accessibility violations across login forms, modal dialogs, and detail drawers
- **rule-engine**: Synchronized environment variable mutation in unit tests with a static mutex lock to prevent concurrent test races
- **frontend**: Flattened sidebar into direct clickable tabs for Alerts, Endpoints, Logs, Network, Dashboard, and Settings, removing nested section headers and sub-item labels
- **frontend**: Unified sidebar collapse toggle into a single button on the TopBar Aigis-Zero logo and removed separate Hide and Show Menu controls
- **frontend**: Removed bottom Aigis-Zero branding from sidebar to maintain brand identity strictly in the top bar
- **frontend**: Confined sliding ASCII background exclusively to the login screen, leaving the authenticated dashboard layout clean
- **frontend**: Implemented dynamic viewport row and column repeat calculations in AsciiBackground to eliminate blank gaps across zoom levels and resolutions
- **frontend**: Replaced complex operator authentication copy with standard terminology (Username, Password, Sign in)
- **frontend**: Aligned operator interface with Aigis-Zero editorial identity, restored favicon and logo assets across the console, and added full-viewport scrolling ASCII background
- **frontend**: Removed EDR branding labels and version strings across console headers, sidebar, and documentation
- **agents**: Reorganized agents directory into dedicated directories per agent at `.agents/agents/{agent_name}/agent.md` with updated index and documentation
- **fleet-server**: Migrated database layer from sqlx to diesel-async with deadpool connection pooling for non-blocking offline compilation and unified PostgreSQL ORM architecture
- **workspace**: Consolidated shared dependencies (diesel, diesel-async, deadpool-diesel, yara-x, arc-swap, lru, num_cpus, dotenvy, futures-util, clap, metrics, tempfile) into root workspace dependencies across all crate manifests
- **fleet-server**: Configured KafkaPublisher with LZ4 compression, linger micro-batching, and buffer limits
- **kafka-pipeline**: Configured EventRouter producer with LZ4 compression, micro-batching, and graceful buffer flushing on shutdown
- **rule-engine**: Configured gitignore to exclude downloaded external YARA signatures and STIX JSON files while preserving custom rules in `rules/custom/`
- **agent**: Switched fleet transport and offline buffer serialization from Protobuf to JSON
- **infra**: Consolidated all scattered configuration files into a single root `.env` and `.env.example`
- **infra**: Updated PostgreSQL logs database port mapping to 5435 to avoid host port conflicts

### Removed

- **workspace**: Removed Justfile and Makefile wrappers in favor of direct execution of scripts in ./scripts/
- **workspace**: Removed sqlx from workspace dependencies following the fleet-server diesel-async migration
- **workspace**: Removed unused `sled` and `http-body` dependencies from root Cargo.toml
- **kafka-pipeline**: Removed unused `sqlx` dependency from `kafka-pipeline/Cargo.toml`


### Fixed

- **agent**: Passed configured enrollment secret during fleet registration handshake to allow successful node authentication

- **frontend**: WebSocket heartbeat payload corrected from `{ type: 'ping' }` to `{ action: 'ping' }` matching backend parser schema in `models/ws.rs`
- **fleet-client**: Added pre-enrollment validation check in concurrent integration test to safely skip when fleet server is offline
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
- **infra**: Pinned Fleet Server gRPC port to 50051 in Docker Compose to prevent port collision with the API backend port variable
- **infra**: Added missing kafka-pipeline stream router service definition to Docker Compose configuration
- **agent**: Marked /run/osquery path optional in systemd service mount namespace to prevent startup failure when the directory is absent on boot
- **api-backend**: Extracted agent UUID, event type, and payload correctly from osquery telemetry envelopes across WebSocket and REST feeds
- **infra**: Upgraded Rust builder base images to `rust:1-slim-bookworm` across all microservice Dockerfiles to satisfy Rust 2024 edition and transitive crate requirements
- **agent**: Removed brittle `x86_64-musl` target and hardcoded `amd64` osquery repository architecture to enable portable multi-platform container builds
- **scripts**: Exported `RULES_DIR` and parameterized output paths in `scripts/fetch-rules.sh` to allow reliable execution from any working directory
- **infra**: Stripped comments across all Dockerfiles and Docker Compose files to maintain minimal configuration standards
- **ci**: Updated `agent-release.yml` with dual `aarch64` and `arm64` release tarball assets, protoc installation, and root workspace compilation
- **ci**: Configured multi-platform image builds (`linux/amd64`, `linux/arm64`) with QEMU in `docker-publish.yml`
- **agent**: Replaced osquery apt repository install with direct GitHub release download (`osquery_5.23.1-1.linux_${ARCH}.deb`), enabling multi-platform agent Docker builds for both `linux/amd64` and `linux/arm64`
- **infra**: Fixed `infra/docker-compose.yml` app service image tags to use local names (`aigis-*:local`) so `./scripts/infra.sh` always builds from local Dockerfiles and never pulls from the container registry
- **scripts**: Updated `infra.sh` to build from local Dockerfiles by default and accept `--pull` flag to switch to the production manifest pulling images from GHCR

### Security

- **infra**: Hardened `.dockerignore` to completely block `.env` files, certificates, private keys, logs, and node_modules from leaking into Docker build contexts
- **infra**: Removed hardcoded plaintext passwords and fallback secrets from fleet-server and api-backend Docker Compose files
- **ci**: Removed legacy `SQLX_OFFLINE` environment variable references from CI workflows and Dockerfiles
- **fleet-server**: Enforced pre-shared enrollment secret (`FLEET_ENROLLMENT_SECRET`) validation on gRPC `RegisterAgent` endpoint to prevent rogue node registration
- **fleet-server**: Pinned JWT validation to HMAC-SHA256 algorithm in gRPC authentication filter to prevent algorithm downgrade attacks
- **api-backend**: Pinned JWT decoding strictly to HS256 in authentication middleware
- **api-backend**: Injected HTTP security headers (`X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `X-XSS-Protection`, `Referrer-Policy`) on all REST and WebSocket responses
- **dependencies**: Upgraded `h2` from v0.4.15 to v0.4.19 resolving upstream advisory RUSTSEC-2026-0258

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
