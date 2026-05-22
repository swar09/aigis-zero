# fleet-client — Implementation Timeline

> **Phase**: 1 (initial) → 3 (full streaming)
> **Priority**: 🟡 High — agent's only outbound connection
> **Estimated Duration**: 3–4 days
> **Depends on**: `sdk v0.1.0`, `event-buffer`

---

## Overview

gRPC client that connects to the Fleet Server. Handles enrollment, bidirectional event streaming, heartbeats, and command reception (isolation, config updates).

---

## PR Plan

### PR #1 — gRPC channel and enrollment
**Branch**: `feat/fleet-enrollment`
**Duration**: 1.5 days

**Files**:
- `src/lib.rs` — public API
- `src/connection.rs` — gRPC channel with TLS, reconnect logic

**Tasks**:
- [ ] Implement `FleetConnection::new(endpoint, tls_config)` — creates tonic channel
- [ ] Implement TLS/mTLS configuration (load certs from paths)
- [ ] Implement `enroll(hostname, os_version, agent_version, machine_id)` → `RegisterResponse`
- [ ] Persist received `node_id` and JWT token to disk
- [ ] Implement exponential backoff reconnect (1s → 2s → 4s → ... → 60s cap)
- [ ] Unit tests with mock gRPC server

### PR #2 — Bidirectional event stream and command handling
**Branch**: `feat/fleet-streaming`
**Duration**: 1.5 days
**Depends on**: PR #1

**Files**:
- `src/stream.rs` — bidirectional streaming, command dispatch

**Tasks**:
- [ ] Implement `EventStream` — opens `FleetService::EventStream` RPC
- [ ] Send events from `event-buffer` in configurable batch sizes
- [ ] Receive `ServerCommand` messages (Isolate, ConfigUpdate, Ack)
- [ ] Dispatch commands to appropriate handlers via channels
- [ ] Handle stream disconnection → trigger reconnect + buffer drain
- [ ] Implement heartbeat sending on configurable interval
- [ ] Integration test: mock server ↔ fleet-client stream
