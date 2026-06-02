# GitHub Issues — project-edr

---

## Issue: Wire the ebpf-collector crate into the workspace build and establish the aya build pipeline
**Labels:** `ebpf`, `kernel`, `scaffolding`, `unsafe`
**Depends on:** none
**Blocks:** BPF map definitions issue; process-lifecycle probe issue; network probe issue; userspace loader issue

### What this is
The `ebpf-collector` crate exists at `agent/crates/ebpf-collector/` with `aya = "0.13"` and `aya-build = "0.1"` declared in its `Cargo.toml`, but it is explicitly excluded from the workspace root `Cargo.toml` (`exclude = ["agent/crates/ebpf-collector"]`). Its `src/lib.rs` is a two-line comment stub. The crate has no `build.rs`, no BPF program directory, and produces nothing. This issue establishes the complete build foundation: workspace integration, cross-compilation target configuration, `build.rs` that compiles BPF C programs via `aya-build`, directory layout for BPF sources, and verification that the whole thing compiles against a BTF-enabled kernel (≥5.8).

### What is currently blocking this
Nothing external. This is the root of the eBPF workstream. The `agent/.cargo/config.toml` already has `[target.bpfel-unknown-none] rustflags = ["-C", "link-arg=--btf"]`, confirming intent. The blocker is the missing workspace membership, missing `build.rs`, and the empty crate body.

### What this is blocking
Every downstream eBPF issue. BPF map definitions, probe implementations, and the userspace loader all depend on the build pipeline this issue establishes.

### Implementation tasks
- [ ] Remove `"agent/crates/ebpf-collector"` from the `exclude` list in the workspace root `Cargo.toml` and add it to `members`. Verify `cargo check -p ebpf-collector` compiles.
- [ ] Create `agent/crates/ebpf-collector/build.rs` that calls `aya_build::build()` or equivalent to compile BPF C programs from `bpf/` into the output directory. Follow the aya-build 0.1 API: `aya_build::build()?` respects `CARGO_CFG_TARGET_ARCH` and invokes `clang` with the correct BPF target flags.
- [ ] Create the `agent/crates/ebpf-collector/bpf/` directory. Add a placeholder `common.h` defining the shared event structs that all BPF programs will write into ring buffers. Start with `struct process_event { u32 pid; u32 ppid; char comm[16]; char cmdline[256]; u32 uid; }` and equivalents for file and network. These structs must be `#[repr(C)]` on the Rust side.
- [ ] In `agent/crates/ebpf-collector/src/lib.rs`, add the module skeleton: `pub mod loader; pub mod events; pub mod error;`. Add a `CollectorError` enum using `thiserror` covering `BpfLoadError`, `ProgramAttachError`, `RingBufError`.
- [ ] Add `thiserror` to `ebpf-collector/Cargo.toml` dependencies (use workspace version).
- [ ] Verify that `cargo check -p ebpf-collector --target x86_64-unknown-linux-gnu` passes (on a Linux dev box or CI). Document the required host toolchain: `clang ≥ 14`, `llvm-strip`, `bpf-linker` if needed. Add a `README.md` in `agent/crates/ebpf-collector/` listing these requirements.
- [ ] Add a `#[cfg(target_os = "linux")]` guard to the crate's public API surface so the workspace compiles on macOS during development without failing (aya does not support non-Linux targets at runtime).
- [ ] Write a unit test `test_error_variants_display` that verifies `CollectorError::BpfLoadError("test".into())` formats without panic.

### Definition of done
- `cargo check -p ebpf-collector --target x86_64-unknown-linux-gnu` succeeds on a Linux host with clang ≥14 installed.
- `build.rs` is present and `aya_build::build()` is called; the `bpf/` directory exists and is referenced.
- `ebpf-collector` is a member of the workspace (appears in `cargo metadata --format-version 1 | jq '.workspace_members'`).
- `CollectorError` variants compile and display correctly.
- A macOS `cargo check` (without `--target bpf`) does not error on the crate.

### Notes / constraints
- `aya` 0.13 (declared in the existing `Cargo.toml`) targets kernels ≥5.8 for ring buffer support. This is the minimum acceptable kernel version for this workstream. Document this in the `README.md`.
- BTF (`CONFIG_DEBUG_INFO_BTF=y`) is required for CO-RE. Verify with `bpftool btf list` on the target kernel. Without BTF, the loader will need to embed vmlinux BTF via `aya_tool::generate`.
- The `aya-build` crate requires `clang` on `PATH` at build time. This must be in the CI runner and documented.
- Do not attempt to use `cargo-bpf` — the crate has already committed to `aya`.

---

## Issue: Define BPF ring buffer maps and shared event structs for process, file, and network telemetry
**Labels:** `ebpf`, `kernel`, `unsafe`
**Depends on:** Wire the ebpf-collector crate into the workspace build and establish the aya build pipeline
**Blocks:** process-lifecycle probe issue; network probe issue; userspace loader issue; event consumer issue

### What this is
Before any BPF program can be written, the shared data structures that live in BPF maps must be defined and agreed upon between kernel space (C) and userspace (Rust). This issue defines the three `RingBuf` maps (one per event category), the C structs written by BPF programs, and the corresponding `#[repr(C)]` Rust structs that the userspace event consumer will deserialize from ring buffer memory. It also wires these structs into the `edr-sdk` types pipeline (specifically `agent.proto` already has `ProcessEvent`, `FileEvent`, `NetworkEvent` — these Rust structs must be compatible).

### What is currently blocking this
The build pipeline issue above must land first. Once `bpf/common.h` exists as a placeholder, this issue replaces the placeholder with production-ready definitions.

### What this is blocking
The process probe and network probe issues, both of which write into these maps. The userspace event consumer, which reads from them.

### Implementation tasks
- [ ] In `agent/crates/ebpf-collector/bpf/common.h`, define `struct process_event { u32 pid; u32 ppid; char comm[TASK_COMM_LEN]; char cmdline[512]; u32 uid; u32 euid; char cwd[256]; }`. Use `TASK_COMM_LEN = 16` from `<linux/sched.h>`.
- [ ] In the same header, define `struct file_event { u32 pid; char comm[16]; char path[256]; u8 operation; s32 ret; }` where `operation` is an enum-equivalent `u8`: `0=open, 1=write, 2=delete, 3=rename`.
- [ ] Define `struct network_event { u32 pid; char comm[16]; u32 src_ip; u32 dst_ip; u16 src_port; u16 dst_port; u8 protocol; u8 direction; }` for IPv4. Add a `u8 is_ipv6` flag and `u8 src_ip6[16]` / `u8 dst_ip6[16]` fields for future IPv6 support (write zeroes if unused).
- [ ] Create `agent/crates/ebpf-collector/src/events.rs`. Define `#[repr(C)] pub struct ProcessEvent { pub pid: u32, pub ppid: u32, pub comm: [u8; 16], pub cmdline: [u8; 512], pub uid: u32, pub euid: u32, pub cwd: [u8; 256] }` — field layout must exactly match the C struct. Do the same for `FileEvent` and `NetworkEvent`. Derive nothing that requires heap allocation (no `String` here — these are read directly from kernel ring buffer memory).
- [ ] Implement `TryFrom<&[u8]>` for each struct that reads from a `&[u8]` slice (from the ring buffer). Use `zerocopy` or manual `ptr::read_unaligned` under `unsafe`. Add a bounds check: if the slice is shorter than `mem::size_of::<T>()`, return `CollectorError::MalformedEvent`.
- [ ] Add a `fn to_sdk_process_event(&self) -> edr_sdk::proto::agent::ProcessEvent` converter on `ProcessEvent` that maps the `[u8; N]` comm/cmdline/cwd arrays to `String` using `from_utf8_lossy`. Do the same for `FileEvent` → `edr_sdk::proto::agent::FileEvent` and `NetworkEvent` → `edr_sdk::proto::agent::NetworkEvent`. (This requires `edr-sdk` to expose generated proto types — verify `sdk/src/lib.rs` re-exports them.)
- [ ] Define the three ring buffer map names as constants: `pub const PROCESS_EVENTS: &str = "PROCESS_EVENTS"`, `FILE_EVENTS`, `NETWORK_EVENTS`. These strings must match the map section names in the BPF C programs.
- [ ] Write unit tests: `test_process_event_from_bytes_exact_size`, `test_process_event_from_bytes_too_short`, `test_file_event_operation_roundtrip`, `test_network_event_ipv4_conversion_to_sdk_type`. Run with `cargo test -p ebpf-collector`.

### Definition of done
- `cargo test -p ebpf-collector` passes all unit tests above.
- The three `#[repr(C)]` structs are defined, size-checked in tests (`assert_eq!(mem::size_of::<ProcessEvent>(), <expected>)`).
- `TryFrom<&[u8]>` is implemented and tested for malformed input.
- Conversion to `edr-sdk` proto types is implemented and compiles.

### Notes / constraints
- Do not use `serde` or `bincode` for deserializing ring buffer events — the kernel writes raw C structs. Only `zerocopy` or manual pointer casts (with alignment guarantees checked) are appropriate.
- Padding bytes in C structs will appear in the ring buffer. Ensure the Rust struct fields are laid out in the same order with the same alignment as the C struct. Use `static_assertions::assert_eq_size!` if desired.
- `TASK_COMM_LEN` is 16 bytes on all supported Linux kernels. `cmdline` is bounded to 512 to avoid stack overflow in BPF programs (BPF stack limit is 512 bytes total per program).

---

## Issue: Implement the process-lifecycle BPF program (execve tracepoint) and wire it to the ring buffer
**Labels:** `ebpf`, `kernel`, `unsafe`, `tracing`
**Depends on:** BPF map definitions issue
**Blocks:** Userspace loader issue; event consumer issue; eBPF integration test issue

### What this is
This issue writes the first production BPF program: `process_probe.bpf.c`, attached to the `sys_enter_execve` tracepoint. It populates a `RingBuf` map with `struct process_event` entries on every `execve` syscall. This is the canonical first probe for any EDR because process execution is the root of nearly every attack chain. Getting this right — correct map access patterns, correct argument extraction, BPF verifier compliance — establishes the pattern for all subsequent probes.

### What is currently blocking this
The BPF map definitions issue (structs must be defined in `common.h` before programs can use them).

