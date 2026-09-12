# Aigis-Zero Issue Log & Tracking Matrix

This document tracks all bugs, discrepancies, regressions, and verification records across the full-stack test execution lifecycle.

---

## Baseline Status (Phase 0)
- **Date/Time:** 2026-09-08T15:13:06Z
- **Branch:** `testing/full-system-verification-20260908`
- **Environment:** WSL2 Ubuntu (Kernel 6.6.87.2-microsoft-standard-WSL2, x86_64, systemd PID 1)
- **osquery:** `/usr/bin/osqueryd` active (running), Thrift socket `/var/osquery/osquery.em` (untouched)
- **Toolchain Checks (`./scripts/check.sh`):**
  - Nightly rustfmt: PASS
  - Clippy (`-D warnings`): PASS (0 warnings)
  - Typos: PASS (0 typos)
  - Cargo build (all targets, all features): PASS
  - Cargo test (all features): PASS (all tests pass)

---

## ISSUE-0-1: Stale Component Status and Legacy SQLx References in README.md
- **Environment:** WSL2 Ubuntu 24.04, kernel 6.6.87.2-microsoft-standard-WSL2, documentation
- **Repro steps:**
  1. Inspect `README.md` "Current State" table.
  2. Inspect actual source code in `rule-engine/src`, `api-backend/src`, and `CHANGELOG.md`.
  3. Inspect `README.md` build instructions referencing `SQLX_OFFLINE=true`.
- **Actual repro output:**
  - `README.md` marks `rule-engine` and `api-backend` as "stubbed" despite full implementations existing.
  - `README.md` references `sqlx` instructions even though database layer migrated to `diesel-async`.
- **Root cause:** README was not synchronized following major implementations and diesel-async migration (`README.md`).
- **Owning role (per AGENTS.md):** Documentation Specialist
- **Fix:** Pending final report and documentation refresh pass in Phase 11.
- **Verification:** Compare updated `README.md` against codebase architecture and `CHANGELOG.md`.
- **Regression check:** `./scripts/check.sh`
- **Status:** Open (Tracked for documentation consolidation in Phase 11)


---

## Phase 1 — Agent Installation (WSL Ubuntu)
- **Pre-flight Check (`/usr/sbin/aigis-zero --check`):** PASS (All checks OK: root, directories, BPF JIT, inotify watches, osqueryd, nftables)
- **Binary Permissions:** `/usr/sbin/aigis-zero` (0755, root:root) - PASS
- **Config & Directory Permissions:** `/etc/aigis-zero` (0700), `/var/lib/aigis-zero` (0700), `/var/log/aigis-zero` (0755) - PASS
- **osqueryd Socket Connectivity:** Tested against `/var/osquery/osquery.em` - PASS (connected cleanly)
- **osqueryd Daemon Status:** PID 463 remained active and untouched - PASS
- **Non-Root Execution Safety:** Failed cleanly with exit code 1 and descriptive error message - PASS

---

## ISSUE-1-1: install.sh and uninstall.sh Unconditionally Stop and Mutate Host osqueryd
- **Environment:** WSL2 Ubuntu 24.04, kernel 6.6.87.2-microsoft-standard-WSL2, `agent/install.sh`, `agent/uninstall.sh`
- **Repro steps:**
  1. Inspect `agent/install.sh` lines 154, 259-268, 314.
  2. Inspect `agent/uninstall.sh` lines 79-80, 100-115.
- **Actual repro output:**
  - `install.sh` issues `systemctl stop osqueryd.service` and overwrites `/etc/osquery/osquery.conf` and `/etc/osquery/osquery.flags`.
  - `uninstall.sh` stops and disables `osqueryd.service` and wipes `/var/osquery`.
- **Root cause:** Scripts assume osquery is exclusively managed by Aigis-Zero rather than checking if a working host osquery daemon already exists or accepting a flag (e.g. `--skip-osquery`).
- **Owning role (per AGENTS.md):** EDR Agent Developer
- **Fix:** Add check/flag to preserve existing osquery installations or skip osquery teardown if externally managed.
- **Verification:** Dry-run and review script logic.
- **Regression check:** `./scripts/check.sh`
- **Status:** Open (Tracked for installer script safety enhancement)


---

## Phase 2 — Agent → Fleet Server Telemetry Flow
- **gRPC Connectivity:** Successfully established to `http://127.0.0.1:50051` - PASS
- **Node Enrollment:** Machine `hawkins` enrolled into `edr_nodes` with UUID `b6b8bed7-decf-4a56-80de-1600b762f750` - PASS
- **Telemetry Streaming:** Live osquery telemetry batches forwarded via gRPC into Kafka topic `aigis.events.raw` - PASS
- **Heartbeat Loop:** Heartbeats recorded in `node_health` with advancing timestamps - PASS
- **Disconnection & Exponential Backoff:** On fleet-server shutdown, agent transitioned smoothly to exponential backoff (starting at ~50ms, capping at 12.8s) with 0 crashes and 0 busy-loops - PASS
- **SQLite WAL Spooling:** Generated events buffered locally in `/var/lib/aigis-zero/events.db` - PASS
- **Auto-Reconnection & Drain:** On fleet-server restoration, agent reconnected instantly, drained all 16 buffered events into Kafka (`aigis.events.raw`), and emptied the local SQLite buffer to 0 - PASS

