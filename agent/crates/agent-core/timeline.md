# agent-core — Implementation Timeline

> **Phase**: 1 (basic) → 2 (full) → 8 (isolation)
> **Priority**: 🔴 Critical — binary entry point, orchestrates all subsystems
> **Estimated Duration**: 3–4 days (spread across phases)
> **Depends on**: all other agent crates

---

## PR Plan

### PR #1 — Config loading and basic orchestrator (Phase 1)
**Branch**: `feat/agent-core-basic`
**Duration**: 1.5 days

**Files**:
- `src/main.rs` — tokio runtime bootstrap, signal handling
- `src/config.rs` — reads `/etc/edr/agent.toml`
- `src/orchestrator.rs` — spawns osquery-client and event-buffer tasks

**Tasks**:
- [ ] Parse `agent.toml` config file (fleet endpoint, buffer path, osquery socket)
- [ ] Set up tracing-subscriber with JSON structured logging
- [ ] Spawn `osquery-client` scheduled query task
- [ ] Spawn `event-buffer` flush task
- [ ] Spawn `fleet-client` enrollment + streaming task
- [ ] Wire channels: osquery-client → event-buffer → fleet-client
- [ ] Handle SIGTERM/SIGINT for graceful shutdown
- [ ] Integration test: config load → task spawn → shutdown

### PR #2 — Full orchestration with eBPF (Phase 2)
**Branch**: `feat/agent-core-ebpf`
**Duration**: 1 day
**Depends on**: `ebpf-collector` complete

**Tasks**:
- [ ] Spawn `ebpf-collector` probe tasks
- [ ] Wire eBPF events into the same event-buffer pipeline
- [ ] Add event-type tagging (source: ebpf vs osquery)
- [ ] Handle probe attach failures gracefully (agent continues without eBPF)

### PR #3 — Isolation command handling (Phase 8)
**Branch**: `feat/agent-core-isolation`
**Duration**: 0.5 day
**Depends on**: `isolation` crate, `fleet-client` command dispatch

**Tasks**:
- [ ] Listen for `IsolateCommand` from fleet-client command channel
- [ ] Invoke `isolation::IsolationManager` on command receipt
- [ ] Update heartbeat status to reflect isolation state