### What this is blocking
The userspace loader (which loads and attaches this program). The event consumer (which reads from the ring buffer this program writes into).

### Implementation tasks
- [ ] Create `agent/crates/ebpf-collector/bpf/process_probe.bpf.c`. Include `<linux/bpf.h>`, `<bpf/bpf_helpers.h>`, `<bpf/bpf_tracing.h>`, and `"common.h"`.
- [ ] Declare the ring buffer map: `struct { __uint(type, BPF_MAP_TYPE_RINGBUF); __uint(max_entries, 256 * 1024); } PROCESS_EVENTS SEC(".maps");`. 256 KiB = 64 pages, safe default for high-frequency execve.
- [ ] Implement `SEC("tracepoint/syscalls/sys_enter_execve") int trace_execve(struct trace_event_raw_sys_enter *ctx)`. Use `bpf_ringbuf_reserve` to allocate a `struct process_event` slot, fill `pid` from `bpf_get_current_pid_tgid() >> 32`, `ppid` from walking `task_struct` via `bpf_get_current_task()` + `BPF_CORE_READ`, `uid/euid` from `bpf_get_current_uid_gid()`, `comm` from `bpf_get_current_comm()`, and `cmdline` by reading `ctx->args[0]` (argv[0]) via `bpf_probe_read_user_str`. Submit with `bpf_ringbuf_submit`.
- [ ] Handle the BPF verifier constraint: `bpf_probe_read_user_str` on `cmdline` must use a bounded length (≤ 512). Add a null terminator at `cmdline[511]` defensively.
- [ ] For `ppid`: use `BPF_CORE_READ(task, real_parent, tgid)` — this requires BTF CO-RE. Add a preprocessor guard `#ifdef __TARGET_ARCH_x86` for architecture portability if needed.
- [ ] Add `char _license[] SEC("license") = "GPL";` — required for helper access.
- [ ] Verify the program compiles with `clang -O2 -g -target bpf -D__TARGET_ARCH_x86_64 -c process_probe.bpf.c -o /dev/null` as a manual check. The `build.rs` will handle this at cargo build time.
- [ ] Write a unit test in `src/events.rs` (already tracking this crate) that constructs a fake `ProcessEvent` byte buffer mimicking what the kernel would write, then parses it and asserts field values.

### Definition of done
- `process_probe.bpf.c` compiles without verifier errors when loaded on a Linux 5.15+ kernel with BTF enabled.
- `build.rs` picks up the new file and produces a compiled BPF object in `target/bpf/`.
- The `struct process_event` layout in C matches the `ProcessEvent` Rust struct (verified via size assertions in unit tests).
- `cargo build -p ebpf-collector` succeeds on a Linux host with clang ≥14.

### Notes / constraints
- `bpf_probe_read_user_str` is the correct helper for reading userspace memory (argv). Do not use `bpf_probe_read_kernel_str` for userspace pointers — it will silently read zeroes on modern kernels.
- Tracepoint `sys_enter_execve` provides the raw syscall arguments. The `ctx->args[0]` is a pointer to the filename string, `ctx->args[1]` is argv (pointer-to-pointer). Reading individual argv elements requires multiple `bpf_probe_read_user` calls inside a bounded loop (BPF verifier requires loops to have provable termination). For this issue, reading only argv[0] (filename) into `cmdline` is acceptable. Full cmdline reconstruction is a follow-on.
- CO-RE with `BPF_CORE_READ` requires the kernel to expose BTF. If `CONFIG_DEBUG_INFO_BTF` is not set, the loader must supply vmlinux BTF. This constraint is documented in the build pipeline issue.

---

## Issue: Implement the network-event BPF program (connect/bind tracepoints) and wire it to the ring buffer
**Labels:** `ebpf`, `kernel`, `networking`, `unsafe`
**Depends on:** BPF map definitions issue
**Blocks:** Userspace loader issue; event consumer issue; eBPF integration test issue

### What this is
This issue writes `network_probe.bpf.c`, attached to `tracepoint/syscalls/sys_enter_connect` and `tracepoint/syscalls/sys_enter_bind`. On each call it extracts the 5-tuple (src IP, dst IP, src port, dst port, protocol) and the process context (PID, comm), then writes a `struct network_event` into the `NETWORK_EVENTS` ring buffer. This probe feeds the connection isolation workstream: the isolation table (Workstream B) needs to know which connections the EDR process itself makes in order to register them as allowed.

### What is currently blocking this
The BPF map definitions issue (the `struct network_event` definition must be in `common.h`).

### What this is blocking
The userspace loader and event consumer. The connection isolation table (Workstream B) — specifically, the "population" issue in that workstream depends on network events being available to identify the EDR's own connections.

### Implementation tasks
- [ ] Create `agent/crates/ebpf-collector/bpf/network_probe.bpf.c`. Declare `NETWORK_EVENTS` ring buffer map (same pattern as `PROCESS_EVENTS`, 256 KiB initial).
- [ ] Implement `SEC("tracepoint/syscalls/sys_enter_connect") int trace_connect(struct trace_event_raw_sys_enter *ctx)`. Extract the `sockaddr` pointer from `ctx->args[1]`. Use `bpf_probe_read_user` to read the `struct sockaddr` header. Branch on `sa_family`: if `AF_INET`, read `struct sockaddr_in` and extract `sin_addr.s_addr` and `sin_port`; if `AF_INET6`, set the `is_ipv6` flag and read `struct sockaddr_in6`. Write into ring buffer with `direction = 1` (outbound). Ignore `AF_UNIX` and other families (submit nothing).
- [ ] Implement `SEC("tracepoint/syscalls/sys_enter_bind") int trace_bind(...)` with the same extraction logic, setting `direction = 0` (inbound).
- [ ] Fill `pid` and `comm` in both handlers using the same helpers as the process probe (`bpf_get_current_pid_tgid`, `bpf_get_current_comm`).
- [ ] Set `protocol = IPPROTO_TCP` by default. Distinguishing TCP vs UDP at the `connect`/`bind` tracepoint requires reading the socket struct via `bpf_get_current_task` and `BPF_CORE_READ(task, files, ...)` — this is complex and error-prone. For this issue, mark protocol as `0xFF` (unknown) and resolve via a kretprobe on `sock_recvmsg`/`sock_sendmsg` in a follow-on issue if needed.
- [ ] Port numbers from `sockaddr` are in network byte order. Convert to host byte order using `bpf_ntohs()` before writing into the event struct.
- [ ] Add `char _license[] SEC("license") = "GPL";`.
- [ ] Write unit tests for `NetworkEvent::try_from(&[u8])` covering an IPv4 outbound event and an IPv6 inbound event.

### Definition of done
- `network_probe.bpf.c` compiles without verifier errors on a Linux 5.15+ kernel with BTF.
- Both `trace_connect` and `trace_bind` are present and functional.
- IPv4 and IPv6 addresses are extracted correctly (verified in unit tests on the Rust struct parsing side).
- `cargo build -p ebpf-collector` succeeds with both `process_probe.bpf.c` and `network_probe.bpf.c` in the `bpf/` directory.

### Notes / constraints
- `bpf_probe_read_user` on the `sockaddr *` pointer can fail if the pointer is invalid (e.g., the userspace process passed a bogus address). Always check the return value. On error, discard the event rather than submitting garbage.
- The connect tracepoint fires before the kernel validates the address. The connection may fail; we still want to record the attempt.
- IPv6 detection via `sa_family == AF_INET6` is the correct check. The `struct network_event` has fields for both. Only one set should be populated per event.
- Port 0 (ephemeral assignment) may appear in `bind` calls. This is a valid event — do not filter it.

---

## Issue: Implement the userspace BPF loader: load compiled objects and attach programs to kernel hooks
**Labels:** `ebpf`, `kernel`, `unsafe`, `async`
**Depends on:** Process-lifecycle probe issue; network-event probe issue
**Blocks:** Event consumer issue; eBPF error handling issue; eBPF integration test issue

### What this is
This issue implements `agent/crates/ebpf-collector/src/loader.rs`: the userspace Rust code that loads the compiled BPF object files using `aya`, attaches each program to its kernel hook point, and returns handles to the loaded maps so the event consumer can read from them. This is the bridge between the compiled `.bpf.o` artifacts and the running kernel.

### What is currently blocking this
Both BPF programs (process probe and network probe) must be compiled and their ring buffer map definitions must be finalized before the loader can reference them by name.

### What this is blocking
The event consumer (which receives map handles from the loader). The error handling issue (which wraps loader failures). The integration test (which calls the loader in a privileged context).

### Implementation tasks
- [ ] Create `agent/crates/ebpf-collector/src/loader.rs`. Define `pub struct EbpfLoader` that owns an `aya::Ebpf` instance (the loaded BPF object) and exposes the ring buffer map handles.
- [ ] Implement `EbpfLoader::load() -> Result<Self, CollectorError>`. Use `aya::Ebpf::load(include_bytes_aligned!(concat!(env!("OUT_DIR"), "/bpf/process_probe.bpf.o")))` to embed the compiled object at link time. Do the same for `network_probe.bpf.o`. These two programs can be in separate `Ebpf` instances or combined if the build pipeline merges them.
- [ ] Attach `process_probe` to its tracepoint: `let prog: &mut TracePoint = bpf.program_mut("trace_execve").unwrap().try_into()?; prog.load()?; prog.attach("syscalls", "sys_enter_execve")?;` — handle `ProgramError` by mapping to `CollectorError::ProgramAttachError`.
- [ ] Attach `trace_connect` and `trace_bind` from `network_probe.bpf.o` analogously.
- [ ] Implement `EbpfLoader::process_ring_buf(&mut self) -> &mut RingBuf<&mut MapData>` and `network_ring_buf` accessors that retrieve the `PROCESS_EVENTS` and `NETWORK_EVENTS` maps from the loaded `Ebpf` instance using `aya::maps::RingBuf::try_from(bpf.map_mut("PROCESS_EVENTS")?)`.
- [ ] Implement `EbpfLoader::detach(self) -> Result<(), CollectorError>` that drops all program handles, causing kernel detachment. Log each detach operation.
- [ ] Add `pub fn is_btf_available() -> bool` that reads `/sys/kernel/btf/vmlinux` existence as a pre-flight check. Return `false` if the file doesn't exist. The caller (error handling issue) uses this to provide a useful error message.
- [ ] Write a compile-time test `test_loader_struct_is_send` that asserts `EbpfLoader: !Send` (it holds a raw `MapData` reference that is not `Send`). This documents the threading constraint. The loader must live on a single task; use `tokio::task::LocalSet` in the consumer.