---

## ISSUE-2-1: Fleet Server Port Collision with API Backend PORT in docker-compose.yml
- **Environment:** Docker Compose `infra/docker-compose.yml`, `fleet-server`
- **Repro steps:**
  1. Set `PORT=8088` or use default `PORT=8080` in `.env`.
  2. Inspect `fleet-server` settings loading logic (`Settings::load` parses `PORT`).
  3. Run `docker compose -f infra/docker-compose.yml up -d fleet-server`.
  4. Inspect `docker logs edr-fleet-server`.
- **Actual repro output:** `fleet server starting host=0.0.0.0 port=8088` instead of 50051, causing connection refused on host port 50051.
- **Root cause:** `infra/docker-compose.yml` lacked an explicit `PORT=50051` in `fleet-server.environment`, causing it to inherit the HTTP API port from `.env`.
- **Owning role (per AGENTS.md):** Senior Backend Developer
- **Fix:** Added `- PORT=50051` to `fleet-server` environment in `infra/docker-compose.yml`.
- **Verification:** Recreated container, confirmed `fleet server starting host=0.0.0.0 port=50051` and verified gRPC reachability on 50051.
- **Regression check:** `./scripts/check.sh`
- **Status:** Fixed-Verified

---

## ISSUE-2-2: aigis-zero.service Fails with 226/NAMESPACE when /run/osquery is Missing
- **Environment:** WSL2 Ubuntu 24.04, `agent/systemd/aigis-zero.service`
- **Repro steps:**
  1. Ensure `/run/osquery` does not exist (typical after system reboot).
  2. Start agent service: `sudo systemctl start aigis-zero.service`.
- **Actual repro output:** `aigis-zero.service: Failed to set up mount namespacing: /run/osquery: No such file or directory` (exit code 226/NAMESPACE).
- **Root cause:** `ReadWritePaths=` contained `/run/osquery` without the `-` (ignore-if-missing) prefix.
- **Owning role (per AGENTS.md):** EDR Agent Developer
- **Fix:** Prefixed `-/run/osquery` in `ReadWritePaths=` in `agent/systemd/aigis-zero.service`.
- **Verification:** Reloaded systemd daemon and started `aigis-zero.service`; service transitioned to `active (running)`.
- **Regression check:** `./scripts/check.sh`
- **Status:** Fixed-Verified


---

## Phase 3 — Fleet Server Functional Testing
- **Enrollment Idempotency:** Concurrent re-enrollment of the same machine produced identical `node_id` (`b6b8bed7-decf-4a56-80de-1600b762f750`), logged distinct `enrollment_events`, and created 0 duplicate rows in `nodes` table - PASS
- **Operator vs. Agent Status Separation:** Issued `POST /api/v1/nodes/{id}/isolate`; verified `operator_status = 'isolated'`. Subsequent heartbeat writes populated `node_health` with `agent_status = 'healthy'` but strictly preserved `operator_status = 'isolated'` without clobbering - PASS
- **JWT Correctness:**
  - Valid token accepted (HTTP 200)
  - Tampered token rejected (HTTP 401)
  - Expired/Malformed token rejected (HTTP 401)
  - Missing token rejected (HTTP 401) - PASS
- **Kafka Forwarding Correctness:** All events ingested via gRPC forwarded to Kafka `aigis.events.raw` with zero event drop - PASS
- **Enrollment Secret Validation:** Verified unit test coverage in `node_enrollment` and `grpc_listener` enforcing `x-enrollment-secret` / Bearer token validation - PASS
- **Concurrency Test:** Ran 5 concurrent agents registering simultaneously via gRPC; all 5 acquired distinct node UUIDs in 10ms with 0 leaks and 0 race condition crashes - PASS


---

## Phase 4 — Kafka + DB Testing (kafka-pipeline)
- **Topic Provisioning:** All canonical Kafka topics present with expected partitions (`aigis.events.raw`, `aigis.events.process`, `aigis.events.network`, `aigis.events.file`, `aigis.events.auth`, `aigis.events.dlq`, `aigis.alerts`, etc.) - PASS
- **Unit & Integration Test Suite:** `cargo test -p edr-kafka-pipeline` - PASS (0 failures)
- **Router Classification & Fan-Out:** Synthetic events published to `aigis.events.raw` were accurately categorized and routed:
  - `process_start` -> `aigis.events.process` (1)
  - `network_connect` -> `aigis.events.network` (1)
  - `file_create` -> `aigis.events.file` (1)
  - `user_login` -> `aigis.events.auth` (1)
  - Malformed non-JSON -> `aigis.events.dlq` (with tracing headers) - PASS
