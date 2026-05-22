# API Backend — Implementation Timeline

> **Phase**: 6 (REST API + WebSocket)
> **Priority**: 🟡 High — serves the frontend dashboard
> **Estimated Duration**: 6–7 days
> **Depends on**: `sdk v0.1.0`, infra running, data flowing through pipeline

---

## PR Plan

### PR #1 — Skeleton, config, AppState, and health endpoint
**Branch**: `feat/api-skeleton`
**Duration**: 1 day

**Files**:
- `src/main.rs` — axum router setup, server binding
- `src/config.rs` — env/config loading
- `src/state.rs` — `AppState` (3 DB pools, Kafka consumer handle, WS broadcaster)
- `src/error.rs` — unified API error responses

**Tasks**:
- [ ] Initialize 3 sqlx pools (logs, nodes, alerts DBs)
- [ ] Set up axum router with CORS and tracing middleware
- [ ] Health check endpoint (`GET /health`)
- [ ] Structured JSON error responses

### PR #2 — Auth routes and JWT middleware
**Branch**: `feat/api-auth`
**Duration**: 1.5 days
**Depends on**: PR #1

**Files**:
- `src/routes/auth.rs` — login, refresh, logout
- `src/middleware/auth.rs` — JWT extraction and validation layer

**Tasks**:
- [ ] `POST /auth/login` — validate credentials with argon2, return JWT pair
- [ ] `POST /auth/refresh` — validate refresh token, issue new access token
- [ ] `POST /auth/logout` — invalidate refresh token
- [ ] JWT middleware — extract Bearer token, validate, inject claims into request
- [ ] Seed initial operator account on first startup
- [ ] Unit tests for auth flow

### PR #3 — Node and log query routes
**Branch**: `feat/api-nodes-logs`
**Duration**: 1.5 days
**Depends on**: PR #2

**Files**:
- `src/routes/nodes.rs`, `src/routes/logs.rs`
- `src/db/nodes.rs`, `src/db/logs.rs`

**Tasks**:
- [ ] `GET /nodes` — list all nodes with status, last_seen, alert count
- [ ] `GET /nodes/:id` — single node detail
- [ ] `GET /nodes/:id/logs` — paginated event logs with filters (from, to, type, limit, offset)
- [ ] SQL query optimisation with indexes
- [ ] Unit tests with mock DB

### PR #4 — Alert routes and command routes
**Branch**: `feat/api-alerts-commands`
**Duration**: 1 day
**Depends on**: PR #3

**Files**:
- `src/routes/alerts.rs`, `src/routes/commands.rs`
- `src/db/alerts.rs`

**Tasks**:
- [ ] `GET /alerts` — filtered alert list (severity, status, date range, node_id)
- [ ] `GET /alerts/:id` — single alert with MITRE context
- [ ] `PATCH /alerts/:id` — update status (acknowledged/dismissed)
- [ ] `POST /nodes/:id/isolate` — write isolation command to `pending_commands`
- [ ] `POST /nodes/:id/deisolate` — write de-isolation command

### PR #5 — WebSocket and Kafka consumer
**Branch**: `feat/api-websocket`
**Duration**: 1.5 days
**Depends on**: PR #4

**Files**:
- `src/routes/ws.rs` — WebSocket upgrade handler
- `src/kafka/consumer.rs` — consumes `edr.alerts` + `edr.health`

**Tasks**:
- [ ] `GET /ws` — upgrade to WebSocket, authenticate via query param token
- [ ] Kafka consumer for `edr.alerts` → broadcast `alert_created` to all WS clients
- [ ] Kafka consumer for `edr.health` → broadcast `node_health` and `node_status_changed`
- [ ] Connection lifecycle management (heartbeat pings, cleanup on disconnect)
- [ ] Integration test: produce alert to Kafka → verify WS client receives it
