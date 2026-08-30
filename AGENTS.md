# AGENTS.md — Agent & Developer Operating Instructions

Welcome to the **Aigis-Zero EDR** repository. This document defines the quality gates, development workflows, script usage rules, and architectural standards for all AI agents and engineers working in this codebase.

---

## 1. Mandatory Quality Gates

Before completing any task or pushing code:

1. **Zero Warnings Policy:** Every crate must compile with zero warnings:
   ```bash
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```
2. **Strict Import Grouping & Formatting:** Code must be formatted using Nightly rustfmt (which enforces standard import grouping: `std` $\to$ external crates $\to$ `crate`):
   ```bash
   cargo +nightly fmt --all
   ```
3. **Compulsory Test & Build Verification:** All workspace targets and tests must pass:
   ```bash
   cargo test --workspace --all-features
   ```
4. **Typos & Documentation Check:** Ensure zero typos across code and documentation:
   ```bash
   ./scripts/check.sh
   ```
5. **Changelog Updates:** Any functional change, feature addition, bug fix, or breaking change must be recorded in `CHANGELOG.md` under `[Unreleased]`.


---

## 2. Scripts Directory Reference & Usage Rules

The `scripts/` directory contains standard tooling for local development and CI verification. **Always use these scripts rather than running ad-hoc commands.**

| Script | When to Use | Execution Command | Behavior & Rules |
|---|---|---|---|
| **`check.sh`** | **Daily development & before finishing any task.** | `./scripts/check.sh` or `./scripts/check.sh --fix` | Runs `rustfmt` (via `+nightly`), `clippy` (with `-D warnings`), `typos`, `cargo build`, and `cargo test`. Use `--fix` to automatically format imports and auto-fix clippy suggestions. |
| **`ci.sh`** | **Pre-push verification.** | `./scripts/ci.sh` | Strictly mirrors GitHub Actions CI. Runs non-destructive format checks, clippy, typos, full build, test suite, documentation tests (`cargo doc`), and security audit (`cargo audit`). |
| **`setup.sh`** | **Initial workspace onboarding or tool upgrade.** | `./scripts/setup.sh` | Cross-platform dependency installer for macOS (Homebrew) and Linux (apt, dnf, pacman, apk). Installs system libraries (`libpq`, `openssl`, `pkg-config`, `protobuf`, `cmake`), Rust nightly toolchain components, locked cargo tools (`typos-cli`, `cargo-audit`, `cargo-cache`, `sqlx-cli`), and marks all shell scripts executable. |
| **`precommit.sh`** | **Repository setup.** | `./scripts/precommit.sh` | Installs `./scripts/check.sh` as a Git pre-commit hook in `.git/hooks/pre-commit` to prevent committing broken code. |
| **`infra.sh`** | **One-command infrastructure startup, seeding, and teardown.** | `./scripts/infra.sh` or `./scripts/infra.sh [up\|down\|reset\|seed\|status]` | Boots all Docker containers (PostgreSQL, Kafka, Zookeeper, Kafka UI), waits for health checks, provisions Kafka topics, and applies DDL and mock seed data to all databases. |
| **`seed.sh`** | **Local database reset or demo data seeding.** | `./scripts/seed.sh` or `./scripts/seed.sh --reset` | Connects to PostgreSQL using `.env` variables and applies migrations and test fixtures from `fleet-server/migrations/seed.sql`. Use `--reset` to drop, recreate, and reseed databases. |
| **`clean.sh`** | **Disk cleanup or resolving build cache corruption.** | `./scripts/clean.sh` or `./scripts/clean.sh --deep` | Removes `target/`, temporary `*.rs.bk`, `*.orig` files, and optionally prunes the local cargo registry cache with `--deep`. |
| **`infra/scripts/create-topics.sh`** | **Local Kafka cluster initialization.** | `./infra/scripts/create-topics.sh` | Provisions Kafka topics (`aigis.events.*`, `aigis.alerts`, `aigis.heartbeats`, `aigis.events.dlq`) with their defined partitions and retention policies inside Docker. |

---

## 3. Rust Codebase Architecture & Coding Standards