### Definition of done
- `EbpfLoader::load()` compiles and, when run as root on a Linux 5.15+ kernel, loads both BPF programs and attaches them to their tracepoints without error.
- `process_ring_buf()` and `network_ring_buf()` return valid map handles.
- `EbpfLoader::detach()` cleans up without leaving dangling programs (verify with `bpftool prog list` before and after).
- `is_btf_available()` returns `true` on a BTF-enabled kernel and `false` on a kernel without `/sys/kernel/btf/vmlinux`.

### Notes / constraints
- `aya::Ebpf::load` requires the process to have `CAP_BPF` (kernel ≥5.8) or `CAP_SYS_ADMIN` (older). The agent binary must be deployed with the appropriate capability set.
- `include_bytes_aligned!` is provided by `aya` and must be used instead of `include_bytes!` to satisfy BPF object alignment requirements.
- If the two BPF programs are in separate object files (compiled separately by `build.rs`), they require two separate `aya::Ebpf` instances. The `EbpfLoader` struct must own both. Do not merge them into one object to avoid BPF map naming conflicts.
- `RingBuf` map access is not `Send`. The entire loader and consumer must run on a single OS thread using `tokio::task::LocalSet` or `std::thread::spawn`.

---

## Issue: Implement the ring buffer event consumer: read, deserialize, and forward events into the agent pipeline
**Labels:** `ebpf`, `kernel`, `async`, `tracing`
**Depends on:** Userspace loader issue
**Blocks:** eBPF pipeline integration issue; eBPF integration test issue

### What this is
This issue implements `agent/crates/ebpf-collector/src/events.rs` consumer logic: a polling loop that reads raw bytes from the `PROCESS_EVENTS` and `NETWORK_EVENTS` ring buffers (via `aya::maps::ring_buf::RingBuf`), deserializes them into the typed Rust structs defined in the map definitions issue, converts them to `edr-sdk` proto types (`AgentEvent` with appropriate `event_type`), and forwards them to the `agent-core` orchestrator via an `mpsc::Sender<AgentEvent>`. This is the final hop from kernel telemetry to the agent's event pipeline.

### What is currently blocking this
The userspace loader must land first (this issue depends on the ring buffer map handles it returns).

### What this is blocking
The pipeline integration issue (wiring this into `agent-core/orchestrator.rs`). The integration test (which validates end-to-end event emission).

### Implementation tasks
- [ ] In `agent/crates/ebpf-collector/src/events.rs`, implement `pub struct EventConsumer` that holds an `EbpfLoader` and a `tokio::sync::mpsc::Sender<fleet_client::types::AgentEvent>`.
- [ ] Implement `EventConsumer::run(self) -> Result<(), CollectorError>` as a blocking loop (must run in `tokio::task::spawn_blocking` or a `LocalSet`-driven loop because `RingBuf::next()` is synchronous). The loop calls `process_ring_buf.next()` and `network_ring_buf.next()` in an interleaved fashion using a short poll interval (`tokio::time::sleep(Duration::from_millis(1))`). For each returned `Item`, call `ProcessEvent::try_from(item.as_ref())` or `NetworkEvent::try_from(item.as_ref())`, convert to proto bytes via `encode_to_vec()`, then send as an `AgentEvent` with `event_type = 1` (process) or `event_type = 3` (network), `timestamp_ns = SystemTime::now()`, and a `sequence_id = Uuid::new_v4().to_string()`.
- [ ] Handle deserialization errors by logging `tracing::warn!` with the raw byte length and continuing the loop — a malformed event must not crash the consumer.
- [ ] Handle `mpsc::Sender::send` returning `Err` (receiver dropped) by returning `CollectorError::PipelineClosed` — this signals the orchestrator has shut down.
- [ ] Add a shutdown signal: `EventConsumer::run_until_shutdown(self, shutdown: tokio::sync::CancellationToken)` that breaks the poll loop when the token is cancelled.
- [ ] Expose `pub fn start(loader: EbpfLoader, node_id: String) -> (EventConsumer, mpsc::Receiver<AgentEvent>)` as the public API. The `node_id` is set on every `AgentEvent.node_id` field.
- [ ] Write unit tests: `test_consumer_forwards_process_event` (mock the ring buffer with a pre-built byte slice, verify the sender receives a correctly-typed `AgentEvent`), `test_consumer_handles_malformed_bytes_without_crash`.

### Definition of done
- `EventConsumer::run_until_shutdown` runs without error on a real kernel and produces `AgentEvent` structs on the returned channel when processes exec or make network connections.
- Malformed ring buffer data does not panic or crash the consumer loop.
- Shutdown via `CancellationToken` terminates the loop cleanly.
- Unit tests pass with `cargo test -p ebpf-collector`.

### Notes / constraints
- `RingBuf::next()` does not block — it returns `None` immediately if no events are pending. The polling approach with a 1ms sleep trades latency for CPU. A production follow-on should use `epoll`/`tokio::io::unix::AsyncFd` on the ring buffer's file descriptor. This issue does not need to solve that.
- The `node_id` stamped on events comes from the enrollment flow in `fleet-client`. The consumer must receive it at construction time (not read from config) because enrollment is async and may not have completed when the consumer starts.
- Events produced by the BPF programs include the EDR agent's own process events (the agent will observe itself execing). This is not filtered here — filtering is downstream in the isolation table and rule engine.

---

## Issue: Wire the ebpf-collector event consumer into agent-core/orchestrator.rs alongside the existing osquery pipeline
**Labels:** `ebpf`, `async`, `tracing`
**Depends on:** Event consumer issue
**Blocks:** eBPF integration test issue

### What this is
The `agent-core/orchestrator.rs` currently runs a complete osquery pipeline: `OsqueryCollector::start()` → `mpsc::Receiver<OsqueryResult>` → `EventBuffer::push()`. The eBPF pipeline must be integrated in parallel: `EventConsumer::start()` → `mpsc::Receiver<AgentEvent>` → `EventBuffer::push()`. This issue adds the eBPF consumer as a concurrent task in the main orchestrator loop, guarded by a compile-time `#[cfg(target_os = "linux")]` block so the agent still compiles on macOS.

### What is currently blocking this
The event consumer issue must be complete (the `EventConsumer::start()` API must exist and return an `mpsc::Receiver<AgentEvent>`).

### What this is blocking
The eBPF integration test (which validates the full path from kernel probe to buffer).

### Implementation tasks
- [ ] In `agent/crates/agent-core/src/orchestrator.rs`, add a `#[cfg(target_os = "linux")]` block after the `OsqueryCollector` startup that calls `ebpf_collector::EventConsumer::start(EbpfLoader::load()?, node_id.clone())`. This yields a `(EventConsumer, Receiver<AgentEvent>)` tuple.
- [ ] Add `ebpf-collector = { path = "../../crates/ebpf-collector" }` to `agent/crates/agent-core/Cargo.toml` under a `[target.'cfg(target_os = "linux")'.dependencies]` section.
- [ ] Spawn the `EventConsumer::run_until_shutdown(consumer, cancellation_token.clone())` call inside a `tokio::task::spawn_blocking` or a dedicated `LocalSet`-driven thread (see loader notes on non-`Send` constraint).
- [ ] Add the eBPF `AgentEvent` receiver to the main `tokio::select!` loop alongside the existing `results_rx.recv()` arm: when an `AgentEvent` arrives from the eBPF channel, encode it to bytes via `AgentEvent::encode_to_vec()` and call `buffer.push(&bytes)`.
- [ ] Handle the case where `EbpfLoader::load()` fails (e.g., insufficient capabilities, kernel too old, BTF not available): log `tracing::warn!("eBPF collector failed to load: {}. Continuing in OSQuery-only mode.", e)` and skip the eBPF pipeline. Do not abort agent startup.
- [ ] Thread the `CancellationToken` through the shutdown path so the eBPF consumer is stopped before the process exits.
- [ ] Update the existing `test_orchestrator_startup` integration test (in `tests/TEST_PLAN.md` it is listed as `agent-core` integration test) to assert that a Linux agent starts with the eBPF consumer active, and a degraded-mode test that asserts startup succeeds even when the eBPF loader returns an error.

### Definition of done
- `cargo build -p agent-bin --target x86_64-unknown-linux-gnu` succeeds with eBPF integration enabled.
- On a Linux host with sufficient capabilities: the agent logs `"eBPF collector started"` on startup.
- On a host without sufficient capabilities (or on macOS during development): the agent logs the degraded-mode warning and continues with osquery only.
- The shutdown path stops the eBPF consumer cleanly (no log errors on exit).

### Notes / constraints
- The `EventBuffer` (SQLite via `rusqlite`) is `!Send`. The eBPF consumer receiver's `AgentEvent`s must be forwarded to the buffer on the same thread that owns it (the main orchestrator task). The `select!` loop in orchestrator already handles this correctly for osquery — extend it the same way.
- `EbpfLoader` is also `!Send` and must live on the same thread as the consumer. If using `spawn_blocking`, pass ownership of the loader into the closure before spawning.

---

## Issue: eBPF error handling and graceful degradation when probe attachment fails
**Labels:** `ebpf`, `error-handling`, `kernel`
**Depends on:** Wire eBPF into orchestrator issue
**Blocks:** eBPF integration test issue

### What this is
This issue hardens the eBPF subsystem against runtime failures: partial probe attachment failures (process probe loads but network probe fails), ring buffer exhaustion, and consumer task panic. The current `CollectorError` enum from the build pipeline issue has the variants but no structured handling. This issue adds the handling logic and the graceful fallback strategy.

### What is currently blocking this
The orchestrator integration must be in place so there is a running system to harden.

### What this is blocking
The eBPF integration test (which validates the degraded-mode behavior).

