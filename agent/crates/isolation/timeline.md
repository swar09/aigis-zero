# isolation — Implementation Timeline

> **Phase**: 8 (Node Isolation End-to-End)
> **Priority**: 🟢 Medium — depends on fleet-client command handling
> **Estimated Duration**: 2 days
> **Depends on**: `fleet-client` command dispatch

---

## PR Plan

### PR #1 — IPTables rule management
**Branch**: `feat/isolation-iptables`
**Duration**: 1.5 days

**Files**:
- `src/lib.rs` — public API (`IsolationManager`)
- `src/iptables.rs` — adds/removes iptables rules via `std::process::Command`

**Tasks**:
- [ ] Implement `IsolationManager::new(fleet_server_ip)` — stores allowed endpoint
- [ ] Implement `isolate()` — drops all traffic except to Fleet Server IP
- [ ] Implement `deisolate()` — removes isolation iptables rules
- [ ] Implement `is_isolated()` — checks current iptables state
- [ ] Add iptables rule validation (ensure rules are correctly applied)
- [ ] Handle permission errors gracefully (requires root/CAP_NET_ADMIN)
- [ ] Unit tests with mock `Command` executor
- [ ] Log all rule changes for audit trail

### PR #2 — Isolation status reporting
**Branch**: `feat/isolation-status`
**Duration**: 0.5 day
**Depends on**: PR #1

**Tasks**:
- [ ] Report isolation state in heartbeat status field
- [ ] Emit `node_status_changed` event on isolation toggle
- [ ] Integration test: receive IsolateCommand → apply rules → report status
