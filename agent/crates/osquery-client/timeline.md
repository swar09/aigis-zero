# osquery-client — Implementation Timeline

> **Phase**: 1 (Agent: OSQuery Integration)
> **Priority**: 🟡 High — first data source to implement
> **Estimated Duration**: 3–4 days
> **Depends on**: `sdk v0.1.0`

---

## Overview

Connects to OSQuery's unix socket, executes scheduled queries, and returns structured results. This is the simplest collector and validates the entire event pipeline.

---

## PR Plan

### PR #1 — Unix socket client and connection management
**Branch**: `feat/osquery-socket-client`
**Duration**: 1.5 days

**Files**:
- `src/lib.rs` — module declarations, public API
- `src/client.rs` — unix socket connection, query execution, reconnect logic

**Tasks**:
- [ ] Implement `OsqueryClient` struct with unix socket path config
- [ ] Implement `connect()` — async connection to OSQuery extension socket
- [ ] Implement `query(sql: &str)` → returns `Vec<HashMap<String, String>>`
- [ ] Add connection health check and auto-reconnect with backoff
- [ ] Handle socket not found / permission denied errors gracefully
- [ ] Unit tests with mock socket

### PR #2 — Scheduled query execution and event conversion
**Branch**: `feat/osquery-scheduler`
**Duration**: 1.5 days
**Depends on**: PR #1

**Files**:
- `src/queries.rs` — scheduled query definitions, interval timer
- `src/lib.rs` — update public API

**Tasks**:
- [ ] Define `ScheduledQuery` struct (name, sql, interval_secs)
- [ ] Implement query scheduler using `tokio::time::interval`
- [ ] Convert OSQuery JSON results → `edr_sdk::types::OsqueryEvent`
- [ ] Wrap results in `NormalisedEvent` envelope
- [ ] Support dynamic query schedule updates (from fleet config push)
- [ ] Unit tests for query scheduling and event conversion