### A. Subsystem Overview
* **`api-backend/`**: Axum 0.8 REST & WebSocket gateway. Uses `diesel-async` with `deadpool` for PostgreSQL and RdKafka for streaming live feeds.
* **`fleet-server/`**: Tonic gRPC controller handling node enrollment, agent authentication, and telemetry forwarding. Uses `diesel-async` with `deadpool`.
* **`kafka-pipeline/`**: Event router & normalizer fanning out `aigis.events.raw` into typed topics.
* **`rule-engine/`**: Pure-Rust YARA-X scanning and MITRE ATT&CK alert generation.
* **`agent/`**: Endpoint agent binary running osquery Thrift polling, SQLite WAL buffering, and `nftables` isolation.
* **`sdk/`**: Shared Protobuf schemas (`.proto`) and shared data structures. Strictly domain models; no business logic.
* **`frontend/`**: React + TypeScript SOC dashboard (Vite).
* **`infra/`**: Docker Compose and Kubernetes manifests.

---

### B. Idiomatic Rust Requirements

1. **No `.unwrap()` or `.expect()` in Runtime Paths:**
   * All fallible operations must return a typed `Result<T, AppError>` or bubble errors via `?`.
   * `.expect()` is permitted strictly in tests or true program invariants with an explanatory comment.

2. **Ownership & Cheap Cloning (`Arc`):**
   * Use `Arc` for shared state (e.g. `AppState`, connection pools, broadcast senders).
   * Avoid defensive `.clone()` on heavy data structures; borrow with references (`&str`, `&[T]`) when possible.

3. **Layered Decoupling:**
   * **Handlers:** Parse HTTP/WS parameters, call services, and return responses. **No direct database queries inside handlers.**
   * **Services:** Contain business rules and coordinate repositories and external clients.
   * **Repositories:** Manage database access (`diesel-async` or `sqlx`).
   * **Streaming:** Background tasks pipe events into `tokio::sync::broadcast` for WebSockets. Live telemetry never blocks on database writes.

4. **Async Best Practices:**
   * Never use blocking I/O (`std::thread::sleep`, sync file I/O, synchronous mutex locks) across `.await` points.
   * Use Tokio primitives (`tokio::time::sleep`, `tokio::sync::Mutex`, `tokio::select!`).

---

## 4. Changelog Maintenance & Humanizer Rules

Every agent and developer must maintain `CHANGELOG.md` following [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) conventions and the repository Humanizer style rules:

### A. Entry Format & Categorization
All new unreleased work belongs under `## [Unreleased]`, grouped into standard categories:
* `### Added` — New capabilities, endpoints, or crates.
* `### Changed` — Functional changes, breaking migrations, or updated behaviors.
* `### Fixed` — Bug fixes, error handling corrections, or build repairs.
* `### Removed` — Deprecated or removed components.
* `### Security` — Vulnerability mitigations and auth hardening.

Prefix every entry with its package name in bold:
```markdown
- **<package-name>**: <concise plain-language summary of what changed for consumers>
```

### B. Humanizer Style Rules
To keep changelog entries clear and natural:
1. **No Em or En Dashes:** Do not use `—` or `–` in changelog text; use colons, commas, or parentheses instead.
2. **No Promotional Fluff:** Avoid words like *robust*, *seamless*, *cutting-edge*, *pivotal*, *crucial*, or *groundbreaking*.
3. **Consumer Perspective:** Write what changed from a developer or operator perspective rather than internal code refactor steps.
4. **Breaking Changes:** Prefix breaking modifications with `⚠️ BREAKING:` and include a brief migration note.

---

## 5. Virtual Engineering Team & Agent Orchestration

The repository maintains an autonomous **Virtual Engineering Team** located in `.agents/engineering-team/` and `.agents/skills/`. All agents function as specialized software engineers ("employees") orchestrated by the Lead Agent.

### A. Team Roster & Specialization Matrix

