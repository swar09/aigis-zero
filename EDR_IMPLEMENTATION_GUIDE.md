# EDR — Full System Implementation Guide

> **AXIOM** | Version 1.0 | Rust-First Architecture | Linux Only | eBPF + OSQuery
>
> This document is the single source of truth for building, initializing, and understanding every
> component of the EDR system. Read it fully before writing a single line of code.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Technology Decisions & Rationale](#2-technology-decisions--rationale)
3. [Repository Strategy — Polyrepo](#3-repository-strategy--polyrepo)
4. [Repository Initialization — Step by Step](#4-repository-initialization--step-by-step)
5. [Linux Node Agent — Deep Dive](#5-linux-node-agent--deep-dive)
6. [Fleet Server — Deep Dive](#6-fleet-server--deep-dive)
7. [Kafka Pipeline — Deep Dive](#7-kafka-pipeline--deep-dive)
8. [Rule Engine — Deep Dive](#8-rule-engine--deep-dive)
9. [API Backend — Deep Dive](#9-api-backend--deep-dive)
10. [Frontend Dashboard — Deep Dive](#10-frontend-dashboard--deep-dive)
11. [EDR SDK — Deep Dive](#11-edr-sdk--deep-dive)
12. [Docker & Container Strategy](#12-docker--container-strategy)
13. [Inter-Service Communication](#13-inter-service-communication)
14. [Database Design](#14-database-design)
15. [GitHub Actions CI/CD](#15-github-actions-cicd)
16. [Phase Roadmap](#16-phase-roadmap)

---

## 1. Architecture Overview

### System Data Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                         LINUX ENDPOINTS                              │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    edr-agent (Rust)                           │   │
│  │                                                               │   │
│  │  ┌─────────────┐   ┌──────────────┐   ┌──────────────────┐  │   │
│  │  │ eBPF Probes │   │ OSQuery Shim │   │  Local Buffer    │  │   │
│  │  │  (Rust/C)   │   │   (Rust)     │   │  (RocksDB/sled)  │  │   │
│  │  └──────┬──────┘   └──────┬───────┘   └────────┬─────────┘  │   │
│  │         │                 │                     │             │   │
│  │         └────────┬────────┘                     │             │   │
│  │                  ▼                               │             │   │
│  │         ┌────────────────┐    buffers to ───────┘             │   │
│  │         │  Event Stream  │◄──────────────────────             │   │
│  │         │  Aggregator    │                                     │   │
│  │         └───────┬────────┘                                     │   │
│  │                 │ gRPC (TLS) bidirectional stream               │   │
│  └─────────────────┼───────────────────────────────────────────┘   │
│                    │                                                  │
└────────────────────┼──────────────────────────────────────────────┘
                     │
                     ▼
┌────────────────────────────────────────────────────────────────────┐
│                    edr-fleet-server (Rust/Axum/Tokio)               │
│                                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │ Enrollment   │  │  Config Mgr  │  │  Command & Control       │  │
│  │ & Auth       │  │  (push)      │  │  (isolation relay)       │  │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘  │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │              Kafka Producer (rdkafka)                          │ │
│  │   topic: edr.events.raw  |  topic: edr.health                  │ │
│  └────────────────────────────────────────────────────────────────┘ │
└───────────────────────────────┬────────────────────────────────────┘
                                │
                                ▼
┌───────────────────────────────────────────────────────────────────┐
│                    Apache Kafka (Docker)                            │
│                                                                     │
│   topic: edr.events.raw   ──────────► Event Processor consumes    │
│   topic: edr.events.norm  ──────────► Rule Engine consumes        │
│   topic: edr.alerts       ──────────► API Backend consumes        │
│   topic: edr.health       ──────────► API Backend consumes        │
└───────────────────────────────────────────────────────────────────┘
                                │
              ┌─────────────────┼──────────────────┐
              ▼                 ▼                    ▼
┌─────────────────┐  ┌──────────────────┐  ┌───────────────────────┐
│ edr-event-      │  │  edr-rule-engine │  │  edr-ml-detection     │
│ processor(Rust) │  │     (Rust)       │  │  (Python - separate)  │
│                 │  │                  │  │                        │
│ Kafka Consumer  │  │ YARA scanning    │  │ Anomaly detection      │
│ Normalizer      │  │ MITRE mapping    │  │ Inference pipeline     │
│ PostgreSQL write│  │ Alert generation │  │ Threat scoring         │
└────────┬────────┘  └────────┬─────────┘  └──────────┬────────────┘
         │                    │                         │
         ▼                    ▼                         ▼
┌────────────────────────────────────────────────────────────────────┐
│                    PostgreSQL (Docker)                              │
│                                                                     │
│   edr_logs_db      — all raw + normalised event logs               │
│   edr_nodes_db     — node registry, health, config state           │
│   edr_alerts_db    — alerts, MITRE mappings, threat scores         │
└───────────────────────────────┬────────────────────────────────────┘
                                │
                                ▼
┌───────────────────────────────────────────────────────────────────┐
│                 edr-api-backend (Rust/Axum/Tokio)                  │
│                                                                     │
│   REST endpoints   ──► Node list, logs query, alerts query        │
│   WebSocket        ──► Real-time alert + health push              │
│   JWT Auth         ──► Operator authentication                    │
└───────────────────────────────┬────────────────────────────────────┘
                                │
                                ▼
┌───────────────────────────────────────────────────────────────────┐
│                    edr-frontend (React + Vite)                     │
│                                                                     │
│   Node Map  |  Live Logs  |  Alerts Panel  |  Node Controls       │
└───────────────────────────────────────────────────────────────────┘
```

### Component Summary

| Repository | Language | Role | Scales? |
|---|---|---|---|
| `edr-agent` | Rust (Cargo workspace) | Runs on every monitored endpoint | Per-node |
| `edr-fleet-server` | Rust (Axum + Tokio) | Central gRPC server for all agents | Horizontal |
| `edr-kafka-pipeline` | Rust (rdkafka) | Event processing + DB writes | Horizontal |
| `edr-rule-engine` | Rust | YARA + MITRE detection | Horizontal |
| `edr-api-backend` | Rust (Axum + Tokio) | REST + WebSocket for frontend | Horizontal |
| `edr-frontend` | React + Vite + TypeScript | Operator dashboard | Static CDN |
| `edr-sdk` | Rust (lib crate) | Shared types, proto, client helpers | Library |
| `edr-infra` | Docker Compose + K8s | All infrastructure definitions | — |

---

## 2. Technology Decisions & Rationale

### Kafka vs Zenoh — Decision: Kafka

Zenoh is excellent for IoT robotics pub-sub with sub-millisecond latency. However EDR has different requirements:

**Why Kafka wins for EDR:**
- **Durability**: Events are persisted to disk with configurable retention. If the Rule Engine is down, events queue. With Zenoh, you lose them.
- **Consumer Groups**: Multiple Rule Engine instances can consume in parallel, each processing different partitions — native horizontal scaling.
- **Replay**: If the ML model is retrained, you can replay the last 7 days of events through it from Kafka. Impossible with Zenoh.
- **Exactly-once semantics**: Security pipelines cannot lose or double-process events. Kafka supports this natively.
- **Rust ecosystem**: `rdkafka` (librdkafka bindings) is mature, production-tested, and well maintained.
- **Operational tooling**: Kafka UI, consumer lag monitoring, partition rebalancing — all battle-tested.

Zenoh would be appropriate if agents were IoT devices with sub-100ms latency requirements and no persistence needs. That is not EDR.

### Rust Framework — Decision: Axum + Tokio (No Rocket, No Actix)

- **Tokio** is the async runtime. Everything in the Rust async ecosystem is built around it.
- **Axum** is built by the Tokio team, uses Tower middleware natively, compiles faster than Actix, has excellent ergonomics for extractors and state management, and does not require macros for routing.
- **Actix-web** uses its own actor runtime which can conflict with Tokio in complex multi-crate workspaces. It also has a steeper learning curve for middleware composition.
- **Rocket** requires nightly Rust features and has slower compile times.

### gRPC Framework — Decision: Tonic

`tonic` is the standard async gRPC library for Rust built on Tokio. It generates Rust code from `.proto` files via `prost` and supports bidirectional streaming which the agent↔fleet connection requires.

### Agent Architecture — Decision: Cargo Workspace

The agent has multiple concerns (eBPF, OSQuery integration, buffer management, gRPC client) that are best separated into individual crates within one Cargo workspace. This enables:
- Independent compilation of each subsystem
- Clean dependency boundaries
- Shared types via a common crate
- Easy testing of individual components in isolation

---

## 3. Repository Strategy — Polyrepo

### Why Polyrepo, Not Monorepo

The EDR system has components in fundamentally different languages and build systems: Rust (Cargo), React (Vite/npm), and Python (ML). A monorepo would require a meta-build system (Bazel/Nx) to manage cross-language builds, which adds complexity that is not justified at this stage.

More importantly, each component has a completely independent deployment lifecycle:
- The agent is compiled to a binary and distributed to endpoints. It does not redeploy when the frontend changes.
- The frontend is a static build served by a CDN or nginx. It does not need Rust toolchain installed.
- The fleet server is a long-running service that scales independently.

**Polyrepo with a shared `edr-sdk` crate** gives the independence of separate repos while maintaining shared type safety.

### Repositories

```
github.com/your-org/
├── edr-agent           ← Cargo workspace, compiled binary for Linux endpoints
├── edr-fleet-server    ← Rust/Axum, Docker image
├── edr-kafka-pipeline  ← Rust/rdkafka, Docker image
├── edr-rule-engine     ← Rust, Docker image
├── edr-api-backend     ← Rust/Axum, Docker image
├── edr-frontend        ← React/Vite/TypeScript, static build
├── edr-sdk             ← Rust library crate (published internally or as git dep)
└── edr-infra           ← Docker Compose, K8s manifests, Terraform
```

### How Shared Types Work (edr-sdk)

All services that communicate share types via `edr-sdk`. In `Cargo.toml`:

```toml
[dependencies]
edr-sdk = { git = "https://github.com/your-org/edr-sdk", tag = "v0.1.0" }
```

This means when a proto definition changes, it changes in `edr-sdk`, a new tag is cut, and each consuming service bumps its dependency. Breaking changes are visible at compile time.

---

## 4. Repository Initialization — Step by Step

### Step 1 — Create All Repos on GitHub

Create each repo as **private**. Initialize with a `README.md` only — no auto-generated code.

```bash
# Using GitHub CLI (gh)
for repo in edr-agent edr-fleet-server edr-kafka-pipeline edr-rule-engine edr-api-backend edr-frontend edr-sdk edr-infra; do
  gh repo create your-org/$repo --private --description "EDR: $repo"
done
```

### Step 2 — Initialize edr-sdk First (Everything depends on it)

```bash
git clone git@github.com:your-org/edr-sdk.git
cd edr-sdk

# Initialize as a Rust library
cargo init --lib .

# Create the directory structure
mkdir -p proto src/{types,auth,events,health}
```

`Cargo.toml` for edr-sdk:
```toml
[package]
name = "edr-sdk"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
prost = "0.12"
tonic = "0.11"

[build-dependencies]
tonic-build = "0.11"
```

`build.rs` for edr-sdk:
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &[
                "proto/agent.proto",
                "proto/fleet.proto",
                "proto/events.proto",
            ],
            &["proto/"],
        )?;
    Ok(())
}
```

### Step 3 — Initialize edr-agent as Cargo Workspace

```bash
git clone git@github.com:your-org/edr-agent.git
cd edr-agent

# Create workspace Cargo.toml manually (do NOT run cargo init here)
```

Create `Cargo.toml` (workspace root):
```toml
[workspace]
resolver = "2"
members = [
    "crates/agent-core",
    "crates/ebpf-collector",
    "crates/osquery-client",
    "crates/event-buffer",
    "crates/fleet-client",
    "crates/isolation",
]

[workspace.dependencies]
# Pin versions once here, reference in member crates with { workspace = true }
tokio = { version = "1", features = ["full"] }
tonic = "0.11"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
anyhow = "1"
thiserror = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
edr-sdk = { git = "https://github.com/your-org/edr-sdk", tag = "v0.1.0" }
```

Initialize each crate:
```bash
for crate in agent-core ebpf-collector osquery-client event-buffer fleet-client isolation; do
  cargo new --lib crates/$crate
done
```

### Step 4 — Initialize Rust Services (fleet-server, kafka-pipeline, rule-engine, api-backend)

Each follows the same pattern:

```bash
cd edr-fleet-server
cargo init --bin .
```

`Cargo.toml` template for services:
```toml
[package]
name = "edr-fleet-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "edr-fleet-server"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = { version = "0.7", features = ["ws"] }
tonic = "0.11"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "compression-gzip"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
anyhow = "1"
thiserror = "1"
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls", "uuid", "chrono"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
config = "0.14"
dotenv = "0.15"
edr-sdk = { git = "https://github.com/your-org/edr-sdk", tag = "v0.1.0" }
```

### Step 5 — Initialize Frontend

```bash
cd edr-frontend
npm create vite@latest . -- --template react-ts
npm install
npm install axios @tanstack/react-query zustand react-router-dom
npm install -D tailwindcss postcss autoprefixer
npx tailwindcss init -p
```

### Step 6 — Initialize edr-infra

```bash
cd edr-infra
mkdir -p docker k8s/manifests terraform scripts
touch docker-compose.yml docker-compose.dev.yml
touch README.md
```

### Step 7 — Set Branch Protection on Every Repo

For each repo on GitHub:
- `main` → require 2 approvals, all CI checks must pass, no direct push
- `develop` → require 1 approval, CI must pass

### Step 8 — Add .github/ to Every Repo

Create `.github/PULL_REQUEST_TEMPLATE.md` in each repo:

```markdown
## Summary
<!-- What does this PR do? One paragraph. -->

## Type
- [ ] feat — new functionality
- [ ] fix — bug fix
- [ ] chore — dependency update, refactor, tooling
- [ ] docs — documentation only
- [ ] sec — security fix or hardening

## Checklist
- [ ] Linked issue: closes #
- [ ] No secrets or credentials in code
- [ ] Tests added or updated
- [ ] `docker-compose up` tested locally
- [ ] Breaking changes documented in PR description

## How to verify
<!-- Steps for the reviewer -->
```

---

## 5. Linux Node Agent — Deep Dive

### Cargo Workspace Structure

```
edr-agent/
├── Cargo.toml                    ← workspace root
├── Cargo.lock
├── .cargo/
│   └── config.toml               ← linker config for eBPF targets
├── crates/
│   ├── agent-core/               ← binary entry point, orchestrator
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── config.rs         ← reads agent config from Fleet Server
│   │       └── orchestrator.rs   ← spawns all subsystem tasks
│   │
│   ├── ebpf-collector/           ← eBPF programs and loader
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── loader.rs         ← loads compiled eBPF objects
│   │   │   └── events.rs         ← parses perf buffer events
│   │   └── bpf/                  ← eBPF C programs compiled with clang
│   │       ├── process_probe.bpf.c
│   │       ├── file_probe.bpf.c
│   │       └── network_probe.bpf.c
│   │
│   ├── osquery-client/           ← OSQuery socket IPC client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs         ← thrift/unix socket client
│   │       └── queries.rs        ← scheduled query definitions
│   │
│   ├── event-buffer/             ← local disk buffer (sled embedded DB)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── buffer.rs         ← write-ahead buffer, flush on reconnect
│   │
│   ├── fleet-client/             ← gRPC client to Fleet Server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── connection.rs     ← manages gRPC channel with reconnect
│   │       └── stream.rs         ← bidirectional event stream
│   │
│   └── isolation/                ← IPTables-based network isolation
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── iptables.rs       ← adds/removes isolation rules
│
├── Dockerfile
└── build.rs                      ← compiles eBPF C programs via clang
```

### Required Crates per Sub-Crate

**agent-core:**
```toml
[dependencies]
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }
config = "0.14"
serde = { workspace = true }
edr-sdk = { workspace = true }
ebpf-collector = { path = "../ebpf-collector" }
osquery-client = { path = "../osquery-client" }
event-buffer = { path = "../event-buffer" }
fleet-client = { path = "../fleet-client" }
isolation = { path = "../isolation" }
```

**ebpf-collector:**
```toml
[dependencies]
aya = "0.12"                    # eBPF loader — pure Rust, no libbpf C dependency
aya-log = "0.2"
tokio = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
bytes = "1"
edr-sdk = { workspace = true }

[build-dependencies]
aya-build = "0.1"               # compiles BPF C programs at build time
```

**osquery-client:**
```toml
[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tokio-util = { version = "0.7", features = ["codec"] }
edr-sdk = { workspace = true }
```

**event-buffer:**
```toml
[dependencies]
sled = "0.34"                   # embedded key-value store, perfect for WAL buffer
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
edr-sdk = { workspace = true }
```

**fleet-client:**
```toml
[dependencies]
tonic = { workspace = true }
tokio = { workspace = true }
tokio-stream = "0.1"
tower = "0.4"
anyhow = { workspace = true }
tracing = { workspace = true }
edr-sdk = { workspace = true }
```

**isolation:**
```toml
[dependencies]
tokio = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
# Uses std::process::Command to invoke iptables — no external crate needed
```

### eBPF Probes — What They Capture

Three BPF programs, each attached to specific kernel hooks:

**process_probe.bpf.c** — attaches to `sys_enter_execve`:
- Process name, PID, PPID
- Full command-line arguments
- User ID, effective UID
- Working directory

**file_probe.bpf.c** — attaches to `sys_enter_openat`, `sys_enter_unlinkat`:
- File path accessed
- Operation (open/read/write/delete)
- Process that triggered it
- Return code (was it successful?)

**network_probe.bpf.c** — attaches to `sys_enter_connect`, `sys_enter_bind`:
- Source IP, destination IP
- Source port, destination port
- Protocol (TCP/UDP)
- Process that made the call

### OSQuery Scheduled Queries

Queries run on configurable intervals (pushed from Fleet Server):

```json
{
  "schedule": {
    "running_processes": {
      "query": "SELECT pid, name, path, cmdline, uid, parent FROM processes;",
      "interval": 30
    },
    "active_connections": {
      "query": "SELECT pid, local_address, local_port, remote_address, remote_port, state FROM process_open_sockets WHERE state = 'ESTABLISHED';",
      "interval": 60
    },
    "file_events": {
      "query": "SELECT target_path, action, time, auid FROM file_events;",
      "interval": 15
    },
    "logged_in_users": {
      "query": "SELECT user, tty, host, time, pid FROM logged_in_users;",
      "interval": 120
    },
    "installed_packages": {
      "query": "SELECT name, version, source FROM deb_packages;",
      "interval": 3600
    }
  }
}
```

### Agent State Machine

```
INITIALIZING
    │
    ▼
ENROLLING ──────────────────► ENROLLMENT_FAILED (retry with backoff)
    │
    ▼
CONNECTED
    │
    ├── collecting (eBPF events flowing)
    ├── collecting (OSQuery results flowing)
    ├── buffering (if connection drops → RECONNECTING)
    │
    ▼
RECONNECTING ───────────────► exponential backoff, drain buffer on reconnect
    │
    ▼
ISOLATING ──────────────────► received ISOLATE command from Fleet Server
    │
    ▼
ISOLATED ───────────────────► IPTables rules active, only Fleet Server reachable
```

### Agent Config File (`/etc/edr/agent.toml`)

```toml
[fleet]
endpoint = "https://fleet.internal:50051"
tls_cert = "/etc/edr/certs/agent.crt"
tls_key = "/etc/edr/certs/agent.key"
ca_cert = "/etc/edr/certs/ca.crt"

[agent]
node_id = ""          # populated on first enrollment, persisted
buffer_path = "/var/lib/edr/buffer"
log_level = "info"

[osquery]
socket_path = "/var/osquery/osquery.em"
config_refresh_interval_secs = 300

[ebpf]
ringbuf_size_pages = 256
```

---

## 6. Fleet Server — Deep Dive

### Architecture

The Fleet Server is a single horizontally-scalable Rust service. Multiple instances can run behind a load balancer. Shared state lives in PostgreSQL, not in-memory, so any instance can handle any agent connection.

```
edr-fleet-server/
├── Cargo.toml
├── src/
│   ├── main.rs                   ← starts Tokio runtime, binds servers
│   ├── config.rs                 ← reads env + config file
│   ├── state.rs                  ← AppState shared via Arc<>
│   │
│   ├── grpc/                     ← tonic gRPC server
│   │   ├── mod.rs
│   │   ├── server.rs             ← implements FleetService trait (from proto)
│   │   ├── enrollment.rs         ← handles RegisterAgent RPC
│   │   ├── stream.rs             ← handles bidirectional EventStream RPC
│   │   └── commands.rs           ← sends IsolateNode / PushConfig commands
│   │
│   ├── db/                       ← sqlx database layer
│   │   ├── mod.rs
│   │   ├── nodes.rs              ← CRUD for node registry
│   │   ├── health.rs             ← node heartbeat tracking
│   │   └── config.rs             ← agent config storage
│   │
│   ├── kafka/                    ← rdkafka producer
│   │   ├── mod.rs
│   │   └── producer.rs           ← publishes events to edr.events.raw topic
│   │
│   └── error.rs                  ← unified error types
│
├── migrations/                   ← sqlx migrations for nodes_db
│   ├── 001_create_nodes.sql
│   └── 002_create_agent_configs.sql
│
└── Dockerfile
```

### Key Crates

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.7"                                      # HTTP admin API (health check, metrics)
tonic = "0.11"                                    # gRPC server
tonic-reflection = "0.11"                         # gRPC reflection for tooling
tower = "0.4"
tower-http = { version = "0.5", features = ["trace"] }
prost = "0.12"
rdkafka = { version = "0.36", features = ["cmake-build"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls", "uuid", "chrono", "migrate"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
jsonwebtoken = "9"                                # for agent JWT tokens on enrollment
config = "0.14"
anyhow = "1"
thiserror = "1"
edr-sdk = { git = "...", tag = "v0.1.0" }
```

### gRPC Proto Definition (lives in edr-sdk/proto/fleet.proto)

```protobuf
syntax = "proto3";
package edr.fleet;

service FleetService {
  // Agent calls this once to register. Returns a JWT token.
  rpc RegisterAgent(RegisterRequest) returns (RegisterResponse);

  // Bidirectional stream: agent sends events, server sends commands.
  rpc EventStream(stream AgentEvent) returns (stream ServerCommand);

  // Agent sends periodic heartbeat.
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
}

message RegisterRequest {
  string hostname = 1;
  string os_version = 2;
  string agent_version = 3;
  string machine_id = 4;   // from /etc/machine-id
}

message RegisterResponse {
  string node_id = 1;       // UUID assigned by server
  string token = 2;         // JWT for subsequent calls
  AgentConfig config = 3;   // initial configuration
}

message AgentEvent {
  string node_id = 1;
  string event_type = 2;    // "process" | "file" | "network" | "osquery"
  bytes payload = 3;         // JSON-encoded event data
  int64 timestamp_ns = 4;
  string sequence_id = 5;
}

message ServerCommand {
  oneof command {
    IsolateCommand isolate = 1;
    ConfigUpdateCommand config_update = 2;
    AckCommand ack = 3;
  }
}

message IsolateCommand {
  bool isolate = 1;           // true = isolate, false = de-isolate
  string reason = 2;
}

message ConfigUpdateCommand {
  AgentConfig config = 1;
}

message AgentConfig {
  repeated OsquerySchedule osquery_schedule = 1;
  int32 heartbeat_interval_secs = 2;
  int32 batch_size = 3;
}

message HeartbeatRequest {
  string node_id = 1;
  string status = 2;          // "healthy" | "degraded" | "isolated"
  int64 events_buffered = 3;
}

message HeartbeatResponse {
  bool ok = 1;
}
```

### Node Enrollment Flow

```
Agent                           Fleet Server                    PostgreSQL
  │                                  │                               │
  │──── RegisterAgent(hostname) ────►│                               │
  │                                  │── INSERT nodes ──────────────►│
  │                                  │◄─ node_id returned ───────────│
  │                                  │── sign JWT(node_id, exp=24h) ─│
  │◄─── RegisterResponse(token) ─────│                               │
  │                                  │                               │
  │── EventStream (with JWT header) ►│                               │
  │                                  │── verify JWT ─────────────────│
  │◄─── ServerCommand(config) ───────│                               │
  │                                  │                               │
  │  [stream stays open]             │                               │
  │──── AgentEvent (events) ────────►│                               │
  │                                  │── produce to Kafka ───────────►
```

### Horizontal Scaling Design

When multiple fleet-server instances run:
- Each agent connects to one instance (sticky via load balancer)
- Node state in PostgreSQL — any instance can read any node's state
- Isolation commands: operator triggers via API Backend → writes command to PostgreSQL `pending_commands` table → fleet-server instances poll for pending commands and relay to connected agents
- No shared in-memory state between instances

### Fleet Server Environment Variables

```bash
DATABASE_URL=postgres://user:pass@postgres:5432/edr_nodes
KAFKA_BROKERS=kafka:9092
GRPC_BIND_ADDR=0.0.0.0:50051
HTTP_BIND_ADDR=0.0.0.0:8080    # admin/health HTTP port
JWT_SECRET=<random-256-bit-hex>
LOG_LEVEL=info
RUST_LOG=edr_fleet_server=info,tonic=warn
```

---

## 7. Kafka Pipeline — Deep Dive

### Topic Design

| Topic | Producer | Consumer | Retention | Partitions |
|---|---|---|---|---|
| `edr.events.raw` | Fleet Server | Event Processor | 7 days | 12 |
| `edr.events.norm` | Event Processor | Rule Engine, ML | 7 days | 12 |
| `edr.alerts` | Rule Engine, ML | API Backend | 30 days | 4 |
| `edr.health` | Fleet Server | API Backend | 1 day | 4 |

### edr-kafka-pipeline Structure

```
edr-kafka-pipeline/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── consumer.rs       ← consumes edr.events.raw
│   ├── normalizer.rs     ← parses + normalises raw events to common schema
│   ├── producer.rs       ← publishes to edr.events.norm
│   ├── db_writer.rs      ← writes to edr_logs_db PostgreSQL
│   └── error.rs
└── Dockerfile
```

### Key Crates

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
rdkafka = { version = "0.36", features = ["cmake-build", "ssl"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls", "uuid", "chrono"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
anyhow = "1"
thiserror = "1"
edr-sdk = { git = "...", tag = "v0.1.0" }
```

### Normalised Event Schema (defined in edr-sdk)

```rust
// edr-sdk/src/events.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalisedEvent {
    pub id: Uuid,
    pub node_id: Uuid,
    pub event_type: EventType,
    pub timestamp: DateTime<Utc>,
    pub hostname: String,
    pub payload: EventPayload,
    pub raw_sequence_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    Process(ProcessEvent),
    File(FileEvent),
    Network(NetworkEvent),
    OsqueryResult(OsqueryEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cmdline: String,
    pub uid: u32,
    pub exe_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub path: String,
    pub operation: FileOperation,  // Open | Write | Delete | Rename
    pub pid: u32,
    pub process_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEvent {
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: String,
    pub pid: u32,
    pub direction: NetworkDirection,  // Inbound | Outbound
}
```

### Docker — Kafka and PostgreSQL Containers

**Important decision**: Kafka and PostgreSQL run in **separate containers**. Do not combine them. They have different resource profiles (Kafka is I/O bound, PostgreSQL is memory bound), different restart policies, and different backup strategies. Combining them in one container is an operational anti-pattern.

`edr-infra/docker-compose.yml`:

```yaml
version: "3.9"

services:

  zookeeper:
    image: confluentinc/cp-zookeeper:7.6.0
    container_name: edr-zookeeper
    environment:
      ZOOKEEPER_CLIENT_PORT: 2181
      ZOOKEEPER_TICK_TIME: 2000
    volumes:
      - zookeeper_data:/var/lib/zookeeper/data
    healthcheck:
      test: ["CMD", "nc", "-z", "localhost", "2181"]
      interval: 10s
      timeout: 5s
      retries: 5

  kafka:
    image: confluentinc/cp-kafka:7.6.0
    container_name: edr-kafka
    depends_on:
      zookeeper:
        condition: service_healthy
    ports:
      - "9092:9092"
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: PLAINTEXT:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://kafka:29092,PLAINTEXT_HOST://localhost:9092
      KAFKA_INTER_BROKER_LISTENER_NAME: PLAINTEXT
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
      KAFKA_LOG_RETENTION_HOURS: 168
      KAFKA_AUTO_CREATE_TOPICS_ENABLE: "false"
    volumes:
      - kafka_data:/var/lib/kafka/data
    healthcheck:
      test: ["CMD", "kafka-topics", "--bootstrap-server", "localhost:29092", "--list"]
      interval: 15s
      timeout: 10s
      retries: 5

  kafka-init:
    image: confluentinc/cp-kafka:7.6.0
    depends_on:
      kafka:
        condition: service_healthy
    entrypoint: ["/bin/sh", "-c"]
    command: |
      "
      kafka-topics --bootstrap-server kafka:29092 --create --if-not-exists --topic edr.events.raw --partitions 12 --replication-factor 1
      kafka-topics --bootstrap-server kafka:29092 --create --if-not-exists --topic edr.events.norm --partitions 12 --replication-factor 1
      kafka-topics --bootstrap-server kafka:29092 --create --if-not-exists --topic edr.alerts --partitions 4 --replication-factor 1
      kafka-topics --bootstrap-server kafka:29092 --create --if-not-exists --topic edr.health --partitions 4 --replication-factor 1
      echo 'Topics created.'
      "
    restart: on-failure

  postgres-logs:
    image: postgres:16-alpine
    container_name: edr-postgres-logs
    environment:
      POSTGRES_DB: edr_logs
      POSTGRES_USER: edr
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    ports:
      - "5432:5432"
    volumes:
      - postgres_logs_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edr -d edr_logs"]
      interval: 10s
      timeout: 5s
      retries: 5

  postgres-nodes:
    image: postgres:16-alpine
    container_name: edr-postgres-nodes
    environment:
      POSTGRES_DB: edr_nodes
      POSTGRES_USER: edr
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    ports:
      - "5433:5432"
    volumes:
      - postgres_nodes_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edr -d edr_nodes"]
      interval: 10s
      timeout: 5s
      retries: 5

  postgres-alerts:
    image: postgres:16-alpine
    container_name: edr-postgres-alerts
    environment:
      POSTGRES_DB: edr_alerts
      POSTGRES_USER: edr
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    ports:
      - "5434:5432"
    volumes:
      - postgres_alerts_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edr -d edr_alerts"]
      interval: 10s
      timeout: 5s
      retries: 5

  kafka-ui:
    image: provectuslabs/kafka-ui:latest
    container_name: edr-kafka-ui
    depends_on:
      - kafka
    ports:
      - "8090:8080"
    environment:
      KAFKA_CLUSTERS_0_NAME: edr-local
      KAFKA_CLUSTERS_0_BOOTSTRAPSERVERS: kafka:29092

volumes:
  zookeeper_data:
  kafka_data:
  postgres_logs_data:
  postgres_nodes_data:
  postgres_alerts_data:
```

---

## 8. Rule Engine — Deep Dive

```
edr-rule-engine/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── consumer.rs           ← Kafka consumer for edr.events.norm
│   ├── yara_scanner.rs       ← YARA rule evaluation
│   ├── mitre_mapper.rs       ← MITRE ATT&CK technique lookup
│   ├── alert_producer.rs     ← Kafka producer to edr.alerts
│   ├── db_writer.rs          ← writes alerts to edr_alerts_db
│   └── rules/
│       └── loader.rs         ← loads rules from /etc/edr/rules/*.yar
│
├── rules/                    ← default YARA rules (shipped with container)
│   ├── process_injection.yar
│   ├── credential_access.yar
│   └── persistence.yar
│
└── Dockerfile
```

### Key Crates

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
rdkafka = { version = "0.36", features = ["cmake-build"] }
yara-x = "0.5"               # Pure Rust YARA implementation (no C library dependency)
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
anyhow = "1"
edr-sdk = { git = "...", tag = "v0.1.0" }
```

### Alert Schema (defined in edr-sdk)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub node_id: Uuid,
    pub hostname: String,
    pub timestamp: DateTime<Utc>,
    pub severity: Severity,        // Critical | High | Medium | Low
    pub source: AlertSource,       // Yara | MlModel | RuleEngine
    pub mitre_technique_id: Option<String>,   // e.g. "T1059.004"
    pub mitre_tactic: Option<String>,         // e.g. "Execution"
    pub description: String,
    pub triggering_event_id: Uuid,
    pub threat_score: f32,         // 0.0 – 100.0
    pub status: AlertStatus,       // Open | Acknowledged | Dismissed
}
```

---

## 9. API Backend — Deep Dive

### Architecture

```
edr-api-backend/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── state.rs                  ← Arc<AppState> with DB pools + WS broadcaster
│   │
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── auth.rs               ← POST /auth/login, POST /auth/refresh
│   │   ├── nodes.rs              ← GET /nodes, GET /nodes/:id
│   │   ├── logs.rs               ← GET /nodes/:id/logs
│   │   ├── alerts.rs             ← GET /alerts, PATCH /alerts/:id
│   │   ├── commands.rs           ← POST /nodes/:id/isolate, POST /nodes/:id/deisolate
│   │   └── ws.rs                 ← GET /ws (WebSocket upgrade)
│   │
│   ├── middleware/
│   │   ├── auth.rs               ← JWT extraction + validation layer
│   │   └── logging.rs
│   │
│   ├── db/
│   │   ├── nodes.rs
│   │   ├── logs.rs
│   │   └── alerts.rs
│   │
│   ├── kafka/
│   │   └── consumer.rs           ← consumes edr.alerts + edr.health, broadcasts to WS
│   │
│   └── error.rs
└── Dockerfile
```

### API Routes Reference

**Authentication**

| Method | Path | Description |
|---|---|---|
| POST | `/auth/login` | Operator login → returns JWT access + refresh token |
| POST | `/auth/refresh` | Refresh access token using refresh token |
| POST | `/auth/logout` | Invalidate refresh token |

**Nodes**

| Method | Path | Description |
|---|---|---|
| GET | `/nodes` | List all enrolled nodes with current status |
| GET | `/nodes/:id` | Single node detail (OS info, last seen, alert count, isolation state) |
| GET | `/nodes/:id/logs` | Paginated log query. Query params: `from`, `to`, `type`, `limit`, `offset` |

**Alerts**

| Method | Path | Description |
|---|---|---|
| GET | `/alerts` | List alerts. Query params: `severity`, `status`, `from`, `to`, `node_id` |
| GET | `/alerts/:id` | Single alert with full MITRE context |
| PATCH | `/alerts/:id` | Update alert status (`acknowledged` / `dismissed`) |

**Commands**

| Method | Path | Description |
|---|---|---|
| POST | `/nodes/:id/isolate` | Trigger node isolation. Body: `{ "reason": "string" }` |
| POST | `/nodes/:id/deisolate` | Remove node isolation |

**WebSocket**

| Path | Description |
|---|---|
| `GET /ws` | Upgrade to WebSocket. Server pushes `alert_created`, `node_status_changed`, `node_health` events |

### WebSocket Message Format

```json
{
  "event": "alert_created",
  "data": {
    "alert_id": "uuid",
    "node_id": "uuid",
    "severity": "High",
    "mitre_technique_id": "T1059.004",
    "description": "Bash reverse shell detected",
    "timestamp": "2025-01-01T00:00:00Z"
  }
}
```

```json
{
  "event": "node_status_changed",
  "data": {
    "node_id": "uuid",
    "hostname": "prod-server-01",
    "status": "isolated",
    "timestamp": "2025-01-01T00:00:00Z"
  }
}
```

### Key Crates

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
axum = { version = "0.7", features = ["ws", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "compression-gzip"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls", "uuid", "chrono"] }
rdkafka = { version = "0.36", features = ["cmake-build"] }
jsonwebtoken = "9"
argon2 = "0.5"                  # password hashing for operator accounts
tokio-tungstenite = "0.21"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "1"
edr-sdk = { git = "...", tag = "v0.1.0" }
```

---

## 10. Frontend Dashboard — Deep Dive

### Stack

- **React 18** + **TypeScript** + **Vite** (fast dev builds)
- **TailwindCSS** — utility-first styling
- **TanStack Query (React Query)** — server state management, auto-refetch
- **Zustand** — lightweight client state (auth token, UI state)
- **React Router v6** — routing
- **Recharts** — charts for alert trends and threat scores
- **TanStack Table** — virtualized log tables (handles 10k+ rows)

### Directory Structure

```
edr-frontend/
├── src/
│   ├── main.tsx
│   ├── App.tsx                   ← router setup
│   │
│   ├── api/                      ← typed API client wrappers
│   │   ├── client.ts             ← axios instance with JWT interceptor
│   │   ├── nodes.ts
│   │   ├── alerts.ts
│   │   ├── logs.ts
│   │   └── auth.ts
│   │
│   ├── hooks/                    ← React Query hooks
│   │   ├── useNodes.ts
│   │   ├── useAlerts.ts
│   │   ├── useLogs.ts
│   │   └── useWebSocket.ts       ← WS connection manager with reconnect
│   │
│   ├── store/                    ← Zustand stores
│   │   ├── authStore.ts          ← JWT token, user info
│   │   └── uiStore.ts            ← selected node, active filters
│   │
│   ├── pages/
│   │   ├── LoginPage.tsx
│   │   ├── DashboardPage.tsx
│   │   ├── NodeMapPage.tsx       ← grid/list of all nodes
│   │   ├── NodeDetailPage.tsx    ← single node logs + alerts
│   │   ├── AlertsPage.tsx        ← alerts panel with filters
│   │   └── LiveLogsPage.tsx      ← real-time log stream
│   │
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   └── TopBar.tsx
│   │   ├── nodes/
│   │   │   ├── NodeCard.tsx
│   │   │   ├── NodeStatusBadge.tsx
│   │   │   └── IsolateButton.tsx
│   │   ├── alerts/
│   │   │   ├── AlertRow.tsx
│   │   │   ├── SeverityBadge.tsx
│   │   │   └── MitreTechniqueTag.tsx
│   │   └── logs/
│   │       ├── LogTable.tsx
│   │       └── LogTypeFilter.tsx
│   │
│   └── types/                    ← TypeScript interfaces mirroring Rust types
│       ├── node.ts
│       ├── alert.ts
│       └── event.ts
│
├── index.html
├── vite.config.ts
├── tailwind.config.js
├── tsconfig.json
└── Dockerfile                    ← nginx serving the static build
```

### Views — Implementation Notes

**Node Map** — polls `GET /nodes` every 30s via React Query. Nodes displayed as cards with colour-coded status (green = healthy, yellow = degraded, red = isolated). Clicking a card navigates to NodeDetailPage.

**Live Logs** — connects to `GET /ws` WebSocket via `useWebSocket` hook. Incoming `log_event` messages appended to a circular buffer (max 500 entries). TanStack Table renders with row virtualisation so DOM does not explode. Filter bar filters client-side.

**Alerts Panel** — `GET /alerts` with server-side filtering. Each row shows node hostname, MITRE technique ID, severity badge, timestamp. Acknowledge / Dismiss buttons call `PATCH /alerts/:id`. Unacknowledged alert count shown in sidebar badge updated via WebSocket.

**Node Controls** — `IsolateButton` shows confirmation modal before calling `POST /nodes/:id/isolate`. Optimistic UI update (card immediately shows "Isolating..."), reconciled when WebSocket delivers `node_status_changed`.

---

## 11. EDR SDK — Deep Dive

The SDK is a Rust library crate that is a **compile-time contract** between all services.

```
edr-sdk/
├── Cargo.toml
├── build.rs                      ← compiles .proto files via tonic-build
├── proto/
│   ├── fleet.proto
│   ├── agent.proto
│   └── events.proto
└── src/
    ├── lib.rs
    ├── types/
    │   ├── mod.rs
    │   ├── node.rs               ← Node, NodeStatus, NodeConfig
    │   ├── event.rs              ← NormalisedEvent, EventPayload variants
    │   ├── alert.rs              ← Alert, Severity, AlertSource
    │   └── auth.rs               ← Claims (JWT payload struct)
    └── proto/
        └── mod.rs                ← re-exports generated tonic code
```

### Versioning Strategy

The SDK uses semantic versioning strictly:
- **Patch** (0.1.1): adding optional fields to existing structs
- **Minor** (0.2.0): adding new message types, backwards-compatible proto changes
- **Major** (1.0.0): breaking changes to existing message fields or RPC signatures

When `edr-sdk` releases a new tag, each consuming service opens a PR to bump the dependency. The PR will fail to compile if the service has not updated its usage of changed types — this is intentional. Breaking changes are caught at compile time, not at runtime.

---

## 12. Docker & Container Strategy

### Rust Service Dockerfile Template

Use multi-stage builds to produce minimal images. Final image is `debian:bookworm-slim`, not `scratch`, because Rust services link against `libssl` and `libpq`.

```dockerfile
# Stage 1: Build
FROM rust:1.78-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
# Cache dependencies layer
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

COPY --from=builder /build/target/release/edr-fleet-server /usr/local/bin/
RUN chmod +x /usr/local/bin/edr-fleet-server

USER edr
EXPOSE 50051 8080

ENTRYPOINT ["edr-fleet-server"]
```

### Frontend Dockerfile

```dockerfile
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine AS runtime
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
```

### Agent Dockerfile (for testing only — real deployment is a binary)

```dockerfile
FROM rust:1.78-slim-bookworm AS builder
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev cmake clang llvm \
    linux-headers-generic \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY . .
RUN cargo build --release --bin agent-core
```

> **Note on the real agent**: In production, the agent is a statically-linked binary (`RUSTFLAGS="-C target-feature=+crt-static"`) installed via a `.deb` or `.rpm` package. It is **not** run inside a container on the monitored endpoint — eBPF probes need direct access to the host kernel.

---

## 13. Inter-Service Communication

### Communication Matrix

| From | To | Protocol | Notes |
|---|---|---|---|
| Agent | Fleet Server | gRPC / TLS (mTLS) | Bidirectional streaming |
| Fleet Server | Kafka | Producer (rdkafka) | `edr.events.raw`, `edr.health` |
| Kafka Pipeline | PostgreSQL logs | sqlx | Bulk inserts, batched |
| Rule Engine | Kafka | Consumer (rdkafka) | `edr.events.norm` |
| Rule Engine | Kafka | Producer (rdkafka) | `edr.alerts` |
| Rule Engine | PostgreSQL alerts | sqlx | Alert writes |
| API Backend | PostgreSQL (all 3) | sqlx | Read-mostly |
| API Backend | Kafka | Consumer (rdkafka) | `edr.alerts`, `edr.health` |
| API Backend | Fleet Server | HTTP (internal) | Relay isolation commands |
| Frontend | API Backend | REST + WebSocket | JWT auth |

### mTLS for Agent ↔ Fleet Server

Every agent has a unique certificate signed by an internal CA. The Fleet Server validates the client certificate on connection. This prevents rogue agents from injecting data.

Certificate lifecycle:
1. On first enrollment, agent generates a keypair and sends a CSR in the `RegisterRequest`
2. Fleet Server signs it with the internal CA and returns the cert in `RegisterResponse`
3. Subsequent connections present this cert — the JWT token is a second layer, not a replacement

---

## 14. Database Design

### edr_nodes_db — Node Registry

```sql
-- migrations/001_create_nodes.sql
CREATE TABLE nodes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hostname    VARCHAR(255) NOT NULL,
    os_version  VARCHAR(255),
    agent_version VARCHAR(50),
    machine_id  VARCHAR(64) UNIQUE NOT NULL,
    enrolled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen   TIMESTAMPTZ,
    status      VARCHAR(20) NOT NULL DEFAULT 'online',
                -- 'online' | 'offline' | 'isolated' | 'degraded'
    ip_address  INET,
    CONSTRAINT status_check CHECK (status IN ('online','offline','isolated','degraded'))
);

CREATE TABLE agent_configs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id     UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    config      JSONB NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE pending_commands (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id     UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    command     JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered   BOOLEAN NOT NULL DEFAULT FALSE,
    delivered_at TIMESTAMPTZ
);

CREATE INDEX idx_nodes_status ON nodes(status);
CREATE INDEX idx_pending_commands_undelivered ON pending_commands(node_id, delivered) WHERE delivered = FALSE;
```

### edr_logs_db — Event Storage

```sql
-- Use TimescaleDB extension for time-series performance (optional but recommended)
CREATE TABLE events (
    id              UUID NOT NULL DEFAULT gen_random_uuid(),
    node_id         UUID NOT NULL,
    event_type      VARCHAR(30) NOT NULL,
                    -- 'process' | 'file' | 'network' | 'osquery'
    timestamp       TIMESTAMPTZ NOT NULL,
    hostname        VARCHAR(255) NOT NULL,
    payload         JSONB NOT NULL,
    sequence_id     VARCHAR(64)
) PARTITION BY RANGE (timestamp);

-- Create monthly partitions
CREATE TABLE events_2025_01 PARTITION OF events
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');

CREATE INDEX idx_events_node_time ON events(node_id, timestamp DESC);
CREATE INDEX idx_events_type ON events(event_type, timestamp DESC);
CREATE INDEX idx_events_payload ON events USING GIN(payload);
```

### edr_alerts_db — Alerts

```sql
CREATE TABLE alerts (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id             UUID NOT NULL,
    hostname            VARCHAR(255) NOT NULL,
    timestamp           TIMESTAMPTZ NOT NULL,
    severity            VARCHAR(20) NOT NULL,
    source              VARCHAR(20) NOT NULL,
    mitre_technique_id  VARCHAR(20),
    mitre_tactic        VARCHAR(100),
    description         TEXT NOT NULL,
    triggering_event_id UUID,
    threat_score        REAL NOT NULL DEFAULT 0.0,
    status              VARCHAR(20) NOT NULL DEFAULT 'open',
    acknowledged_at     TIMESTAMPTZ,
    acknowledged_by     VARCHAR(100)
);

CREATE INDEX idx_alerts_node ON alerts(node_id, timestamp DESC);
CREATE INDEX idx_alerts_severity ON alerts(severity, status);
CREATE INDEX idx_alerts_status ON alerts(status, timestamp DESC);
CREATE INDEX idx_alerts_open ON alerts(status, timestamp DESC) WHERE status = 'open';
```

---

## 15. GitHub Actions CI/CD

### Template for Rust Services (`.github/workflows/ci.yml`)

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: Format check
        run: cargo fmt --all -- --check
      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

  test:
    name: Test
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_PASSWORD: testpass
          POSTGRES_DB: edr_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run tests
        run: cargo test --all
        env:
          DATABASE_URL: postgres://postgres:testpass@localhost:5432/edr_test

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install cargo-audit
        run: cargo install cargo-audit
      - name: Audit dependencies
        run: cargo audit

  build:
    name: Build & Push Image
    runs-on: ubuntu-latest
    needs: [lint, test, security]
    if: github.ref == 'refs/heads/main' || github.ref == 'refs/heads/develop'
    steps:
      - uses: actions/checkout@v4
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3
      - name: Login to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          push: true
          tags: ghcr.io/${{ github.repository }}:${{ github.sha }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  scan-image:
    name: Scan Image
    runs-on: ubuntu-latest
    needs: [build]
    steps:
      - name: Run Trivy
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: ghcr.io/${{ github.repository }}:${{ github.sha }}
          format: table
          exit-code: 1
          severity: CRITICAL,HIGH
```

---

## 16. Phase Roadmap

### Phase 0 — Repository & Infrastructure Foundation
**Goal**: Every repo exists, branch protection is on, infra containers run locally.

- [ ] Create all 8 repos on GitHub
- [ ] Initialize `edr-sdk` with proto files and shared types (v0.1.0 tag)
- [ ] Initialize `edr-infra` with `docker-compose.yml` (Kafka + 3x PostgreSQL)
- [ ] Add PR templates + issue templates to all repos
- [ ] Set branch protection rules on `main` and `develop`
- [ ] Verify `docker-compose up` starts Kafka, all PostgreSQL instances, and Kafka UI

**Milestone complete when**: `docker-compose up` works, all repos have `develop` branch, CI skeleton passes.

---

### Phase 1 — Agent: OSQuery Integration
**Goal**: Agent reads from OSQuery socket and logs results locally.

- [ ] `edr-agent` Cargo workspace scaffolded with all 6 crates
- [ ] `osquery-client` crate connects to OSQuery unix socket
- [ ] Scheduled query execution (hardcoded queries initially)
- [ ] Results serialised to `NormalisedEvent` using `edr-sdk` types
- [ ] `event-buffer` crate stores events to `sled` on disk

---

### Phase 2 — Agent: eBPF Probes
**Goal**: eBPF programs compile and stream kernel events to agent.

- [ ] `process_probe.bpf.c` compiles via `aya-build`
- [ ] `ebpf-collector` crate loads and attaches probes
- [ ] Perf buffer events parsed and converted to `ProcessEvent`, `FileEvent`, `NetworkEvent`
- [ ] Events flow into `event-buffer`

---

### Phase 3 — Fleet Server: Enrollment & Streaming
**Goal**: Agent can enroll, receive config, and stream events to Fleet Server.

- [ ] `edr-fleet-server` gRPC server implements `RegisterAgent` and `EventStream`
- [ ] PostgreSQL `edr_nodes_db` migrations run on startup
- [ ] Agent enrolls, receives JWT + config
- [ ] Bidirectional stream established, events flow from agent to fleet server
- [ ] Fleet server produces events to Kafka `edr.events.raw`

---

### Phase 4 — Kafka Pipeline & Database
**Goal**: Events flow from Kafka into PostgreSQL, normalised.

- [ ] `edr-kafka-pipeline` consumes `edr.events.raw`
- [ ] Normaliser converts raw bytes to `NormalisedEvent`
- [ ] Events written to `edr_logs_db`
- [ ] Produces to `edr.events.norm`

---

### Phase 5 — Rule Engine: YARA + MITRE
**Goal**: Alerts generated for suspicious events.

- [ ] `edr-rule-engine` consumes `edr.events.norm`
- [ ] YARA rules loaded from `/etc/edr/rules/`
- [ ] MITRE ATT&CK mapping lookup table
- [ ] Alerts published to `edr.alerts` and written to `edr_alerts_db`

---

### Phase 6 — API Backend
**Goal**: REST API and WebSocket serving frontend.

- [ ] All REST endpoints implemented and tested
- [ ] JWT auth middleware
- [ ] WebSocket server broadcasting alerts and health events
- [ ] Kafka consumer connected to `edr.alerts` + `edr.health`

---

### Phase 7 — Frontend Dashboard
**Goal**: Operator can view nodes, logs, and alerts in a browser.

- [ ] Auth flow (login page, JWT storage in memory)
- [ ] Node Map page with real-time status
- [ ] Alerts panel with filtering and acknowledge/dismiss
- [ ] Live Logs page over WebSocket
- [ ] Node isolation control with confirmation

---

### Phase 8 — Node Isolation End-to-End
**Goal**: Operator can isolate a node from the dashboard and the agent applies IPTables rules.

- [ ] `POST /nodes/:id/isolate` writes to `pending_commands`
- [ ] Fleet Server detects pending command and sends `IsolateCommand` over existing stream
- [ ] Agent `isolation` crate applies IPTables rules
- [ ] Status flows back via heartbeat → WebSocket → dashboard

---

### Phase 9 — Hardening, CI/CD, and Observability
**Goal**: Production-ready pipelines, security scanning, structured logging.

- [ ] Full GitHub Actions CI on all repos (lint, test, audit, build, Trivy scan)
- [ ] Structured JSON logging with correlation IDs across all services
- [ ] Prometheus metrics endpoints on all Rust services
- [ ] Grafana dashboards for event throughput, alert rate, node health
- [ ] All secrets in environment variables, no hardcoded credentials
- [ ] Runbooks written for each service in `edr-infra/docs/`

---

*End of EDR Implementation Guide — AXIOM*