### Implementation tasks
- [ ] In `loader.rs`, change `EbpfLoader::load()` to return a `LoadResult { loader: EbpfLoader, warnings: Vec<String> }` struct instead of bare `Result<EbpfLoader>`. If a probe fails to attach (e.g., the tracepoint doesn't exist on this kernel), push the error into `warnings` and continue loading remaining probes. A partial load is better than no telemetry.
- [ ] Implement `fn attach_with_warn(bpf: &mut Ebpf, program_name: &str, category: &str, tracepoint: &str, warnings: &mut Vec<String>) -> bool` that attempts attachment and on failure pushes a human-readable message into warnings and returns `false`.
- [ ] In `orchestrator.rs`, log each `LoadResult.warning` at `tracing::warn!` level with a prefix of `"[ebpf][degraded]"`.
- [ ] Handle ring buffer exhaustion: if `RingBuf::next()` returns data but deserialization consistently fails for more than 100 consecutive events, emit a `tracing::error!` and pause polling for 5 seconds before resuming (back-pressure heuristic).
- [ ] Wrap the entire `EventConsumer::run_until_shutdown` call in `spawn_blocking` with a `catch_unwind` equivalent (use `std::panic::catch_unwind` inside the blocking closure). If the consumer panics, log the panic message and do not crash the agent — restart the consumer loop after a 10-second delay.
- [ ] Add a metric counter (or at minimum a `tracing::info!` log) for `events_received_from_ebpf` and `events_dropped_from_ebpf` that increments in the consumer loop. This feeds future observability.
- [ ] Write a unit test `test_partial_load_continues_on_attachment_failure` that mocks the aya attach call returning an error and asserts `warnings` is non-empty while `loader.process_ring_buf()` still works.

### Definition of done
- When the process probe loads but the network probe fails: the agent starts, logs a degraded warning, and continues streaming process events.
- When the entire BPF load fails: the agent starts in osquery-only mode with no errors beyond the degraded warning.
- A consumer panic does not crash the agent process.
- `cargo test -p ebpf-collector` passes the partial-load test.

### Notes / constraints
- Do not retry failed probe attachment on a loop — the failure is usually structural (capability missing, kernel too old). Log once and accept degraded mode.
- Ring buffer exhaustion (events dropped by the kernel) appears as gaps in sequence IDs. The kernel tracks drops internally; aya exposes `RingBuf::dropped_events()` if available. Check aya 0.13 API for this method.

---

## Issue: eBPF integration test: load probes in a controlled environment and verify end-to-end event emission
**Labels:** `ebpf`, `testing`, `kernel`
**Depends on:** eBPF error handling issue; Wire eBPF into orchestrator issue
**Blocks:** nothing (leaf node in workstream)

### What this is
This issue implements the integration tests for the entire eBPF pipeline, covering: successful probe load and attachment, event emission verification (exec a known subprocess → observe `ProcessEvent` on the ring buffer), network event capture (make a loopback TCP connection → observe `NetworkEvent`), and degraded-mode startup. These tests require a real Linux kernel and run in CI on `ubuntu-latest` (GitHub Actions runner) as a job that includes `sudo` access for `CAP_BPF`.

### What is currently blocking this
All prior eBPF issues must be complete. The test exercises the full stack.

### What this is blocking
Nothing — this is the leaf node of the eBPF workstream.

### Implementation tasks
- [ ] Create `agent/crates/ebpf-collector/tests/integration_test.rs`. Gate the entire file with `#[cfg(target_os = "linux")]` and a custom feature flag `ebpf_integration_tests` (controlled by `CARGO_FEATURE_EBPF_INTEGRATION_TESTS` env var) to prevent these from running in normal `cargo test` invocations.
- [ ] Implement `test_loader_loads_and_attaches`: calls `EbpfLoader::load()`, asserts `Ok`, calls `is_btf_available()` and skips if false, asserts ring buffer handles are accessible.
- [ ] Implement `test_process_probe_captures_execve`: after loading, `std::process::Command::new("/bin/true").spawn().wait()`, then poll the ring buffer for up to 500ms expecting at least one `ProcessEvent` with `comm == b"true\0...\0"` (null-padded). Assert `pid != 0` and `ppid == current PID`.
- [ ] Implement `test_network_probe_captures_connect`: spawn a `tokio::net::TcpListener` on `127.0.0.1:0`, get the bound port, then `TcpStream::connect` to it. Poll the network ring buffer for up to 500ms expecting a `NetworkEvent` with `dst_port == bound_port` and `direction == 1` (outbound).
- [ ] Implement `test_probe_detach_removes_program`: load probes, call `detach()`, then verify with `bpftool prog list` via `std::process::Command` that the program name is no longer listed.
- [ ] Implement `test_degraded_mode_no_cap_bpf`: run as unprivileged user (or drop caps in the test), call `EbpfLoader::load()`, assert `Err(CollectorError::BpfLoadError(_))`, and assert the agent-core degraded-mode path does not panic.
- [ ] Add a `[[test]]` entry in `ebpf-collector/Cargo.toml` for the integration test file with `required-features = ["ebpf_integration_tests"]`.
- [ ] Add a `.github/workflows/ebpf-integration.yml` workflow (or note in the existing CI template) that runs `cargo test -p ebpf-collector --features ebpf_integration_tests` on `ubuntu-latest` with `sudo` or using the `BPF_CAP` action.

### Definition of done
- All four integration tests pass on a GitHub Actions `ubuntu-latest` runner (kernel 6.x, BTF enabled).
- `test_degraded_mode_no_cap_bpf` passes without root — it should return a `BpfLoadError`, not panic.
- Tests are gated by feature flag and do not run in a normal `cargo test --workspace` invocation.
- CI job is defined and runs on PRs targeting `main`.

### Notes / constraints
- GitHub Actions `ubuntu-latest` (as of 2025) runs kernel 6.5+, which has BTF enabled. The tests can rely on BTF being present.
- `CAP_BPF` requires either `sudo` in CI or setting up a test runner with the capability. Using `sudo -E cargo test` in the CI step is the simplest approach.
- The `test_probe_detach_removes_program` test requires `bpftool` to be installed on the runner. Add `sudo apt-get install -y bpftool` as a CI step.
- Flakiness risk: ring buffer polling has a 500ms window. If the CI runner is extremely loaded, events may arrive after the timeout. Use `tokio::time::timeout` and mark the test as `#[ignore]` with a note to run on dedicated hardware if it becomes flaky in practice.

---

## Issue: Define the connection isolation table data structure and its key/value types
**Labels:** `networking`, `tables`, `firewall`
**Depends on:** none
**Blocks:** Isolation table population issue; enforcement issue; concurrency issue

### What this is
The `agent/crates/isolation/src/lib.rs` is a one-line comment stub. The entire isolation crate needs to be built from scratch. This issue defines the foundational data structure: what the allow-list table is, what its key is, and what it contains. Based on the implementation guide, the isolation model is iptables-based: the `IsolateCommand` from the fleet server triggers iptables rules that drop all traffic except to the fleet server IP. The "table" in this workstream is the in-memory Rust data structure that tracks which connections are registered as allowed (belonging to the EDR process) so that the iptables rule-generation logic knows which addresses to preserve when isolation is applied.

### What is currently blocking this
Nothing — this is the root issue of Workstream B.

### What this is blocking
Everything else in Workstream B. The population and enforcement issues depend on the table types defined here.

### Implementation tasks
- [ ] In `agent/crates/isolation/src/lib.rs`, define the module skeleton: `pub mod table; pub mod iptables; pub mod error;`.
- [ ] Create `agent/crates/isolation/src/error.rs`. Define `IsolationError` using `thiserror`: variants `IptablesExecFailed { exit_code: i32, stderr: String }`, `IptablesNotFound`, `InvalidAddress(String)`, `TableLockTimeout`. Add to `isolation/Cargo.toml` dependencies: `thiserror = { workspace = true }`.
- [ ] Create `agent/crates/isolation/src/table.rs`. Define `pub struct ConnectionKey { pub remote_ip: std::net::IpAddr, pub remote_port: u16, pub protocol: Protocol }` where `Protocol` is `pub enum Protocol { Tcp, Udp }`. The key is the remote endpoint — the "allowed" destination from the EDR's perspective.
- [ ] Define `pub struct ConnectionEntry { pub key: ConnectionKey, pub registered_at: std::time::Instant, pub description: &'static str }` — the value in the table. `description` is a static label like `"fleet-server"` or `"osquery-socket"`.
- [ ] Define `pub struct IsolationTable` as a struct wrapping `Vec<ConnectionEntry>` (not a HashMap — the table is expected to have ≤10 entries: the fleet server address, optionally a few internal addresses). Linear scan is acceptable and avoids the complexity of a hash map with a non-trivial key type.
- [ ] Implement `IsolationTable::new() -> Self`, `register(&mut self, key: ConnectionKey, description: &'static str)`, `deregister(&mut self, key: &ConnectionKey)`, `allowed_remotes(&self) -> Vec<&ConnectionKey>`.
- [ ] Add `serde` derives to `ConnectionKey` and `Protocol` for persistence (see the persistence issue below). Use `#[serde(rename_all = "snake_case")]` on `Protocol`.
- [ ] Write unit tests: `test_register_and_lookup`, `test_deregister_removes_entry`, `test_allowed_remotes_returns_all`, `test_empty_table_allowed_remotes_is_empty`. Place in a `#[cfg(test)]` module inside `table.rs`.

### Definition of done
- `cargo test -p isolation` passes all four unit tests.
- `IsolationTable`, `ConnectionKey`, `Protocol`, `ConnectionEntry`, and `IsolationError` are all defined and exported from `isolation::table` and `isolation::error`.
- No external crates beyond `thiserror` and `serde` are added (no async, no lock primitives — this issue is synchronous data structures only).

### Notes / constraints
- The isolation table is fundamentally different from eBPF maps. It is a userspace Rust data structure, not a kernel construct. The name "table" in the workstream name refers to this in-memory allow-list, not to a BPF map.
- `std::net::IpAddr` handles both IPv4 and IPv6. Use it instead of a custom type.
- `description` is `&'static str` not `String` to keep the entry allocation-free. All call sites use string literals.

---

## Issue: Implement isolation table population: register EDR-owned connections as allowed
**Labels:** `networking`, `tables`, `firewall`, `ipc`
**Depends on:** Isolation table data structure issue
**Blocks:** Enforcement issue; concurrency issue

### What this is
The isolation table needs to be pre-populated with the connections the EDR process itself makes — primarily the gRPC connection to the fleet server. When isolation is applied, these connections must remain reachable (isolation means "block everything except EDR comms"). This issue implements the population path: the `IsolationTable::register()` calls that happen at agent startup and whenever a new EDR connection is established.

### What is currently blocking this
The table data structure issue must be complete.

### What this is blocking
Enforcement: the iptables rule generator reads from the populated table. Concurrency: the locking wrapper is designed around the usage patterns established here.

### Implementation tasks
- [ ] In `agent/crates/isolation/src/table.rs`, implement `IsolationTable::register_fleet_server(endpoint: &str) -> Result<(), IsolationError>` that parses the gRPC endpoint URL (e.g., `"http://fleet.internal:50051"`) into a `ConnectionKey` (resolve hostname to IP using `std::net::ToSocketAddrs`, extract port, set `protocol = Tcp`). Handle DNS resolution failure with `IsolationError::InvalidAddress`.
- [ ] Add `IsolationTable::register_osquery_socket(_path: &Path)` — a no-op stub that documents the intent (osquery uses a Unix domain socket, which is not filtered by iptables IP rules; this entry is for documentation and future netfilter-socket-level filtering).
- [ ] In `agent/crates/agent-core/src/orchestrator.rs`, after the `FleetClient::enroll()` succeeds and the fleet server IP is known, call `table.register_fleet_server(&config.fleet.endpoint)`. The `IsolationTable` instance should be created at the start of `orchestrator::run()` and passed to any component that needs it.
- [ ] Wire the `IsolationTable` into the `FleetClient`'s reconnect path: on each successful reconnect, re-register the fleet server (its IP may have changed via DNS). Implement `update_fleet_server(table: &mut IsolationTable, endpoint: &str)` that calls `deregister` on the old key then `register` on the new one.
- [ ] Add `agent/crates/isolation` to the `agent-core/Cargo.toml` dependencies (it is not listed yet).
- [ ] Write unit tests: `test_register_fleet_server_parses_http_endpoint`, `test_register_fleet_server_parses_https_endpoint`, `test_register_fleet_server_invalid_url_returns_error`, `test_update_fleet_server_replaces_old_entry`.

### Definition of done
- `IsolationTable::register_fleet_server("http://fleet.internal:50051")` resolves and registers the entry correctly (tested with a real DNS lookup in unit tests using `127.0.0.1` as a known-resolving address).
- The orchestrator creates an `IsolationTable` and registers the fleet server on successful enrollment.
- `cargo test -p isolation` and `cargo test -p agent-core` pass.
- The `isolation` crate is a dependency of `agent-core`.

### Notes / constraints
- Hostname resolution in `register_fleet_server` is synchronous (`ToSocketAddrs` is blocking). Since orchestrator startup is async (tokio), call this in `tokio::task::spawn_blocking` or use `tokio::net::lookup_host` instead. Prefer `tokio::net::lookup_host` to stay async.
- The table at this point has no locking. The population issue establishes the write path; the concurrency issue (next) adds locking. For now, `IsolationTable` is assumed to be owned and mutated by a single task.
- Do not attempt to register non-IP connections (unix sockets, abstract namespace). Document the scope limitation in a comment.

---

## Issue: Add concurrency wrapper around IsolationTable for hot-path read and control-path write
**Labels:** `networking`, `tables`, `firewall`, `async`
**Depends on:** Isolation table population issue
**Blocks:** Enforcement issue; integration test

### What this is
The `IsolationTable` will be read from a hot path (the enforcement point checks it on every iptables rule generation, which happens at isolation time) and written from a control path (enrollment, reconnect, and the `IsolateCommand` handler). This issue wraps `IsolationTable` in the appropriate synchronization primitive and defines the shared handle type used by all consumers.

### What is currently blocking this
The population issue must exist to know the write patterns before choosing a locking strategy.

### What this is blocking
The enforcement issue, which takes the shared handle and reads from it. The integration test.

### Implementation tasks
- [ ] Decide on the concurrency primitive: the table is written at most once per reconnect cycle (very low frequency) and read at isolation time (also infrequent — isolation is an operator action). A `std::sync::RwLock<IsolationTable>` wrapped in `Arc` is correct here. There is no hot-path lookup that would justify a lock-free approach. Document this rationale in a comment in `table.rs`.
- [ ] Define `pub type SharedIsolationTable = Arc<RwLock<IsolationTable>>` in `isolation/src/table.rs`. Export it from `isolation/src/lib.rs`.
- [ ] Implement `SharedIsolationTable::new_shared() -> Self` as a constructor shortcut: `Arc::new(RwLock::new(IsolationTable::new()))`.
- [ ] Add `impl IsolationTable { pub fn into_shared(self) -> SharedIsolationTable { Arc::new(RwLock::new(self)) } }`.
- [ ] In `agent/crates/isolation/src/table.rs`, implement convenience methods on `SharedIsolationTable` (via a newtype or inherent methods on a wrapper struct): `register_fleet_server_shared(&self, endpoint: &str) -> Result<(), IsolationError>` that acquires the write lock, calls `register_fleet_server`, and releases. Same pattern for `deregister` and `allowed_remotes`.
- [ ] Update `agent-core/orchestrator.rs` to construct a `SharedIsolationTable` at startup and clone the `Arc` into any component that needs read access (the iptables enforcement module in the next issue).
- [ ] Write unit tests: `test_concurrent_register_and_read` using `std::thread::spawn` to simulate concurrent writer and reader, asserting no data races (this test also validates the `RwLock` usage). `test_shared_table_clone_sees_updates` asserts that a cloned `Arc` reflects writes made through the original.

### Definition of done
- `SharedIsolationTable` is defined and exported.
- `register_fleet_server_shared` and `allowed_remotes` (via read lock) compile and are tested.
- Two threads concurrently accessing the table via `SharedIsolationTable` do not deadlock or panic in the unit test.
- `cargo test -p isolation` passes.

### Notes / constraints
- `std::sync::RwLock` (not `tokio::sync::RwLock`) is appropriate here because the lock is never held across an `.await` point. The critical section in `register_fleet_server` includes a DNS lookup — move the DNS resolution outside the lock before acquiring it. Take the write lock only to mutate the `Vec`.
- `Arc<RwLock<_>>` is `Send + Sync`. Cloning the `Arc` is cheap (one atomic increment).
- Do not use `Mutex` — the read path (allowed_remotes during rule generation) does not mutate state and should allow concurrent readers.

---

## Issue: Implement iptables-based enforcement: generate and apply rules from the isolation table on IsolateCommand
**Labels:** `networking`, `firewall`, `ipc`, `unsafe`
**Depends on:** Concurrency wrapper issue
**Blocks:** Isolation integration test issue

### What this is
This issue implements the actual isolation enforcement: `agent/crates/isolation/src/iptables.rs`. When the agent receives an `IsolateCommand { isolate: true }` from the fleet server, it applies iptables rules that drop all traffic except to endpoints registered in the `IsolationTable`. When it receives `IsolateCommand { isolate: false }`, it removes those rules. The implementation uses `std::process::Command` to invoke `iptables` (no external crate needed per the existing `Cargo.toml` comment).

### What is currently blocking this
The `SharedIsolationTable` from the concurrency issue must be available to read the allow-list.

### What this is blocking
The isolation integration test.

### Implementation tasks
- [ ] Create `agent/crates/isolation/src/iptables.rs`. Define `pub struct IptablesIsolator { table: SharedIsolationTable }`.
- [ ] Implement `IptablesIsolator::isolate(&self) -> Result<(), IsolationError>`. The rule set: (1) create a new chain `EDR_ISOLATION` if it doesn't exist; (2) flush it (`iptables -F EDR_ISOLATION`); (3) for each `ConnectionKey` in `table.read().allowed_remotes()`, append `iptables -A EDR_ISOLATION -d <ip> -p <proto> --dport <port> -j ACCEPT`; (4) append a default drop: `iptables -A EDR_ISOLATION -j DROP`; (5) if the `OUTPUT` chain does not already reference `EDR_ISOLATION`, append `-A OUTPUT -j EDR_ISOLATION`.
- [ ] Implement `IptablesIsolator::deisolate(&self) -> Result<(), IsolationError>`. Removes the jump from `OUTPUT`: `iptables -D OUTPUT -j EDR_ISOLATION`. Then flushes and deletes the chain: `iptables -F EDR_ISOLATION && iptables -X EDR_ISOLATION`.
- [ ] Implement `fn run_iptables(args: &[&str]) -> Result<(), IsolationError>` that executes `iptables` via `Command::new("iptables").args(args).output()`. If `status.success()` is false, return `IsolationError::IptablesExecFailed { exit_code: status.code().unwrap_or(-1), stderr: String::from_utf8_lossy(&output.stderr).into_owned() }`. If iptables is not found on PATH, return `IsolationError::IptablesNotFound`.
- [ ] Implement `IptablesIsolator::is_isolated(&self) -> Result<bool, IsolationError>` by running `iptables -L OUTPUT -n | grep EDR_ISOLATION`. Returns `true` if the chain is referenced in OUTPUT.
- [ ] In `agent/crates/agent-core/src/orchestrator.rs`, handle `ServerCommand::Isolate(cmd)` in the main loop: if `cmd.isolate == true`, call `IptablesIsolator::new(shared_table.clone()).isolate()`; if false, call `deisolate()`. Log the outcome. Update the agent's status to `AgentStatus::Isolated` or `AgentStatus::Healthy` accordingly.
- [ ] Write unit tests for `run_iptables` using a mock: define a trait `IptablesRunner` and use it in `run_iptables` to enable injection of a mock in tests. Assert that `isolate()` generates the expected iptables command arguments.

### Definition of done
- `IptablesIsolator::isolate()` and `deisolate()` compile.
- Unit tests for argument generation pass with the mock runner.
- On a Linux host with root, calling `isolate()` followed by `iptables -L OUTPUT -n` shows the `EDR_ISOLATION` chain jump. Calling `deisolate()` removes it.
- The orchestrator handles `IsolateCommand` from the gRPC stream and calls the appropriate isolator method.

### Notes / constraints
- `iptables` requires root. The agent binary must run as root or with `CAP_NET_ADMIN`. This is expected in production (same capability needed for eBPF); document it.
- The `EDR_ISOLATION` chain must be flushed before re-adding rules (idempotent isolate). If `isolate()` is called twice, the second call must not duplicate rules.
- `iptables -N EDR_ISOLATION` fails if the chain already exists (exit code 1). Treat this exit code specifically as `Ok(())` (chain already created). Check `stderr` for the specific message `"Chain already exists"`.
- IPv6 traffic requires `ip6tables`. This issue covers IPv4 only. Extend to IPv6in a follow-on.

---

## Issue: Isolation table integration test: register a connection, apply isolation, verify non-EDR traffic is blocked
**Labels:** `networking`, `firewall`, `testing`
**Depends on:** Enforcement issue
**Blocks:** nothing (leaf node in workstream)

### What this is
This issue implements the integration tests for the complete isolation pipeline: create an `IsolationTable`, register a known endpoint, apply iptables rules, verify that an outbound connection to the registered endpoint succeeds, verify that a connection to a different endpoint fails. This test requires root and a real Linux network stack. It also tests the wiring between the isolation crate and the `agent-core` orchestrator's `IsolateCommand` handler.

### What is currently blocking this
The enforcement issue must be complete.

### What this is blocking
Nothing — leaf node.

### Implementation tasks
- [ ] Create `agent/crates/isolation/tests/integration_test.rs`. Gate the file with a feature flag `isolation_integration_tests` and `#[cfg(target_os = "linux")]`.
- [ ] Implement `test_isolation_blocks_non_allowed_traffic`: (1) create `SharedIsolationTable`, (2) register `127.0.0.1:9999` as the allowed endpoint, (3) call `isolate()`, (4) spawn a `TcpListener` on `127.0.0.1:9998` (not in the allow-list), (5) attempt `TcpStream::connect("127.0.0.1:9998")` — assert connection times out or is refused, (6) call `deisolate()`, (7) assert `TcpStream::connect("127.0.0.1:9998")` now succeeds.
- [ ] Implement `test_isolation_allows_registered_endpoint`: same setup, attempt connection to `127.0.0.1:9999` — assert it succeeds while isolated.
- [ ] Implement `test_idempotent_isolate`: call `isolate()` twice; run `is_isolated()` — assert `true`. Then `deisolate()` — assert `is_isolated()` returns `false`. Verify `iptables -L OUTPUT -n` does not contain duplicate `EDR_ISOLATION` references.
- [ ] Implement `test_deisolation_restores_connectivity`: full cycle. After `deisolate()`, attempt connections to multiple ports — all should succeed.
- [ ] Add `[[test]]` section in `isolation/Cargo.toml` for the integration tests with `required-features = ["isolation_integration_tests"]`.
- [ ] Add a CI job step (in the same or a separate workflow from the eBPF CI) that runs these tests with `sudo -E cargo test -p isolation --features isolation_integration_tests`.

### Definition of done
- All four integration tests pass on a Linux host with root access and `iptables` available.
- Tests are gated by feature flag.
- CI workflow runs them in an `ubuntu-latest` environment.
- After each test, `iptables -L` shows no `EDR_ISOLATION` chain (cleanup is deterministic, using `Drop` or explicit teardown in test).

### Notes / constraints
- Connecting to `127.0.0.1` with iptables `OUTPUT` chain rules: iptables by default does apply `OUTPUT` chain rules to loopback on Linux. Verify this assumption with `sysctl net.ipv4.conf.lo.accept_local`. If loopback is exempt, use a secondary interface (e.g., a dummy interface created in the test setup).
- Use `tokio::time::timeout(Duration::from_millis(500), TcpStream::connect(...))` to detect blocked connections quickly without waiting for the OS TCP timeout.
- Ensure test cleanup (call `deisolate()` in a `Drop` guard or use `scopeguard`) to avoid leaving isolation rules active if a test panics.

---

## Issue: Create the fleet-server crate structure, async runtime, and binary entry point
**Labels:** `fleet-server`, `scaffolding`, `async`, `config`
**Depends on:** none
**Blocks:** gRPC server stub issue; config system issue; error type issue; logging issue; health endpoint issue

### What this is
`fleet-server/src/main.rs` currently contains `fn main() { println!("edr-fleet-server"); }`. The crate has all its dependencies declared in `Cargo.toml` (tokio, axum, tonic, sqlx, etc.) and a `Dockerfile` and one migration file. This issue transforms it into a real binary: Tokio runtime initialization with the correct feature flags, a structured `main.rs` that initializes subsystems in order, and the module layout that the subsequent issues will fill in. No business logic. No gRPC server implementation. No database queries. The binary must compile, start, and accept a graceful shutdown signal.

### What is currently blocking this
Nothing — root issue of Workstream C.

### What this is blocking
All other fleet-server issues depend on the module skeleton this issue creates.

### Implementation tasks
- [ ] Replace `fleet-server/src/main.rs` entirely. Use `#[tokio::main]` with the `full` feature (already in workspace `Cargo.toml`). Structure `main` as: (1) call `config::load()`, (2) call `tracing_setup::init(&cfg)`, (3) call `error::setup()` if needed, (4) build and run `server::run(cfg).await`. Return `Result<(), ServerError>`.
- [ ] Create the module files as stubs (each containing `// TODO` and correct `pub` declarations): `src/config.rs`, `src/error.rs`, `src/state.rs`, `src/server.rs`, `src/grpc/mod.rs`, `src/grpc/server.rs`, `src/db/mod.rs`, `src/http/mod.rs`, `src/http/health.rs`.
- [ ] In `src/main.rs`, add `mod config; mod error; mod state; mod server; mod grpc; mod db; mod http;` with the appropriate `pub use` re-exports.
- [ ] `src/server.rs`: define `pub async fn run(config: Config) -> Result<(), ServerError>` as a stub that prints `"Fleet server starting..."`, sleeps for 100ms, and returns `Ok(())`. This will be replaced in subsequent issues but must compile.
- [ ] Verify `cargo build -p edr-fleet-server` succeeds and `cargo run -p edr-fleet-server` prints `"Fleet server starting..."` and exits cleanly.
- [ ] Write a smoke test `test_main_returns_ok` in `src/main.rs` under `#[cfg(test)]` that calls `server::run(Config::default())` in a tokio test runtime and asserts `Ok(())`.

### Definition of done
- `cargo build -p edr-fleet-server` succeeds.
- `cargo run -p edr-fleet-server` starts, prints startup message, exits with code 0.
- All module stubs exist and are referenced in `main.rs`.
- `cargo test -p edr-fleet-server` passes (smoke test).

### Notes / constraints
- `tokio = { version = "1", features = ["full"] }` is already in workspace dependencies. Do not add redundant feature flags.
- The async runtime is tokio. The HTTP framework is axum. The gRPC framework is tonic. All are already in `Cargo.toml`. This issue does not change dependencies, only creates the structural skeleton.
- Do not add `actix-web` or any other async runtime. The architecture decision (axum + tokio) is final per the implementation guide.

---

## Issue: Implement the fleet-server configuration system
**Labels:** `fleet-server`, `config`, `scaffolding`
**Depends on:** Fleet-server crate structure issue
**Blocks:** Health endpoint issue; logging issue; gRPC server stub issue; smoke test issue

### What this is
The fleet server needs to read its configuration at startup: bind addresses for the gRPC and HTTP servers, PostgreSQL connection URL, Kafka broker addresses, JWT secret, and log level. The `config` crate is already declared in the workspace `Cargo.toml` at version `0.15`. This issue implements `fleet-server/src/config.rs` using `config = "0.15"` with a layered source: defaults → environment variables → optional config file. No secrets in code. All config fields must be readable from environment variables with the prefix `EDR_FLEET_`.

### What is currently blocking this
The crate structure issue (module stubs must exist).

### What this is blocking
The health endpoint (needs bind address), logging init (needs log level), the gRPC server stub (needs gRPC bind address and JWT secret), and the smoke test (needs `Config::default()`).

### Implementation tasks
- [ ] In `fleet-server/src/config.rs`, define `#[derive(Debug, Clone, serde::Deserialize)] pub struct Config` with fields: `pub grpc_bind: String` (default `"0.0.0.0:50051"`), `pub http_bind: String` (default `"0.0.0.0:8080"`), `pub database_url: String` (no default — required), `pub kafka_brokers: String` (default `"localhost:9092"`), `pub jwt_secret: String` (no default — required), `pub log_level: String` (default `"info"`), `pub log_format: LogFormat` where `LogFormat` is `#[derive(Debug, Clone, serde::Deserialize)] pub enum LogFormat { Human, Json }`.
- [ ] Implement `pub fn load() -> Result<Config, ConfigError>` using `config::Config::builder().add_source(config::Environment::with_prefix("EDR_FLEET")).build()?.try_deserialize()`. Use `thiserror` for `ConfigError`: variants `LoadFailed(#[from] config::ConfigError)`.
- [ ] Implement `Config::default()` manually (not via `derive`) returning the documented defaults with `database_url` and `jwt_secret` set to `"test_placeholder"` — this is only used in tests and the stub `server::run` stub.
- [ ] Add a validation step: `Config::validate(&self) -> Result<(), ConfigError>` that returns `Err` if `jwt_secret.len() < 32` (enforce minimum key length) or if `database_url` is the test placeholder in a non-test build. Use `#[cfg(not(test))]` guard.
- [ ] Write unit tests: `test_config_loads_from_env` (set `EDR_FLEET_GRPC_BIND`, `EDR_FLEET_HTTP_BIND`, `EDR_FLEET_JWT_SECRET`, etc. via `std::env::set_var`, call `load()`, assert fields match), `test_config_default_grpc_port`, `test_config_jwt_secret_too_short_fails_validation`.

### Definition of done
- `config::load()` reads from environment variables with `EDR_FLEET_` prefix.
- `Config::default()` compiles and is used by the existing smoke test stub.
- `Config::validate()` rejects a JWT secret shorter than 32 characters in non-test builds.
- `cargo test -p edr-fleet-server` passes config unit tests.
- No secrets are hardcoded — all defaults that would be credentials are test-only.

### Notes / constraints
- `config = "0.15"` uses `serde` for deserialization. The `Environment` source converts `EDR_FLEET_GRPC_BIND` to the field name `grpc_bind` by lowercasing and stripping the prefix. Verify this behavior with the unit test before finalizing.
- Do not use `dotenv` — it is not in the workspace `Cargo.toml`. Environment variables are the only config source beyond defaults.
- `LogFormat` is defined in `config.rs` and re-exported. The tracing setup issue imports it from here.

---

## Issue: Implement the fleet-server error type hierarchy
**Labels:** `fleet-server`, `error-handling`, `scaffolding`
**Depends on:** Fleet-server crate structure issue
**Blocks:** gRPC server stub issue; health endpoint issue; smoke test issue

### What this is
The fleet server needs a structured error type hierarchy that is ready to be extended as business logic is added. This issue implements `fleet-server/src/error.rs` with a top-level `ServerError` and a set of domain-specific sub-errors. All types use `thiserror`. The design must anticipate gRPC errors, database errors, JWT errors, and Kafka errors without implementing any of those subsystems yet.

### What is currently blocking this
The crate structure issue.

### What this is blocking
The gRPC server stub (which returns `Result<_, ServerError>`), the health endpoint (same), and the smoke test.

### Implementation tasks
- [ ] In `fleet-server/src/error.rs`, define `#[derive(Debug, thiserror::Error)] pub enum ServerError` with variants: `#[error("configuration error: {0}")] Config(#[from] crate::config::ConfigError)`, `#[error("database error: {0}")] Database(#[from] sqlx::Error)`, `#[error("gRPC error: {0}")] Grpc(#[from] tonic::Status)`, `#[error("JWT error: {0}")] Jwt(String)`, `#[error("Kafka error: {0}")] Kafka(String)`, `#[error("IO error: {0}")] Io(#[from] std::io::Error)`.
- [ ] Define `#[derive(Debug, thiserror::Error)] pub enum DbError` with variants: `#[error("node not found: {node_id}")] NodeNotFound { node_id: uuid::Uuid }`, `#[error("duplicate enrollment: machine_id={machine_id}")] DuplicateEnrollment { machine_id: String }`, `#[error("sqlx: {0}")] Sqlx(#[from] sqlx::Error)`.
- [ ] Define `#[derive(Debug, thiserror::Error)] pub enum GrpcError` with variants: `#[error("unauthenticated")] Unauthenticated`, `#[error("node not enrolled")] NotEnrolled`, `#[error("stream closed")] StreamClosed`.
- [ ] Implement `From<GrpcError> for tonic::Status`: `Unauthenticated` → `Status::unauthenticated(msg)`, `NotEnrolled` → `Status::not_found(msg)`, `StreamClosed` → `Status::cancelled(msg)`.
- [ ] Re-export `ServerError`, `DbError`, `GrpcError` from `fleet-server/src/error.rs` and add `pub use error::{ServerError, DbError, GrpcError};` to `src/lib.rs` if a lib target is added, or use path imports in `main.rs`.
- [ ] Write unit tests: `test_db_error_display_node_not_found`, `test_grpc_error_converts_to_tonic_status_unauthenticated`, `test_server_error_from_io_error`.

### Definition of done
- `cargo test -p edr-fleet-server` passes all error type unit tests.
- `ServerError`, `DbError`, and `GrpcError` are defined and compile with `thiserror`.
- `From<GrpcError> for tonic::Status` is implemented and tested.
- The hierarchy is extensible: adding a new variant to any error enum requires no changes outside that enum.

### Notes / constraints
- `thiserror = "2"` is in the workspace (note: version 2, not 1). Use `{ workspace = true }`.
- `tonic::Status` has constructor methods like `Status::unauthenticated(message: impl Into<String>)`. Use these — do not construct the struct directly.
- Keep `Kafka(String)` as a `String`-wrapping variant for now because `rdkafka` is commented out in the workspace `Cargo.toml`. When rdkafka is enabled, replace it with a proper `From<rdkafka::error::KafkaError>` impl.

---

## Issue: Implement structured logging and tracing setup for the fleet-server
**Labels:** `fleet-server`, `tracing`, `scaffolding`
**Depends on:** Configuration system issue
**Blocks:** Health endpoint issue; gRPC server stub issue; smoke test issue

### What this is
The fleet server must emit structured JSON logs in production (for log aggregation) and human-readable logs in development. `tracing` and `tracing-subscriber` are already in the workspace. This issue implements a thin initialization shim in `fleet-server/src/` that reads `Config.log_level` and `Config.log_format`, builds the appropriate `tracing_subscriber` stack, and installs it as the global default. It must run before any `tracing::info!` calls and must not panic if called twice (idempotent for test use).

### What is currently blocking this
The config system must exist so `LogFormat` and `log_level` are defined.

### What this is blocking
Everything that emits logs (health endpoint, gRPC stub, smoke test).

### Implementation tasks
- [ ] Create `fleet-server/src/tracing_setup.rs` (avoid naming it `tracing.rs` to prevent shadowing the `tracing` crate). Define `pub fn init(config: &Config) -> Result<(), ServerError>`.
- [ ] In `init`, build the `EnvFilter` from `config.log_level` using `EnvFilter::try_new(&config.log_level).unwrap_or_else(|_| EnvFilter::new("info"))`.
- [ ] For `LogFormat::Json`: use `tracing_subscriber::fmt().json().with_env_filter(filter).with_current_span(true).with_span_list(true).try_init()`. Map the `Err` from `try_init` (which fires if a global subscriber is already set) to `Ok(())` rather than returning an error — this makes the function safe to call from tests.
- [ ] For `LogFormat::Human`: use `tracing_subscriber::fmt().with_env_filter(filter).pretty().try_init()` with the same error-swallowing behavior.
- [ ] Add the module to `src/main.rs`: `mod tracing_setup;` and call `tracing_setup::init(&config)?;` as the second step in `main` (after config load).
- [ ] Write a unit test `test_init_human_format_does_not_panic` and `test_init_json_format_does_not_panic` — both call `tracing_setup::init(&Config::default())` in a tokio test runtime and assert `Ok(())`.
- [ ] Verify that `cargo run -p edr-fleet-server` with `EDR_FLEET_LOG_FORMAT=json` produces JSON-structured log lines to stdout.

### Definition of done
- `tracing_setup::init` compiles and runs without panic.
- JSON format produces parseable JSON lines (verify with `cargo run | jq .` during manual testing).
- Human format produces colored, pretty output when `EDR_FLEET_LOG_FORMAT` is unset.
- Unit tests pass.
- `try_init` is used (not `init`) so tests can call `init` multiple times without panicking.

### Notes / constraints
- `tracing_subscriber = { version = "0.3", features = ["env-filter", "json"] }` is in the workspace. The `json` feature is required for `.json()` on the `SubscriberBuilder`.
- Do not use `RUST_LOG` as the only env override — the config-driven `log_level` provides a programmatic default independent of `RUST_LOG`. The `EnvFilter` will still respect `RUST_LOG` if set (it checks it first).
- `tracing_setup` is not `tracing` — do not shadow the external crate name.

---

## Issue: Implement graceful shutdown with SIGTERM and SIGINT handling for the fleet-server
**Labels:** `fleet-server`, `scaffolding`, `async`
**Depends on:** Fleet-server crate structure issue; logging issue
**Blocks:** Smoke test issue

### What this is
The fleet server must shut down cleanly when it receives `SIGTERM` (from the container orchestrator stopping the pod) or `SIGINT` (from `Ctrl-C` in development). "Cleanly" at the scaffolding level means: accept the signal, log the shutdown intent, cancel a `CancellationToken` that all subsystems will be given, and exit `main` with code 0. The actual drain logic (waiting for in-flight gRPC calls to complete, flushing Kafka producers) is stubbed here and implemented when those subsystems are built.

### What is currently blocking this
The crate structure and logging issues must be in place.

### What this is blocking
The smoke test (which verifies `SIGTERM` causes clean exit).

### Implementation tasks
- [ ] In `fleet-server/src/server.rs`, replace the stub `run` function with a real implementation: (1) create a `tokio_util::sync::CancellationToken` (add `tokio-util = { version = "0.7", features = ["sync"] }` if not already in scope — it is in the workspace), (2) spawn a signal handler task using `tokio::signal::ctrl_c()` and `tokio::signal::unix::signal(SignalKind::terminate())`, (3) when either signal fires, call `token.cancel()` and log `"Received shutdown signal — initiating graceful shutdown"`, (4) pass the token to all subsystem runners (stubs for now), (5) await all subsystem handles, (6) log `"Fleet server stopped"` and return `Ok(())`.
- [ ] In `src/main.rs`, the existing `server::run(cfg).await?` call now becomes the full lifecycle. Ensure the process exits with code 0 on clean shutdown and code 1 on `Err`.
- [ ] Implement `pub struct ShutdownHandle { token: CancellationToken }` with `ShutdownHandle::new() -> (ShutdownHandle, CancellationToken)` and `ShutdownHandle::wait(self) -> impl Future<Output = ()>` that awaits the signal before cancelling. Expose this from `src/server.rs`.
- [ ] Stub the shutdown drain as `async fn drain_subsystems(_token: CancellationToken) { tokio::time::sleep(Duration::from_millis(100)).await; tracing::info!("All subsystems drained"); }`. This is a placeholder that future issues replace.
- [ ] Write a unit test `test_shutdown_on_cancellation_token` that creates a `CancellationToken`, cancels it immediately, and asserts that `drain_subsystems(token)` returns within 200ms.

### Definition of done
- `cargo run -p edr-fleet-server` starts and exits cleanly when `Ctrl-C` is pressed (logs shutdown message, exits code 0).
- On `SIGTERM` (test with `kill -TERM <pid>` in a separate terminal), the same clean shutdown occurs.
- The `CancellationToken` is threaded through `server::run` and passed to all subsystem stubs.
- Unit test passes.

### Notes / constraints
- `tokio::signal::unix` is only available on Unix targets. Wrap with `#[cfg(unix)]`. For `#[cfg(windows)]` (if needed in future), use `tokio::signal::ctrl_c()` only.
- `tokio::signal::unix::signal(SignalKind::terminate())` returns a `Signal` stream. Use `signal.recv().await` inside a `tokio::select!`.
- Do not call `std::process::exit()` — let `main` return naturally. Calling `exit()` bypasses `Drop` impls and can leave resources in a dirty state.

---

## Issue: Implement the health check HTTP endpoint on the fleet-server
**Labels:** `fleet-server`, `scaffolding`, `async`
**Depends on:** Configuration system issue; logging issue; error type issue; graceful shutdown issue
**Blocks:** Smoke test issue

### What this is
The fleet server exposes an HTTP port (`http_bind`, default `0.0.0.0:8080`) for health checks and future admin routes. This issue implements a single route: `GET /health` returning `{"status":"ok"}` with HTTP 200. The HTTP server runs concurrently with the gRPC server (the gRPC stub issue) using axum. Both servers share the `CancellationToken` for coordinated shutdown.

### What is currently blocking this
Config (for `http_bind`), error types (for `ServerError`), logging (so startup is visible), and graceful shutdown (for the CancellationToken).

### What this is blocking
The smoke test (which calls GET /health and asserts 200).

### Implementation tasks
- [ ] In `fleet-server/src/http/mod.rs`, define `pub async fn serve(bind: &str, token: CancellationToken) -> Result<(), ServerError>`.
- [ ] In `fleet-server/src/http/health.rs`, define `pub async fn health_handler() -> impl IntoResponse { axum::Json(serde_json::json!({"status": "ok"})) }`.
- [ ] In `http/mod.rs`, build the axum router: `Router::new().route("/health", get(health::health_handler))`. Bind with `TcpListener::bind(bind).await?` and serve with `axum::serve(listener, router).with_graceful_shutdown(token.cancelled())`.
- [ ] In `fleet-server/src/server.rs`, alongside the existing shutdown stub, spawn `http::serve(&config.http_bind, token.clone())` as a `tokio::spawn` handle. Await it in the drain step.
- [ ] Add `tower-http` trace middleware: wrap the router with `.layer(TraceLayer::new_for_http())` using `tower_http::trace::TraceLayer`.
- [ ] Write a unit test `test_health_endpoint_returns_200` using `axum::test` (`axum::serve` test helpers or `hyper` test client): build the router, send `GET /health`, assert status 200 and body `{"status":"ok"}`.

### Definition of done
- `cargo run -p edr-fleet-server` starts the HTTP server on port 8080.
- `curl http://localhost:8080/health` returns `{"status":"ok"}` with HTTP 200.
- Sending SIGTERM after startup causes the HTTP server to stop accepting new connections and returns from `serve` cleanly.
- Unit test `test_health_endpoint_returns_200` passes.

### Notes / constraints
- `axum = { version = "0.8", features = ["ws", "macros"] }` is in the workspace. `axum::serve` is the axum 0.8 API (not the older `axum::Server`).
- `axum::serve(...).with_graceful_shutdown(token.cancelled_owned())` requires `CancellationToken::cancelled_owned()` from `tokio-util`. This is available in `tokio-util = "0.7"`.
- The health endpoint must not require authentication — it is called by load balancers and Kubernetes liveness probes.
- Do not implement any other routes in this issue. `/metrics`, `/nodes`, and any other admin routes are out of scope for scaffolding.

---

## Issue: Implement the gRPC server stub: bind the tonic server to its port, register the FleetService, return Unimplemented
**Labels:** `fleet-server`, `scaffolding`, `async`, `ipc`
**Depends on:** Configuration system issue; error type issue; logging issue; graceful shutdown issue
**Blocks:** Smoke test issue

### What this is
The fleet server's primary interface is a tonic gRPC server implementing the `FleetService` defined in `sdk/proto/fleet.proto`. This issue adds the gRPC server stub: the `tonic::transport::Server` that binds to `config.grpc_bind`, registers a `FleetServiceImpl` struct, and returns `Status::unimplemented()` for all three RPCs (`RegisterAgent`, `EventStream`, `Heartbeat`). This is the minimal skeleton that compiles, binds the port, and is ready for actual implementation in downstream issues outside this workstream.

### What is currently blocking this
Config (for `grpc_bind`), error types (for mapping gRPC errors), logging, and graceful shutdown (for the shutdown future). The `sdk` crate must expose the generated tonic service trait — check `sdk/src/lib.rs` is a stub and may need a `build.rs` to compile protos. Note: this issue may surface a dependency on the SDK build system.

### What this is blocking
The smoke test (which verifies the gRPC port is bound).

### Implementation tasks
- [ ] Verify `sdk/src/lib.rs` exposes the generated `fleet_service_server::FleetService` trait from `sdk/proto/fleet.proto`. If `sdk/build.rs` does not yet exist, add it: `fn main() -> Result<(), Box<dyn std::error::Error>> { tonic_build::configure().compile_protos(&["proto/fleet.proto", "proto/agent.proto", "proto/events.proto"], &["proto/"])?; Ok(()) }`. Add `build-dependencies = [ tonic-build ]` to `sdk/Cargo.toml`. This is a prerequisite step — it should be done as part of this issue or tracked as a blocking note.
- [ ] In `fleet-server/src/grpc/server.rs`, define `pub struct FleetServiceImpl;` and implement the tonic-generated `FleetService` trait. All three methods return `Err(Status::unimplemented("not yet implemented"))`. The streaming RPCs return a `Result<Response<Self::EventStreamStream>, Status>` where `Self::EventStreamStream` is a `Pin<Box<dyn Stream<...>>>`.
- [ ] In `fleet-server/src/grpc/mod.rs`, define `pub async fn serve(bind: &str, token: CancellationToken) -> Result<(), ServerError>`. Use `tonic::transport::Server::builder().add_service(FleetServiceServer::new(FleetServiceImpl)).serve_with_shutdown(addr, token.cancelled_owned()).await?`.
- [ ] In `fleet-server/src/server.rs`, spawn `grpc::serve(&config.grpc_bind, token.clone())` alongside the HTTP server.
- [ ] Write a unit test `test_grpc_server_binds_and_returns_unimplemented`: start the gRPC server on an ephemeral port, connect with a tonic client, call `RegisterAgent`, assert the response is `Status::unimplemented`.

### Definition of done
- `cargo build -p edr-fleet-server` succeeds with the gRPC stub present.
- `cargo run -p edr-fleet-server` logs the gRPC bind address and accepts connections on port 50051.
- `grpcurl -plaintext localhost:50051 edr.fleet.FleetService/RegisterAgent` returns `Unimplemented` (not `connection refused`).
- Unit test passes.

### Notes / constraints
- `tonic = "0.14"` is in the workspace. The generated `FleetServiceServer` requires `tonic::async_trait` on the impl block. Use `#[tonic::async_trait]`.
- The `EventStream` RPC is bidirectional streaming. The `EventStreamStream` associated type must be `Pin<Box<dyn Stream<Item = Result<ServerCommand, Status>> + Send + 'static>>`. Return an empty stream for the stub: `Ok(Response::new(Box::pin(tokio_stream::empty())))`.
- `sdk/src/lib.rs` is currently a single-line comment. Adding `build.rs` to the SDK and exposing the generated types is a prerequisite. If the SDK build is not in scope for this issue, use the existing `testing.proto` in `fleet-server/src/grpc/` as a local stand-in — but this is a workaround. The clean solution is to fix the SDK build.

---

## Issue: Fleet-server smoke test: server starts, binds, returns 200 on health, shuts down cleanly
**Labels:** `fleet-server`, `testing`, `scaffolding`
**Depends on:** Health endpoint issue; gRPC server stub issue; logging issue; graceful shutdown issue
**Blocks:** nothing (leaf node in workstream)

### What this is
With all scaffolding pieces in place (config, tracing, error types, shutdown, health endpoint, gRPC stub), this issue implements a single integration test that exercises the full startup-to-shutdown lifecycle of the fleet server binary. This is the "does it all hang together" gate before any business logic is built on top.

### What is currently blocking this
All prior fleet-server scaffolding issues must be complete.

### What this is blocking
Nothing — leaf node of Workstream C.

### Implementation tasks
- [ ] Create `fleet-server/tests/smoke_test.rs`. This is an integration test (in `tests/`, not `src/`), so it tests the public API of the crate.
- [ ] Implement `test_health_endpoint_returns_ok`: (1) build a `Config` with `http_bind = "127.0.0.1:0"` (port 0 = OS assigns ephemeral), `grpc_bind = "127.0.0.1:0"`, `log_level = "error"` (quiet), `jwt_secret = "a".repeat(32)`, `database_url = "postgres://unused"`, (2) spawn `server::run(config)` in a `tokio::spawn`, (3) poll `http://127.0.0.1:<actual_port>/health` with a short timeout (use `reqwest` or `axum`'s test helpers), (4) assert HTTP 200 and body `{"status":"ok"}`, (5) cancel the `CancellationToken`, (6) await the server task, assert it exits with `Ok(())`.
- [ ] Add `reqwest` as a dev dependency in `fleet-server/Cargo.toml`: `reqwest = { version = "0.12", features = ["json"], default-features = false }` — or use `hyper` directly to avoid a heavy dependency.
- [ ] Implement `test_grpc_port_is_bound`: after startup, attempt a TCP connection to `127.0.0.1:<grpc_port>`. Assert the connection is accepted (not refused). This does not require a gRPC client — just a raw `TcpStream::connect`.
- [ ] Implement `test_graceful_shutdown_exits_zero`: cancel the token, await the server future, assert the `Result` is `Ok(())`. Assert that no log lines at `ERROR` level were emitted during the test run (use `tracing_test` crate if available, or skip this assertion).
- [ ] Note the challenge of binding on port 0 with axum: `axum::serve` needs to know the actual bound port. Expose a `BoundServer` struct from `http::serve` that holds the actual `SocketAddr`. This requires a small API change to `http/mod.rs`.

### Definition of done
- `cargo test -p edr-fleet-server` (including `tests/smoke_test.rs`) passes all three smoke tests.
- The tests do not require a running PostgreSQL, Kafka, or any external service.
- The test binary exits with code 0 and leaves no listening sockets behind.
- Test runtime is under 5 seconds total (no real sleep calls — use `tokio::time::timeout` for all waits).

### Notes / constraints
- Port 0 binding: pass `"127.0.0.1:0"` to `TcpListener::bind`, then call `listener.local_addr()` to get the actual port before passing the listener to axum. This requires `http::serve` to take a `TcpListener` instead of a `&str` bind address — or expose the bound address through a `oneshot` channel.
- The gRPC port 0 binding with tonic: `Server::builder().serve_with_shutdown(addr, ...)` where `addr` is `"127.0.0.1:0".parse().unwrap()`. Tonic internally binds and assigns an ephemeral port, but does not expose the actual port without additional hooks. For the smoke test, use a fixed high port (e.g., 59050) known to be available, and skip this test if the port is in use.
- Do not test actual gRPC RPC calls in this smoke test — that is integration testing, not scaffolding testing.

---