| Agent / Role | Directory / Skill Path | Focus Areas & Subsystems | Primary Scripts & Automation |
|---|---|---|---|
| **Senior Architect** | `.agents/engineering-team/senior-architect/` | System architecture, LLD specs, Kafka topic topologies, DB schemas, component boundaries | `project_architect.py`, `dependency_analyzer.py`, `architecture_diagram_generator.py` |
| **Senior Rust Systems Engineer** | `.agents/skills/rust-senior-engineer/` | High-throughput async Rust, zero-copy pipelines, lock-free safety, Tokio optimization, Zero-Warning clippy | `scripts/check.sh`, `cargo clippy`, `cargo +nightly fmt` |
| **EDR Agent Developer** | `.agents/engineering-team/edr-agent-developer/` | Endpoint telemetry daemon, osquery Thrift extension IPC, SQLite WAL ring buffer, `nftables` isolation, Tonic gRPC streaming | Mock gRPC servers, `nftables` validation, SQLite recovery benchmarks |
| **Senior Backend Developer** | `.agents/engineering-team/backend-developer/` | Axum 0.8 REST & WebSocket gateway, Tonic gRPC fleet server, diesel-async repositories, JWT auth, middleware | `api_scaffolder.py`, `backend_decision_engine.py`, `api_load_tester.py`, `scripts/seed.sh` |
| **Kafka & Data Pipeline Engineer** | `.agents/engineering-team/data-engg-kafka/` | Kafka event routing (`aigis.events.*`), normalization, partition keys, DLQ handling, RdKafka tuning | `pipeline_orchestrator.py`, `data_quality_validator.py`, `etl_performance_optimizer.py` |
| **Senior Security Engineer** | `.agents/engineering-team/senior-security-engg/` | Threat modeling, MITRE ATT&CK mappings, YARA-X signature engineering, secret audits, host quarantine verification | `threat_modeler.py`, `secret_scanner.py`, `cargo audit` |
| **Machine Learning Engineer** | `.agents/engineering-team/ml-engg/` | Behavioral anomaly detection, statistical scoring, event classification, ML pipelines | `rag_system_builder.py`, `ml_monitoring_suite.py`, `model_deployment_pipeline.py` |
| **Test & QA Engineer** | `.agents/engineering-team/test-qa/` | Integration tests, property-based tests, Criterion benchmarks, CI test suites, mock services | `test_suite_generator.py`, `coverage_analyzer.py`, `e2e_test_scaffolder.py`, `cargo test` |
| **Code Reviewer** | `.agents/engineering-team/code-reviewer/` | Static analysis verification, anti-pattern detection, zero-warning compliance, idiomatic reviews | `code_quality_checker.py`, `pr_analyzer.py`, `review_report_generator.py` |
| **Zero-Hallucination Coder** | `.agents/engineering-team/hallucination-checker/` | 5-phase loop (Discuss $\to$ Map $\to$ Decompose $\to$ Execute $\to$ Verify), YAGNI / Ponytail elimination ladder | 5-phase loop, Ponytail ladder |
| **Senior Prompt Engineer** | `.agents/engineering-team/senior-prompt-engg/` | System prompt design, evaluation matrices, context optimization, subagent prompt authoring | `prompt_optimizer.py`, `rag_evaluator.py`, `agent_orchestrator.py` |
| **Documentation Specialist** | `.agents/skills/rust-doc-agent/`, `humanizer/` | Crate docstrings, doctests, READMEs, architectural documentation, humanizer style checks | `rust-doc-agent`, `humanizer`, `readme-humanizer` |

### B. Orchestration & Delegation Lifecycle

```
[Requirement / Feature Request]
              |
              v
     [Lead Orchestrator]
              |
     +--------+---------------------------------------+
     | 1. Design & RFC                                | 2. Threat & Schema Review
     v                                                v
[Senior Architect]                           [Senior Security Engineer]
     |                                                |
     +--------+---------------------------------------+
              |
              v
   [Zero-Hallucination Coder] (Decompose into atomic stories)
              |
     +--------+-----------------------+-----------------------+
     |                                |                       |
     v                                v                       v
[EDR Agent Dev]             [Senior Backend Dev]     [Data / Kafka Engg]
 (agent/* crates)            (api-backend, fleet)     (kafka-pipeline)
     |                                |                       |
     +--------+-----------------------+-----------------------+
              |
              v
     [Test & QA Engineer] (Integration tests & benchmarks)
              |
              v
     [Code Reviewer] (Strict quality check & clippy verification)
              |
              v
     [Doc Agent & Humanizer] (Docs, doctests, CHANGELOG.md)
```

### C. Subagent Spawning & Execution Rules

1. **Autonomous Delegation:**
   * Any agent or subagent may spawn additional specialized subagents (`invoke_subagent` or `define_subagent`) to parallelize work or isolate experimental code changes.
   * Agents may execute workspace scripts (`./scripts/check.sh`, `./scripts/infra.sh`, `./scripts/seed.sh`) or agent-specific tools (`.agents/engineering-team/*/scripts/*.py`) using standard shell runners.

2. **Inter-Agent Communication:**
   * Agents coordinate handoffs and exchange context using `send_message` or structured markdown artifacts.
   * Deliverables must include concrete file paths, line numbers, and verifiable test results.

3. **Strict Compliance:**
   * All subagents inherit the mandatory quality gates: zero warnings, nightly formatting, and passing test suites.


