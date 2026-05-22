# ebpf-collector — Implementation Timeline

> **Phase**: 2 (Agent: eBPF Probes)
> **Priority**: 🟡 High — kernel-level telemetry
> **Estimated Duration**: 5–7 days (most complex agent crate)
> **Depends on**: `sdk v0.1.0`, `event-buffer`

---

## PR Plan

### PR #1 — eBPF loader and process probe
**Branch**: `feat/ebpf-process-probe`
**Duration**: 2.5 days

**Files**:
- `src/lib.rs` — public API
- `src/loader.rs` — loads compiled eBPF objects via aya
- `src/events.rs` — parses perf/ring buffer events
- `bpf/process_probe.bpf.c` — attaches to `sys_enter_execve`

**Tasks**:
- [ ] Write `process_probe.bpf.c` capturing PID, PPID, cmdline, UID, exe_path
- [ ] Configure `aya-build` in `build.rs` to compile BPF C programs
- [ ] Implement `EbpfLoader::load(probe_path)` — loads and attaches BPF program
- [ ] Implement ring buffer reader — async event polling via aya
- [ ] Parse raw BPF event bytes → `ProcessEvent` struct
- [ ] Unit tests for event parsing (mock raw bytes)

### PR #2 — File and network probes
**Branch**: `feat/ebpf-file-network`
**Duration**: 2 days
**Depends on**: PR #1

**Files**:
- `bpf/file_probe.bpf.c` — attaches to `sys_enter_openat`, `sys_enter_unlinkat`
- `bpf/network_probe.bpf.c` — attaches to `sys_enter_connect`, `sys_enter_bind`

**Tasks**:
- [ ] Write `file_probe.bpf.c` capturing file path, operation, PID, return code
- [ ] Write `network_probe.bpf.c` capturing src/dst IP:port, protocol, PID
- [ ] Add loader support for multiple probes running concurrently
- [ ] Parse file events → `FileEvent`, network events → `NetworkEvent`
- [ ] Implement probe attach/detach lifecycle management
- [ ] Test on real Linux kernel (requires root / CAP_BPF)

### PR #3 — Event aggregation and rate limiting
**Branch**: `feat/ebpf-aggregation`
**Duration**: 1.5 days
**Depends on**: PR #2

**Tasks**:
- [ ] Implement event deduplication (same process exec within 100ms)
- [ ] Add configurable ring buffer size (`ringbuf_size_pages`)
- [ ] Rate-limit high-frequency events (file I/O) to prevent flooding
- [ ] Metrics: `events_captured`, `events_dropped`, `probe_errors`