- **Dead-Letter Queue (DLQ) & Tracing Headers:** Corrupted/unclassified events forwarded to `aigis.events.dlq` carrying `x-original-topic`, `x-original-partition`, `x-original-offset`, and `x-error-message` metadata headers - PASS
- **Downstream Consumer Flow:** Live `edr-rule-engine` continuously consumed from typed topics (`aigis.events.process`, `aigis.events.network`, `aigis.events.file`, `aigis.events.auth`) - PASS
- **Health & Metrics Endpoints:** Confirmed `/healthz`, `/readyz`, and `/metrics` (Prometheus counters) in `edr-kafka-pipeline` - PASS
- **Known Scope Boundary:** Direct persistence of normalized telemetry into `edr_logs` is not part of `kafka-pipeline` crate design; logged as known architecture boundary.

---

## ISSUE-4-1: kafka-pipeline Service Omitted from docker-compose.yml
- **Environment:** `infra/docker-compose.yml`
- **Repro steps:**
  1. Inspect services listed in `infra/docker-compose.yml`.
  2. Notice `fleet-server`, `rule-engine`, and `api-backend` are present, but `kafka-pipeline` was missing.
- **Actual repro output:** Events in `aigis.events.raw` were not fanned out to typed topics unless `kafka-pipeline` was manually started.
- **Root cause:** Docker Compose configuration did not define the `kafka-pipeline` service.
- **Owning role (per AGENTS.md):** Kafka & Data Pipeline Engineer
- **Fix:** Added `kafka-pipeline` container service with health port mapping `8082:8082` and dependencies on `kafka` to `infra/docker-compose.yml`.
- **Verification:** Started via `docker compose -f infra/docker-compose.yml up -d kafka-pipeline`; confirmed consumer group `aigis-event-router` actively processing messages.
- **Regression check:** `./scripts/check.sh`
- **Status:** Fixed-Verified


---

## Phase 5 — Rule Engine Testing
- **Unit Test Suite:** `cargo test -p edr-rule-engine` - PASS (23 tests passed, 0 failures)
- **Rule Hot-Reload (SIGHUP):** Sent SIGHUP to `edr-rule-engine`; dynamic recompilation across 5 rule categories completed in <150ms via atomic pointer swap with zero dropped messages - PASS
- **True-Positive Detection:** Injected synthetic reverse shell simulation payload (`/dev/tcp/10.0.0.1/4444`); correctly triggered `Linux_Interactive_Reverse_Shell`:
  - Alert persisted to `edr_alerts` in PostgreSQL - PASS
  - Alert broadcast to Kafka topic `aigis.alerts` - PASS
  - MITRE Technique `T1059.004` and tactic `Execution` attached - PASS
  - Threat score `95.0` and severity `critical` populated - PASS
- **Sharded LRU Deduplication:** Streamed 5 identical triggering events within the 60s window; metrics confirmed `aigis_alerts_generated_total 6`, `aigis_alerts_suppressed_total 5`, `aigis_alerts_persisted_total 1` - PASS
- **True-Negative Baseline:** Streamed 10 benign utility execution events (`git status`, `cargo check`, `uptime`, etc.); zero false alerts produced (`count = 0`) - PASS
- **Throughput Burst Test:** Injected burst of 100 events into `aigis.events.process`; scanner processed queue with 0 lag, 0 dropped events, and 0 panics - PASS
- **Prometheus Metrics:** Verified live metrics matching exact pipeline state at `http://localhost:8081/metrics` - PASS


---

## Phase 6 — False Positive Analysis & Detection Quality
- **Labeled Corpus Evaluation:**
  - **Known-Benign (Host Telemetry):** Continuous background osquery ingestion across 100+ native WSL processes and sockets produced **0 false alerts** (0.0% false positive rate).
  - **Known-Benign (CLI Execution):** Standard toolchains (`cargo`, `git`, `systemctl`, `journalctl`, `ls`, single `whoami`) produced **0 false alerts**.
  - **Known-Technique (T1059.004 Reverse Shell):** Triggered critical severity alert with 95.0 threat score and technique mapping.
  - **Known-Technique (Multi-Indicator Reconnaissance):** Triggered medium severity alert when 4+ recon strings matched simultaneously, while cleanly ignoring isolated commands.
- **Scorecard Summary:**
  - True Positives: 2 / 2 (100% true positive rate)
  - False Positives: 0 (0.0% false positive rate)
  - Rule Tuning Needed: None required (existing rulesets already enforce strict multi-condition heuristics).

