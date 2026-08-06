# Rust EDR Agent — Complete Developer Guide

> **Persona**: Senior Rust engineer + Linux kernel internals expert + osquery contributor +
> security engineer. Every decision here is grounded in production experience and kernel-level
> understanding. Linux-only; covers Debian/Ubuntu, RHEL/Rocky/CentOS, Fedora, Arch, and any
> `systemd`-based distro on `x86_64` and `aarch64`.

---

## Table of Contents

1. [System Architecture Overview](#1-system-architecture-overview)
2. [Project Structure & Cargo.toml](#2-project-structure--cargotoml)
3. [The osquery Thrift API — Deep Internals](#3-the-osquery-thrift-api--deep-internals)
4. [Querying osquery via Unix Socket in Rust](#4-querying-osquery-via-unix-socket-in-rust)
5. [Writing Custom osquery Extensions in Rust](#5-writing-custom-osquery-extensions-in-rust)
6. [Custom Table Implementation](#6-custom-table-implementation)
7. [Logger Plugin Implementation](#7-logger-plugin-implementation)
8. [gRPC Transport — Protobuf + Tonic](#8-grpc-transport--protobuf--tonic)
9. [Tokio Agent Core — All Tasks & Channels](#9-tokio-agent-core--all-tasks--channels)
10. [Local Event Buffering — redb vs sled vs SQLite](#10-local-event-buffering--redb-vs-sled-vs-sqlite)
11. [Distro Detection & osquery Auto-Install](#11-distro-detection--osquery-auto-install)
12. [Packaging & Delivery Strategy](#12-packaging--delivery-strategy)
13. [Fleet-Based Config Deployment — config.toml](#13-fleet-based-config-deployment--configtoml)
14. [Systemd Service — Full Setup](#14-systemd-service--full-setup)
15. [Startup Sequence & Boot Ordering](#15-startup-sequence--boot-ordering)
16. [Logging with tracing](#16-logging-with-tracing)
17. [Heartbeat & System Metrics to Fleet](#17-heartbeat--system-metrics-to-fleet)
18. [Network Isolation with nftables](#18-network-isolation-with-nftables)
19. [Enrollment Secret & mTLS Authentication](#19-enrollment-secret--mtls-authentication)
20. [Config Hot-Reload from Fleet Server](#20-config-hot-reload-from-fleet-server)
21. [GitHub Actions — CI + Cross-Compilation + Releases](#21-github-actions--ci--cross-compilation--releases)
22. [Benchmarking the Agent](#22-benchmarking-the-agent)
23. [eBPF Event Loss — Probability & Mitigation](#23-ebpf-event-loss--probability--mitigation)
24. [Debugging Deep Dive](#24-debugging-deep-dive)
25. [Security Hardening](#25-security-hardening)
26. [Resources for Further Deep Dive](#26-resources-for-further-deep-dive)

---

## 1. System Architecture Overview

```
╔═══════════════════════════════════════════════════════════════════════╗
║                       Fleet Server (your backend)                      ║
║  gRPC services:  EnrollService | QueryService | ConfigService          ║
║  Port: 8443 (TLS), mTLS auth via enrollment cert                       ║
╚═══════════════════╦═══════════════════════════════════════════════════╝
                    ║  bidirectional gRPC stream + unary RPCs
                    ║  protobuf over HTTP/2 + TLS
    ╔═══════════════╩═════════════════════════════════════════════════╗
    ║              EDR Agent Process  (Rust / tokio)                   ║
    ║                                                                   ║
    ║  ┌─────────────────────────────────────────────────────────┐     ║
    ║  │                    tokio Runtime                          │     ║
    ║  │                                                           │     ║
    ║  │  task: grpc_uplink  ──┐    ┌── task: heartbeat_loop      │     ║
    ║  │  task: event_drain  ──┤    ├── task: config_watcher      │     ║
    ║  │  task: scheduler    ──┼────┤    (inotify on toml file)   │     ║
    ║  │  task: ext_watchdog ──┘    └── task: metrics_collector   │     ║
    ║  │                                                           │     ║
    ║  │  ┌──────────────┐  mpsc  ┌───────────────────────────┐  │     ║
    ║  │  │ osquery       │ ─────► │    redb local buffer       │  │     ║
    ║  │  │ Thrift Client │       │  (write-ahead log of        │  │     ║
    ║  │  │ (UDS)         │       │   unacked events)           │  │     ║
    ║  │  └──────────────┘       └────────────┬──────────────--┘  │     ║
    ║  │                                       │ drain when online │     ║
    ║  │  ┌──────────────┐                    ▼                    │     ║
    ║  │  │ Extension     │         ┌─────────────────────┐        │     ║
    ║  │  │ Server        │         │   gRPC uplink queue │        │     ║
    ║  │  │ (Thrift srv)  │         │  (tokio mpsc chan)  │        │     ║
    ║  │  └──────────────┘         └─────────────────────┘        │     ║
    ║  └─────────────────────────────────────────────────────────-─┘     ║
    ║                                                                   ║
    ║  ┌──────────────────────────────────────────────┐               ║
    ║  │    nftables isolation rules (when commanded)  │               ║
    ║  │    ALLOW: 8443/tcp to fleet-server-ip only    │               ║
    ║  │    DROP: all other egress/ingress             │               ║
    ║  └──────────────────────────────────────────────┘               ║
    ╚═══════════════════════════════════════════════════════════════════╝
                    ║
                    ║ Unix domain socket:
                    ║ /var/osquery/osquery.em (Thrift/IPC)
    ╔═══════════════╩══════════════════════════════════════════╗
    ║              osqueryd  (separate process)                 ║
    ║  Managed by: systemd (osqueryd.service)                   ║
    ║  Installed by: agent installer script (per-distro)        ║
    ║  Config: /etc/osquery/osquery.{flags,conf}                ║
    ╚═══════════════════════════════════════════════════════════╝
```

### Two-Process Design

The agent and osqueryd are **separate processes**:

- The agent owns the gRPC channel, local buffer, and telemetry shipping
- osqueryd owns the kernel instrumentation (audit, eBPF, inotify)
- They communicate **only** via the osquery extension manager Unix socket
- If osqueryd crashes, the agent continues buffering from its last checkpoint
- If the agent crashes, osqueryd continues collecting events (buffered in RocksDB)

This mirrors how production systems like Fleet (by Kolide) and Uptycs work. The
separation of concerns means one crash does not cascade.

---

## 2. Project Structure & Cargo.toml

### Workspace Layout

```
edr-agent/
├── Cargo.toml              ← workspace root
├── Cargo.lock
├── build.rs                ← tonic-build for .proto compilation
├── proto/
│   └── edr.proto           ← all protobuf definitions
├── src/
│   ├── main.rs             ← agent entry point
│   ├── agent.rs            ← Agent struct, lifecycle
│   ├── config.rs           ← config.toml parsing + hot-reload
│   ├── osquery/
│   │   ├── mod.rs
│   │   ├── client.rs       ← Thrift client (query executor)
│   │   ├── extension.rs    ← Extension server (register tables)
│   │   └── installer.rs    ← distro detect + osquery install
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── grpc.rs         ← tonic gRPC client + streaming
│   │   └── buffer.rs       ← redb local event buffer
│   ├── metrics.rs          ← sysinfo CPU/RAM/disk collection
│   ├── heartbeat.rs        ← periodic heartbeat to fleet
│   ├── isolation.rs        ← nftables network isolation
│   ├── enrollment.rs       ← mTLS cert + enrollment secret
│   └── tables/
│       ├── mod.rs
│       ├── proc_hidden.rs  ← custom table: hidden processes
│       └── kernel_threads.rs
├── install/
│   ├── install.sh          ← one-shot installer (downloads osquery)
│   ├── uninstall.sh
│   └── config.toml.example ← fleet config template
├── systemd/
│   ├── edr-agent.service
│   └── edr-agent-watchdog.conf
└── .github/
    └── workflows/
        ├── ci.yml
        └── release.yml
```

### Cargo.toml

```toml
[workspace]
resolver = "2"
members = [".", "crates/osquery-ext"]

[package]
name = "edr-agent"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
authors = ["Your Team <security@yourcompany.com>"]
description = "EDR agent with osquery integration"
license = "Apache-2.0"

# Build for production: small + fast
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"          # smaller binary + no unwinding cost
strip = "symbols"        # strip debug symbols from release binary

# Dev profile: faster compile, debug info
[profile.dev]
opt-level = 0
debug = true

[[bin]]
name = "edr-agent"
path = "src/main.rs"

[dependencies]
# ── Async Runtime ──────────────────────────────────────────────────────
tokio = { version = "1", features = [
    "macros", "rt-multi-thread", "signal",
    "sync", "time", "fs", "process", "net",
    "io-util"
] }
tokio-stream = "0.1"
tokio-util = { version = "0.7", features = ["codec"] }
futures = "0.3"

# ── gRPC / Protobuf ────────────────────────────────────────────────────
tonic = { version = "0.12", features = ["tls", "tls-roots", "gzip"] }
prost = "0.13"
prost-types = "0.13"

# ── Thrift (osquery IPC over Unix socket) ─────────────────────────────
# The thrift crate added Unix domain socket support in 0.17
thrift = "0.17"

# ── osquery Rust bindings (extension server) ──────────────────────────
# osquery-rust-ng: supports table + logger plugins, pure Rust, no C deps
# Contributed UDS support to upstream thrift crate
osquery-rust-ng = "0.1"

# ── Local Buffer (embedded database) ──────────────────────────────────
# redb: pure Rust, ACID, zero C deps, no separate process
# Better choice than sled (sled is in maintenance mode as of 2024)
redb = "2"

# ── Serialization ─────────────────────────────────────────────────────
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# ── Configuration ─────────────────────────────────────────────────────
config = { version = "0.14", features = ["toml"] }
clap = { version = "4", features = ["derive", "env"] }

# ── Error Handling ─────────────────────────────────────────────────────
anyhow = "1"
thiserror = "1"

# ── Logging / Tracing ─────────────────────────────────────────────────
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = [
    "env-filter", "json", "fmt"
] }
tracing-appender = "0.2"    # non-blocking file appender

# ── System Metrics ─────────────────────────────────────────────────────
sysinfo = "0.30"             # CPU, RAM, disk, process info

# ── Unix / Linux Syscalls ─────────────────────────────────────────────
nix = { version = "0.28", features = [
    "process", "signal", "user", "socket", "fs"
] }

# ── nftables (network isolation) ─────────────────────────────────────
nftables = "0.4"

# ── systemd integration ────────────────────────────────────────────────
sd-notify = "0.4"

# ── File watching (config hot-reload) ─────────────────────────────────
notify = { version = "6", features = ["macos_fsevent"] }

# ── HTTP (download osquery during install) ────────────────────────────
reqwest = { version = "0.12", features = [
    "stream", "rustls-tls", "blocking", "json"
], default-features = false }

# ── Crypto (SHA-256 verify downloads, enrollment secret) ─────────────
sha2 = "0.10"
hex = "0.4"
uuid = { version = "1", features = ["v4", "serde"] }
ring = "0.17"               # TLS key generation, HMAC

# ── TLS / mTLS Certificates ───────────────────────────────────────────
rustls = { version = "0.23", features = ["ring"] }
rustls-pemfile = "2"
rcgen = "0.13"              # generate self-signed enrollment cert

# ── Compression ───────────────────────────────────────────────────────
flate2 = "1"
zstd = "0.13"

# ── Misc Utils ────────────────────────────────────────────────────────
which = "6"                 # find binaries in PATH
once_cell = "1"             # global singletons
parking_lot = "0.12"        # faster Mutex/RwLock
bytes = "1"
chrono = { version = "0.4", features = ["serde"] }

[build-dependencies]
tonic-build = "0.12"

[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio"] }
tokio-test = "0.4"
tempfile = "3"
```

> **Why `redb` over `sled`?** `sled` 0.34 has been in a partial rewrite cycle
> for years and has known data-loss bugs under high write load. `redb` reached
> 1.0 in 2023, is actively maintained by the same author as `Crowbar`, has
> ACID guarantees, and has zero C FFI. For buffering event telemetry with
> durability requirements, `redb` is the correct choice in 2024+.
>
> **Why NOT use osquery's own RocksDB?** osquery locks its RocksDB with a
> file lock (`LOCK` file). Accessing it from the agent process would conflict,
> cause data corruption, and can prevent osqueryd from starting. Never open
> osquery's database from the agent. Create your own.

---

## 3. The osquery Thrift API — Deep Internals

osquery uses **Apache Thrift** over a **Unix Domain Socket** for IPC. This is
not TCP — it is a `AF_UNIX / SOCK_STREAM` socket. The socket path is controlled
by `--extensions_socket` (default `/var/osquery/osquery.em`).

### Full osquery.thrift IDL

This is the complete interface definition. Save this as `osquery.thrift` for
reference — you do NOT need to compile it; `osquery-rust-ng` and `thrift` crate
handle this.

```thrift
namespace cpp osquery.extensions
namespace py osquery.extensions
namespace go osquery.extensions

// ── Types ──────────────────────────────────────────────────────────────
typedef map<string, string> ExtensionPluginRequest
typedef list<map<string, string>> ExtensionPluginResponse

struct InternalOptionInfo {
  1: string value,
  2: string default_value,
  3: string type,
}
typedef map<string, InternalOptionInfo> InternalOptionList

struct InternalExtensionInfo {
  1: string name,
  2: string version,
  3: string sdk_version,
  4: string min_sdk_version,
}

typedef i64 ExtensionRouteUUID
typedef map<string, ExtensionPluginResponse> ExtensionRouteTable
typedef map<string, ExtensionRouteTable> ExtensionRegistry
typedef map<ExtensionRouteUUID, InternalExtensionInfo> InternalExtensionList

enum ExtensionCode {
  EXT_SUCCESS = 0,
  EXT_FAILED  = 1,
  EXT_FATAL   = 2,
}

struct ExtensionStatus {
  1: i32 code,
  2: string message,
  3: ExtensionRouteUUID uuid,
}

struct ExtensionResponse {
  1: ExtensionStatus status,
  2: ExtensionPluginResponse response,
}

exception ExtensionException {
  1: i32 code,
  2: string message,
  3: ExtensionRouteUUID uuid,
}

// ── Extension service (implemented BY extensions) ─────────────────────
service Extension {
  ExtensionStatus ping(),
  ExtensionResponse call(
    1: string registry,           // "table", "logger", "config"
    2: string item,               // plugin name
    3: ExtensionPluginRequest request
  ),
  void shutdown(),
}

// ── ExtensionManager service (implemented BY osqueryd) ────────────────
service ExtensionManager extends Extension {
  InternalExtensionList extensions(),
  InternalOptionList options(),

  // Register your extension's plugins with osqueryd
  ExtensionStatus registerExtension(
    1: InternalExtensionInfo info,
    2: ExtensionRegistry registry
  ),

  ExtensionStatus deregisterExtension(
    1: ExtensionRouteUUID uuid,
  ),

  // Execute SQL and get results back
  ExtensionResponse query(1: string sql),

  // Introspect a SQL query to get column types
  ExtensionResponse getQueryColumns(1: string sql),
}
```

### Protocol Flow: How an Extension Registers

```
Extension Process                           osqueryd (ExtensionManager)
─────────────────                           ────────────────────────────
                                            listen on /var/osquery/osquery.em
connect to socket ──────────────────────►
                  ◄──────────────────────── accept connection

call registerExtension(info, registry) ──►
  info = { name="my_ext", version="1.0",
           sdk_version="5.14.0",
           min_sdk_version="5.0.0" }
  registry = {
    "table": {
      "my_custom_table": [
        {"id": "column", "name": "pid",    "type": "INTEGER"},
        {"id": "column", "name": "path",   "type": "TEXT"},
      ]
    }
  }
                  ◄──────────────────────── ExtensionStatus{code=0, uuid=42}

# At this point "my_custom_table" is visible in osquery:
# osquery> SELECT * FROM my_custom_table;

# osqueryd calls back to extension when table is queried:
                  ◄──────────────────────── call("table", "my_custom_table",
                                                 {"action": "generate",
                                                  "context": "{}"})

respond with rows ──────────────────────►
  ExtensionResponse {
    status: {code: 0},
    response: [
      {"pid": "1234", "path": "/usr/bin/bash"},
    ]
  }
```

### How Query Results Flow

```
Agent calls ExtensionManager.query("SELECT pid FROM processes LIMIT 5")
                                        │
                                        ▼
osqueryd executes SQL via SQLite virtual table layer
                                        │
                                        ▼
Returns: ExtensionResponse {
  status: {code: 0, message: "OK"},
  response: [
    {"pid": "1"},    ← each map<string,string> is a row
    {"pid": "1234"},
    ...
  ]
}
```

---

## 4. Querying osquery via Unix Socket in Rust

### Method A: Using `osquery-rs` (simple query executor)

```rust
// src/osquery/client.rs
use std::time::Duration;
use anyhow::{Context, Result};

/// Re-exported from osquery-rs
pub use osquery_rs::OSQuery;

/// Wrapper that adds retry logic and connection management
pub struct OsqueryClient {
    socket_path: String,
}

impl OsqueryClient {
    pub fn new(socket_path: &str) -> Self {
        Self { socket_path: socket_path.to_owned() }
    }

    /// Execute a SQL query against osqueryd via the extension socket.
    /// Returns rows as Vec<HashMap<String, String>>.
    pub fn query(&self, sql: &str) -> Result<Vec<std::collections::HashMap<String, String>>> {
        let result = OSQuery::new()
            .set_socket(&self.socket_path)
            .query(sql.to_owned())
            .with_context(|| format!("osquery query failed: {}", sql))?;
        Ok(result)
    }

    /// Check if osqueryd is alive
    pub fn ping(&self) -> bool {
        self.query("SELECT 1 AS alive;").is_ok()
    }

    /// Wait for osqueryd socket to appear (called after osqueryd starts)
    pub async fn wait_for_socket(&self, timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if self.ping() { return Ok(()); }
            if tokio::time::Instant::now() >= deadline {
                anyhow::bail!("Timed out waiting for osquery socket: {}", self.socket_path);
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
```

### Method B: Raw Thrift Client (manual implementation for full control)

This approach lets you call `ExtensionManager` methods directly — useful if
you need `registerExtension`, `getQueryColumns`, or `options()`.

```rust
// src/osquery/thrift_client.rs
//
// The thrift crate (0.17+) supports Unix domain sockets.
// osquery-rust-ng builds on top of this.
//
// Low-level usage pattern:

use thrift::protocol::{TBinaryInputProtocol, TBinaryOutputProtocol};
use thrift::transport::{
    TFramedReadTransport, TFramedWriteTransport,
    ReadHalf, WriteHalf,
};
use std::os::unix::net::UnixStream;
use std::time::Duration;

// The generated client from osquery.thrift
// In practice, use osquery-rust-ng which already has this generated.
// Shown here as a conceptual demonstration:
//
// pub struct ExtensionManagerSyncClient<IP, OP> { ... }
// generated by: thrift --gen rs osquery.thrift

pub fn open_thrift_socket(socket_path: &str)
    -> anyhow::Result<(
        TBinaryInputProtocol<TFramedReadTransport<ReadHalf<UnixStream>>>,
        TBinaryOutputProtocol<TFramedWriteTransport<WriteHalf<UnixStream>>>,
    )>
{
    let stream = UnixStream::connect(socket_path)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let (read_half, write_half) = stream.try_clone()
        .map(|s| (ReadHalf::new(UnixStream::from(s.into_raw_fd()
                .into_raw_fd())),
                WriteHalf::new(stream)))?;

    // NOTE: osquery uses FRAMED binary protocol
    let i_trans = TFramedReadTransport::new(ReadHalf::new(
        UnixStream::connect(socket_path)?
    ));
    let o_trans = TFramedWriteTransport::new(WriteHalf::new(
        UnixStream::connect(socket_path)?
    ));

    let i_prot = TBinaryInputProtocol::new(i_trans, true);
    let o_prot = TBinaryOutputProtocol::new(o_trans, true);

    Ok((i_prot, o_prot))
}
```

> **Important protocol detail**: osquery uses the **framed** binary Thrift
> transport, not the unbuffered transport. Each message is prefixed with a
> 4-byte big-endian length. If you use the wrong transport, you will get
> `TProtocolError::BadData` on every read. The `osquery-rust-ng` crate
> handles this correctly.

### Method C: Recommended — `osquery-rust-ng`

```toml
# Cargo.toml
osquery-rust-ng = "0.1"
```

```rust
use osquery_rust_ng::{OsqueryClient, ExtensionManagerClient};

pub async fn run_query(socket: &str, sql: &str)
    -> anyhow::Result<Vec<std::collections::HashMap<String, String>>>
{
    // Creates a framed binary Thrift client over UDS
    let mut client = OsqueryClient::new(socket)?;
    let response = client.query(sql)?;

    if response.status.code != 0 {
        anyhow::bail!("osquery error {}: {}", response.status.code, response.status.message);
    }

    Ok(response.response)
}
```

---

## 5. Writing Custom osquery Extensions in Rust

An osquery extension is a **separate binary** (`.ext` file) that:
1. Parses CLI args `--socket`, `--timeout`, `--interval` (passed by osqueryd)
2. Starts a Thrift server on a **new** Unix socket (not the manager socket)
3. Calls `ExtensionManager.registerExtension()` to advertise its tables
4. Serves `Extension.call()` RPCs when osqueryd queries the table

### Full Extension Binary with `osquery-rust-ng`

```rust
// crates/osquery-ext/src/main.rs
use osquery_rust_ng::prelude::*;
use std::collections::HashMap;

// ── Custom Table: hidden_processes ─────────────────────────────────────
// Detects processes in /proc that are not in the process list
// (basic rootkit detection heuristic)

struct HiddenProcessesTable;

impl TablePlugin for HiddenProcessesTable {
    fn name(&self) -> String {
        "hidden_processes".to_string()
    }

    fn columns(&self) -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("pid",        ColumnType::Integer),
            ColumnDef::new("comm",       ColumnType::Text),
            ColumnDef::new("state",      ColumnType::Text),
            ColumnDef::new("ppid",       ColumnType::Integer),
            ColumnDef::new("reason",     ColumnType::Text),
        ]
    }

    fn generate(&self, _context: QueryContext) -> Result<Rows, String> {
        // Read /proc/[0-9]* to find all PIDs
        let proc_pids = read_proc_pids().map_err(|e| e.to_string())?;

        // Read process list via /proc/[pid]/status
        let mut rows = Vec::new();
        for pid in proc_pids {
            if let Ok(info) = read_proc_status(pid) {
                // Heuristic: process has a /proc entry but no /proc/pid/exe link
                // This can indicate a process that unlinked its binary (fileless)
                let exe_exists = std::fs::read_link(
                    format!("/proc/{}/exe", pid)
                ).is_ok();

                if !exe_exists {
                    let mut row = HashMap::new();
                    row.insert("pid".to_string(), pid.to_string());
                    row.insert("comm".to_string(), info.comm);
                    row.insert("state".to_string(), info.state);
                    row.insert("ppid".to_string(), info.ppid.to_string());
                    row.insert("reason".to_string(),
                               "exe_link_missing".to_string());
                    rows.push(row);
                }
            }
        }
        Ok(rows)
    }
}

// ── Custom Table: kernel_thread_anomalies ──────────────────────────────
struct KernelThreadAnomaliesTable;

impl TablePlugin for KernelThreadAnomaliesTable {
    fn name(&self) -> String {
        "kernel_thread_anomalies".to_string()
    }

    fn columns(&self) -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("pid",           ColumnType::Integer),
            ColumnDef::new("name",          ColumnType::Text),
            ColumnDef::new("wchan",         ColumnType::Text),
            ColumnDef::new("anomaly_type",  ColumnType::Text),
        ]
    }

    fn generate(&self, _ctx: QueryContext) -> Result<Rows, String> {
        let mut rows = Vec::new();
        // Read /proc/[pid]/wchan — kernel wait channel
        // Threads waiting on unusual syscalls may indicate in-kernel implants
        for pid in read_proc_pids().unwrap_or_default() {
            if let Ok(wchan) = std::fs::read_to_string(
                format!("/proc/{}/wchan", pid)) {
                let wchan = wchan.trim().to_string();
                // Heuristic: kernel threads shouldn't be in schedule_hrtimeout
                // unless they have a timer. This catches timer-based implants.
                if wchan.contains("0") {
                    // wchan = 0 means process is running (not sleeping)
                    // Not necessarily anomalous
                    continue;
                }
                // Flag processes waiting in unusual kernel functions
                let is_anomalous = [
                    "sys_epoll_wait",
                    "futex_wait",
                    // Add your heuristics here
                ].iter().any(|&s| wchan == s);

                if !is_anomalous { continue; }

                let name = std::fs::read_to_string(
                    format!("/proc/{}/comm", pid))
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                let mut row = HashMap::new();
                row.insert("pid".to_string(), pid.to_string());
                row.insert("name".to_string(), name);
                row.insert("wchan".to_string(), wchan);
                row.insert("anomaly_type".to_string(), "unusual_wait".to_string());
                rows.push(row);
            }
        }
        Ok(rows)
    }
}

// ── Extension entry point ──────────────────────────────────────────────
#[osquery_rust_ng::args]
fn main() -> std::io::Result<()> {
    let args = Args::parse();   // --socket, --timeout, --interval parsed

    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Build extension with multiple tables
    let extension = ExtensionBuilder::new("edr_custom_tables")
        .version("1.0.0")
        .sdk_version("5.14.0")
        .min_sdk_version("5.0.0")
        .add_table(HiddenProcessesTable)
        .add_table(KernelThreadAnomaliesTable)
        .build();

    // Connect to osqueryd's extension manager socket and register
    extension.run(&args.socket, args.timeout, args.interval)?;

    Ok(())
}

// ── Helper functions ───────────────────────────────────────────────────
fn read_proc_pids() -> anyhow::Result<Vec<u32>> {
    let mut pids = Vec::new();
    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        if let Ok(name) = entry.file_name().into_string() {
            if let Ok(pid) = name.parse::<u32>() {
                pids.push(pid);
            }
        }
    }
    Ok(pids)
}

struct ProcStatus {
    comm: String,
    state: String,
    ppid: u32,
}

fn read_proc_status(pid: u32) -> anyhow::Result<ProcStatus> {
    let content = std::fs::read_to_string(
        format!("/proc/{}/status", pid))?;

    let mut comm = String::new();
    let mut state = String::new();
    let mut ppid = 0u32;

    for line in content.lines() {
        if let Some(v) = line.strip_prefix("Name:\t") { comm = v.to_string(); }
        if let Some(v) = line.strip_prefix("State:\t") { state = v.to_string(); }
        if let Some(v) = line.strip_prefix("PPid:\t") {
            ppid = v.trim().parse().unwrap_or(0);
        }
    }
    Ok(ProcStatus { comm, state, ppid })
}
```

### Deploy the Extension

```ini
# Add to /etc/osquery/extensions.load
/usr/lib/edr-agent/extensions/edr_custom_tables.ext
```

```bash
# Required permissions: root:root, 0700 (world-non-writable)
# osquery REFUSES to load extensions with unsafe permissions
install -o root -g root -m 0700 \
    ./target/release/edr_custom_tables \
    /usr/lib/edr-agent/extensions/edr_custom_tables.ext

# Verify in osqueryi:
osqueryi "SELECT * FROM hidden_processes;"
```

---

## 6. Custom Table Implementation

### Table Plugin — The `QueryContext` Constraint System

When osquery calls your table's `generate()`, it passes a `QueryContext`
containing **constraints** from the WHERE clause. You should respect these
to avoid full-table scans.

```rust
use osquery_rust_ng::prelude::*;
use std::collections::HashMap;

struct ProcessEnvsTable;

impl TablePlugin for ProcessEnvsTable {
    fn name(&self) -> String { "process_envs_extended".to_string() }

    fn columns(&self) -> Vec<ColumnDef> {
        vec![
            ColumnDef::new_with_options("pid",   ColumnType::Integer, ColumnOptions::INDEX),
            ColumnDef::new("key",   ColumnType::Text),
            ColumnDef::new("value", ColumnType::Text),
        ]
    }

    fn generate(&self, ctx: QueryContext) -> Result<Rows, String> {
        let mut rows = Vec::new();

        // Extract PID constraint if provided: WHERE pid = 1234
        // This is the key optimization: only scan requested PIDs
        let pids: Vec<u32> = if let Some(pid_constraints) = ctx.constraints.get("pid") {
            pid_constraints
                .iter()
                .filter_map(|c| {
                    if c.operator == ConstraintOperator::Equal {
                        c.expr.parse().ok()
                    } else { None }
                })
                .collect()
        } else {
            // No constraint: scan all PIDs (expensive on large systems)
            read_proc_pids().unwrap_or_default()
        };

        for pid in pids {
            // /proc/[pid]/environ is null-delimited KEY=VALUE pairs
            // Requires root for other users' processes
            let environ_path = format!("/proc/{}/environ", pid);
            match std::fs::read(&environ_path) {
                Ok(data) => {
                    for var in data.split(|&b| b == 0) {
                        if var.is_empty() { continue; }
                        let s = String::from_utf8_lossy(var);
                        if let Some(eq) = s.find('=') {
                            let mut row = HashMap::new();
                            row.insert("pid".to_string(), pid.to_string());
                            row.insert("key".to_string(),   s[..eq].to_string());
                            row.insert("value".to_string(), s[eq+1..].to_string());
                            rows.push(row);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    // Not root or process exited — skip silently
                }
                Err(_) => {}
            }
        }
        Ok(rows)
    }
}
```

### Writable Table (INSERT/UPDATE/DELETE)

`osquery-rust-ng` supports writable tables. Use case: agent config via SQL,
command dispatch from osquery queries.

```rust
struct AgentCommandTable {
    // Arc<Mutex<...>> for shared state between generate/insert/delete
    commands: std::sync::Arc<parking_lot::Mutex<Vec<AgentCommand>>>,
}

impl WritableTablePlugin for AgentCommandTable {
    fn name(&self) -> String { "agent_commands".to_string() }

    fn columns(&self) -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("id",      ColumnType::Integer),
            ColumnDef::new("command", ColumnType::Text),
            ColumnDef::new("status",  ColumnType::Text),
            ColumnDef::new("output",  ColumnType::Text),
        ]
    }

    fn generate(&self, _ctx: QueryContext) -> Result<Rows, String> {
        let cmds = self.commands.lock();
        Ok(cmds.iter().map(|c| {
            let mut row = HashMap::new();
            row.insert("id".to_string(),      c.id.to_string());
            row.insert("command".to_string(), c.command.clone());
            row.insert("status".to_string(),  c.status.clone());
            row.insert("output".to_string(),  c.output.clone());
            row
        }).collect())
    }

    // INSERT INTO agent_commands (command) VALUES ('isolate_network');
    fn insert(&self, row: &HashMap<String, String>) -> Result<(), String> {
        let command = row.get("command")
            .ok_or("missing command column")?
            .clone();

        let mut cmds = self.commands.lock();
        cmds.push(AgentCommand {
            id: cmds.len() as u64 + 1,
            command,
            status: "pending".to_string(),
            output: String::new(),
        });
        Ok(())
    }
}
```

---

## 7. Logger Plugin Implementation

A logger plugin intercepts all osquery result logs (query results, status logs)
and forwards them directly to your fleet server, bypassing the filesystem log.
This is the **most powerful integration point**: your agent receives telemetry
in real-time as osquery generates it.

```rust
// src/osquery/logger_plugin.rs
use osquery_rust_ng::plugin::{LoggerPlugin, LogStatus};
use tokio::sync::mpsc;

pub struct FleetLoggerPlugin {
    /// Send to agent's event pipeline
    tx: mpsc::Sender<OsqueryEvent>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct OsqueryEvent {
    pub raw_json: String,
    pub timestamp: i64,
    pub source: EventSource,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum EventSource {
    QueryResult,  // differential/snapshot result
    StatusLog,    // INFO/WARNING/ERROR from osquery itself
}

impl LoggerPlugin for FleetLoggerPlugin {
    fn name(&self) -> String {
        "fleet_logger".to_string()
    }

    /// Called for each query result row (when logger_event_type=true)
    fn log_string(&self, message: &str) -> Result<(), String> {
        let event = OsqueryEvent {
            raw_json: message.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            source: EventSource::QueryResult,
        };
        // Non-blocking send to agent pipeline
        self.tx.try_send(event).map_err(|e| e.to_string())
    }

    /// Called for osquery daemon status logs (INFO/WARNING/ERROR)
    fn log_status(&self, status: &LogStatus) -> Result<(), String> {
        let json = serde_json::json!({
            "severity": status.severity,
            "filename": status.filename,
            "line": status.line,
            "message": status.message,
        });
        let event = OsqueryEvent {
            raw_json: json.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            source: EventSource::StatusLog,
        };
        self.tx.try_send(event).map_err(|e| e.to_string())
    }
}
```

> **Critical deployment requirement**: To use a logger plugin, osqueryd must
> be started with `--logger_plugin=fleet_logger` (matching the name you return
> from `name()`). This flag must be in `osquery.flags` BEFORE osqueryd starts.
> The extension providing the logger plugin must be listed in `extensions_require`
> so osqueryd waits for it before processing events. This is the hardest part
> of the logger plugin integration.
>
> For most EDR deployments, **reading the filesystem log** (tail on
> `osqueryd.results.log`) is simpler and equally effective. Reserve logger
> plugins for zero-latency use cases.

---

## 8. gRPC Transport — Protobuf + Tonic

### proto/edr.proto

```protobuf
syntax = "proto3";
package edr.v1;

option java_package = "io.yourcompany.edr.v1";

// ── Enrollment ─────────────────────────────────────────────────────────
service EnrollService {
  // Agent calls this once at startup to register with fleet
  rpc Enroll(EnrollRequest) returns (EnrollResponse);
  // Renew expiring enrollment certificates
  rpc Renew(RenewRequest) returns (RenewResponse);
}

message EnrollRequest {
  string enrollment_secret = 1;   // pre-shared secret from config.toml
  string host_uuid         = 2;   // from system_info.uuid
  string hostname          = 3;
  string os_platform       = 4;   // "ubuntu", "rhel", "arch"
  string os_version        = 5;
  string arch              = 6;   // "x86_64", "aarch64"
  string agent_version     = 7;
  bytes  csr               = 8;   // Certificate Signing Request (DER)
}

message EnrollResponse {
  string node_key      = 1;   // fleet-assigned node identity key
  bytes  client_cert   = 2;   // signed cert for mTLS (DER)
  bytes  ca_cert       = 3;   // CA cert to verify fleet server
  bool   re_enroll     = 4;   // force re-enrollment if true
}

// ── Telemetry (event data) ─────────────────────────────────────────────
service TelemetryService {
  // Bidirectional stream: agent sends events, server sends acks
  rpc StreamEvents(stream EventBatch) returns (stream EventAck);
  // Snapshot queries (less time-sensitive)
  rpc SendSnapshot(SnapshotBatch) returns (SnapshotAck);
}

message EventBatch {
  string node_key      = 1;
  repeated Event events = 2;
  uint64 batch_id      = 3;
}

message Event {
  string  query_name    = 1;
  string  action        = 2;   // "added", "removed"
  int64   unix_time     = 3;
  string  host_uuid     = 4;
  map<string, string> columns = 5;
  map<string, string> decorations = 6;
}

message EventAck {
  uint64 batch_id       = 1;
  bool   accepted       = 2;
  string error_message  = 3;
}

message SnapshotBatch {
  string node_key          = 1;
  string query_name        = 2;
  int64  collected_at      = 3;
  repeated map<string, string> rows = 4;
}

message SnapshotAck {
  bool accepted = 1;
}

// ── Heartbeat & Metrics ────────────────────────────────────────────────
service HeartbeatService {
  rpc SendHeartbeat(Heartbeat) returns (HeartbeatAck);
}

message Heartbeat {
  string node_key       = 1;
  int64  timestamp      = 2;
  SystemMetrics metrics = 3;
  AgentStatus   status  = 4;
}

message SystemMetrics {
  float  cpu_percent           = 1;
  uint64 ram_used_bytes        = 2;
  uint64 ram_total_bytes       = 3;
  float  ram_percent           = 4;
  uint64 disk_read_bytes_sec   = 5;
  uint64 disk_write_bytes_sec  = 6;
  uint64 net_rx_bytes_sec      = 7;
  uint64 net_tx_bytes_sec      = 8;
  uint32 process_count         = 9;
  uint64 uptime_seconds        = 10;
}

message AgentStatus {
  string  agent_version      = 1;
  string  osquery_version    = 2;
  bool    osquery_healthy    = 3;
  uint64  events_queued      = 4;   // in local buffer
  uint64  events_sent_total  = 5;
  uint32  buffer_size_bytes  = 6;
  bool    isolated           = 7;   // nftables isolation active
}

message HeartbeatAck {
  bool   ok              = 1;
  string message         = 2;
  // Server can push immediate commands back in the heartbeat ack
  repeated RemoteCommand commands = 3;
}

// ── Commands (fleet → agent) ───────────────────────────────────────────
service CommandService {
  rpc SendCommand(RemoteCommand) returns (CommandResult);
  // Server streams commands to agent
  rpc WatchCommands(WatchRequest) returns (stream RemoteCommand);
}

message RemoteCommand {
  string command_id    = 1;
  CommandType type     = 2;
  string node_key      = 3;
  bytes  payload       = 4;   // command-specific payload
  int64  issued_at     = 5;
  int64  expires_at    = 6;
}

enum CommandType {
  COMMAND_TYPE_UNSPECIFIED    = 0;
  ISOLATE_NETWORK             = 1;
  DEISOLATE_NETWORK           = 2;
  UPDATE_CONFIG               = 3;
  RESTART_OSQUERY             = 4;
  COLLECT_SNAPSHOT            = 5;
  KILL_PROCESS                = 6;
}

message WatchRequest { string node_key = 1; }

message CommandResult {
  string command_id  = 1;
  bool   success     = 2;
  string output      = 3;
  int64  executed_at = 4;
}
```

### build.rs

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)
        .build_server(false)      // agent is the client, fleet server has server
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &["proto/edr.proto"],
            &["proto/"],
        )?;
    Ok(())
}
```

### gRPC Client Implementation

```rust
// src/transport/grpc.rs
use tonic::transport::{Channel, ClientTlsConfig, Certificate, Identity};
use tonic::metadata::MetadataValue;
use anyhow::Result;
use crate::proto::edr::v1::{
    telemetry_service_client::TelemetryServiceClient,
    heartbeat_service_client::HeartbeatServiceClient,
    EventBatch, Heartbeat,
};

pub struct FleetGrpcClient {
    channel: Channel,
    node_key: String,
    telemetry: TelemetryServiceClient<Channel>,
    heartbeat: HeartbeatServiceClient<Channel>,
}

impl FleetGrpcClient {
    pub async fn connect(config: &AgentConfig) -> Result<Self> {
        // mTLS: both client cert (from enrollment) and server CA
        let tls_config = ClientTlsConfig::new()
            .ca_certificate(Certificate::from_pem(&config.ca_cert_pem))
            .identity(Identity::from_pem(
                &config.client_cert_pem,
                &config.client_key_pem,
            ))
            .domain_name(&config.fleet_hostname);

        let channel = Channel::from_shared(config.fleet_url.clone())?
            .tls_config(tls_config)?
            .tcp_keepalive(Some(std::time::Duration::from_secs(30)))
            .http2_keep_alive_interval(std::time::Duration::from_secs(30))
            .keep_alive_while_idle(true)
            .connect()
            .await?;

        let telemetry = TelemetryServiceClient::new(channel.clone())
            // Enable gzip compression for large event batches
            .send_compressed(tonic::codec::CompressionEncoding::Gzip)
            .accept_compressed(tonic::codec::CompressionEncoding::Gzip);

        let heartbeat = HeartbeatServiceClient::new(channel.clone());

        Ok(Self {
            channel,
            node_key: config.node_key.clone(),
            telemetry,
            heartbeat,
        })
    }

    /// Send a batch of events. Returns whether the server accepted them.
    pub async fn send_events(&mut self, batch: EventBatch) -> Result<bool> {
        // Add node_key as gRPC metadata header for server-side filtering
        let mut request = tonic::Request::new(
            tokio_stream::once(batch)
        );
        request.metadata_mut().insert(
            "x-node-key",
            MetadataValue::try_from(&self.node_key)?,
        );

        let mut stream = self.telemetry.stream_events(request).await?
            .into_inner();

        if let Some(ack) = stream.message().await? {
            return Ok(ack.accepted);
        }
        Ok(false)
    }

    pub async fn send_heartbeat(&mut self, hb: Heartbeat) -> Result<()> {
        self.heartbeat.send_heartbeat(hb).await?;
        Ok(())
    }
}
```

---

## 9. Tokio Agent Core — All Tasks & Channels

```rust
// src/agent.rs
use tokio::sync::mpsc;
use std::sync::Arc;
use parking_lot::RwLock;

/// Shared agent state — accessed from all tasks
pub struct AgentState {
    pub config:       Arc<RwLock<AgentConfig>>,
    pub node_key:     Arc<RwLock<String>>,
    pub is_isolated:  Arc<std::sync::atomic::AtomicBool>,
    pub events_queued: Arc<std::sync::atomic::AtomicU64>,
}

/// Main agent entry point — spawns all background tasks
pub async fn run(config: AgentConfig) -> anyhow::Result<()> {
    // ── Channels ──────────────────────────────────────────────────────
    // Event pipeline: osquery results → local buffer → gRPC uplink
    let (event_tx, event_rx) = mpsc::channel::<OsqueryEvent>(10_000);
    // Command pipeline: fleet server → command executor
    let (cmd_tx, cmd_rx) = mpsc::channel::<RemoteCommand>(100);

    let state = Arc::new(AgentState {
        config:       Arc::new(RwLock::new(config.clone())),
        node_key:     Arc::new(RwLock::new(config.node_key.clone())),
        is_isolated:  Arc::new(std::sync::atomic::AtomicBool::new(false)),
        events_queued: Arc::new(std::sync::atomic::AtomicU64::new(0)),
    });

    // ── Task 1: osquery log tail → event pipeline ─────────────────────
    // Reads osqueryd.results.log as it grows (like `tail -f`)
    // OR uses the logger plugin (if configured)
    let s1 = Arc::clone(&state);
    let e_tx = event_tx.clone();
    let task_log_tail = tokio::spawn(async move {
        log_tail_task(s1, e_tx).await
    });

    // ── Task 2: Local buffer writer ───────────────────────────────────
    // Drains event_rx → writes to redb (durable buffer)
    let s2 = Arc::clone(&state);
    let task_buffer = tokio::spawn(async move {
        buffer_writer_task(s2, event_rx).await
    });

    // ── Task 3: gRPC uplink ────────────────────────────────────────────
    // Reads from redb buffer → ships to fleet server → marks as acked
    let s3 = Arc::clone(&state);
    let task_grpc = tokio::spawn(async move {
        grpc_uplink_task(s3).await
    });

    // ── Task 4: Heartbeat sender ────────────────────────────────────────
    let s4 = Arc::clone(&state);
    let task_heartbeat = tokio::spawn(async move {
        heartbeat_task(s4).await
    });

    // ── Task 5: Config hot-reload watcher ─────────────────────────────
    let s5 = Arc::clone(&state);
    let task_config = tokio::spawn(async move {
        config_watcher_task(s5).await
    });

    // ── Task 6: Command executor ────────────────────────────────────────
    let s6 = Arc::clone(&state);
    let task_commands = tokio::spawn(async move {
        command_executor_task(s6, cmd_rx).await
    });

    // ── Task 7: Extension watchdog ─────────────────────────────────────
    // Ensures the .ext process is running; restarts if it exits
    let s7 = Arc::clone(&state);
    let task_ext_watch = tokio::spawn(async move {
        extension_watchdog_task(s7).await
    });

    // ── Systemd READY notification ────────────────────────────────────
    sd_notify::notify(false, &[sd_notify::NotifyState::Ready])?;
    tracing::info!("EDR agent started, all tasks running");

    // ── Graceful shutdown on SIGTERM / SIGINT ─────────────────────────
    let mut sigterm = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::interrupt())?;

    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("SIGTERM received, initiating graceful shutdown");
        }
        _ = sigint.recv() => {
            tracing::info!("SIGINT received");
        }
    }

    // Notify systemd we are stopping
    sd_notify::notify(false, &[sd_notify::NotifyState::Stopping])?;

    // Abort all tasks
    task_log_tail.abort();
    task_buffer.abort();
    task_grpc.abort();
    task_heartbeat.abort();
    task_config.abort();
    task_commands.abort();
    task_ext_watch.abort();

    tracing::info!("EDR agent stopped cleanly");
    Ok(())
}
```

### Systemd Watchdog Integration

```rust
// src/watchdog.rs — runs inside the heartbeat task
use sd_notify::NotifyState;
use tokio::time::{interval, Duration};

pub async fn systemd_watchdog_task() {
    // Read WATCHDOG_USEC from environment (set by systemd)
    let watchdog_usec: u64 = std::env::var("WATCHDOG_USEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    if watchdog_usec == 0 {
        // Watchdog not configured in systemd unit; skip
        return;
    }

    // Ping every half the watchdog timeout (systemd recommendation)
    let period = Duration::from_micros(watchdog_usec / 2);
    let mut ticker = interval(period);

    loop {
        ticker.tick().await;
        // WATCHDOG=1 tells systemd we are alive
        if let Err(e) = sd_notify::notify(false, &[NotifyState::Watchdog]) {
            tracing::warn!("Failed to notify systemd watchdog: {}", e);
        }
    }
}
```

---

## 10. Local Event Buffering — redb vs sled vs SQLite

### Why You Need a Local Buffer

When the fleet server is unreachable (network partition, server restart,
agent restart), events must not be lost. A **write-ahead buffer** persists
events to disk first, ships to fleet, then marks as acknowledged.

### Database Comparison for This Use Case

| | `redb` | `sled` | `rusqlite` | RocksDB |
|---|---|---|---|---|
| Pure Rust | ✅ | ✅ | ❌ (C SQLite) | ❌ (C++ RocksDB) |
| Production stable | ✅ (v2 2024) | ⚠️ (rewrite) | ✅ | ✅ |
| No C deps | ✅ | ✅ | ❌ | ❌ |
| ACID | ✅ | ✅ | ✅ | ❌ |
| Key ordering | ✅ | ✅ | ✅ | ✅ |
| Write throughput | High | Medium | Medium | Very High |
| Binary size impact | +500 KB | +400 KB | +600 KB | +5 MB |
| **Recommendation** | **✅ Use this** | ⚠️ Avoid | OK | Avoid (conflicts with osquery) |

### redb Buffer Implementation

```rust
// src/transport/buffer.rs
use redb::{Database, TableDefinition, ReadableTable};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::path::Path;

// Table: event_id (u64) → serialized event JSON (String)
const PENDING_EVENTS: TableDefinition<u64, &str> = TableDefinition::new("pending_events");
// Table: batch_id (u64) → batch metadata (for ack tracking)
const BATCHES: TableDefinition<u64, &str> = TableDefinition::new("batches");

pub struct EventBuffer {
    db: Database,
    next_id: u64,
}

impl EventBuffer {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Database::create(path)?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        write_txn.open_table(PENDING_EVENTS)?;
        write_txn.open_table(BATCHES)?;
        write_txn.commit()?;

        // Find next available event ID
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(PENDING_EVENTS)?;
        let next_id = table
            .iter()?
            .next_back()
            .and_then(|r| r.ok())
            .map(|(k, _)| k.value() + 1)
            .unwrap_or(0);

        Ok(Self { db, next_id })
    }

    /// Write events to disk atomically
    pub fn write_batch(&mut self, events: &[OsqueryEvent]) -> Result<u64> {
        let write_txn = self.db.begin_write()?;
        let mut table = write_txn.open_table(PENDING_EVENTS)?;

        let batch_start = self.next_id;
        for event in events {
            let json = serde_json::to_string(event)?;
            table.insert(self.next_id, json.as_str())?;
            self.next_id += 1;
        }

        // Record batch metadata
        let mut batch_table = write_txn.open_table(BATCHES)?;
        let batch_meta = serde_json::json!({
            "start": batch_start,
            "end": self.next_id,
            "count": events.len(),
            "created_at": chrono::Utc::now().timestamp(),
        });
        batch_table.insert(batch_start, batch_meta.to_string().as_str())?;

        write_txn.commit()?;
        Ok(batch_start)
    }

    /// Read up to `limit` unacked events for shipping
    pub fn read_pending(&self, limit: usize) -> Result<Vec<(u64, OsqueryEvent)>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PENDING_EVENTS)?;

        let mut events = Vec::with_capacity(limit);
        for entry in table.iter()?.take(limit) {
            let (k, v) = entry?;
            let event: OsqueryEvent = serde_json::from_str(v.value())?;
            events.push((k.value(), event));
        }
        Ok(events)
    }

    /// Acknowledge events (mark as sent successfully)
    /// Called after fleet server confirms receipt
    pub fn acknowledge(&mut self, up_to_id: u64) -> Result<u64> {
        let write_txn = self.db.begin_write()?;
        let mut table = write_txn.open_table(PENDING_EVENTS)?;

        let mut deleted = 0u64;
        // Delete all events with id <= up_to_id
        let keys_to_delete: Vec<u64> = table
            .iter()?
            .take_while(|r| {
                r.as_ref().map(|(k, _)| k.value() <= up_to_id).unwrap_or(false)
            })
            .filter_map(|r| r.ok())
            .map(|(k, _)| k.value())
            .collect();

        for key in keys_to_delete {
            table.remove(key)?;
            deleted += 1;
        }

        write_txn.commit()?;
        Ok(deleted)
    }

    /// Return count of unacked events
    pub fn pending_count(&self) -> Result<usize> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PENDING_EVENTS)?;
        Ok(table.len()? as usize)
    }

    /// Emergency flush if buffer exceeds max size (drop oldest events)
    pub fn enforce_max_size(&mut self, max_events: u64) -> Result<u64> {
        let current = self.pending_count()? as u64;
        if current <= max_events { return Ok(0); }

        let to_drop = current - max_events;
        let write_txn = self.db.begin_write()?;
        let mut table = write_txn.open_table(PENDING_EVENTS)?;

        let keys: Vec<u64> = table.iter()?
            .take(to_drop as usize)
            .filter_map(|r| r.ok())
            .map(|(k, _)| k.value())
            .collect();

        for key in &keys { table.remove(*key)?; }
        write_txn.commit()?;

        tracing::warn!(
            "Buffer overflow: dropped {} oldest events (max={})",
            keys.len(), max_events
        );
        Ok(keys.len() as u64)
    }
}
```

---

## 11. Distro Detection & osquery Auto-Install

### Why Not Embed osquery in Your Binary

osqueryd is a ~70 MB binary. Bundling it would make every agent update
ship 70 MB. Instead, the installer detects the distro and downloads the
appropriate osquery package from `pkg.osquery.io`. This is what Fleet,
Kolide, and Uptycs do.

### Distro Detection

```rust
// src/osquery/installer.rs
use anyhow::{bail, Result};
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum Distro {
    Debian,
    Ubuntu,
    RaspberryPiOS,
    RHEL,
    CentOS,
    Rocky,
    AlmaLinux,
    Fedora,
    Arch,
    Manjaro,
    Alpine,
    Amazon,
    SUSE,
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub distro:   Distro,
    pub arch:     Arch,
    pub pkg_mgr:  PkgManager,
    pub init:     Init,
    pub kernel:   semver::Version,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arch {
    X86_64,
    Aarch64,
    Armv7,   // 32-bit ARM (Raspberry Pi 2/3)
}

#[derive(Debug, Clone)]
pub enum PkgManager { Apt, Yum, Dnf, Pacman, Apk }

#[derive(Debug, Clone)]
pub enum Init { Systemd, OpenRC, SysV }

pub fn detect_system() -> Result<SystemInfo> {
    // ── Architecture ─────────────────────────────────────────────────
    let arch = match std::env::consts::ARCH {
        "x86_64"  => Arch::X86_64,
        "aarch64" => Arch::Aarch64,
        "arm"     => Arch::Armv7,
        a         => bail!("Unsupported architecture: {}", a),
    };

    // ── Kernel version ────────────────────────────────────────────────
    let uname = std::fs::read_to_string("/proc/version")?;
    let kernel_str = uname.split_whitespace()
        .nth(2)
        .unwrap_or("5.0.0");
    // Strip suffix like "5.15.0-91-generic" → "5.15.0"
    let kernel_clean: String = kernel_str.chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let kernel = semver::Version::parse(&kernel_clean)
        .unwrap_or_else(|_| semver::Version::new(5, 0, 0));

    // ── Distro from /etc/os-release ───────────────────────────────────
    let os_release = std::fs::read_to_string("/etc/os-release")
        .unwrap_or_default();

    let id = extract_os_release_field(&os_release, "ID")
        .to_lowercase();
    let id_like = extract_os_release_field(&os_release, "ID_LIKE")
        .to_lowercase();

    let distro = match id.as_str() {
        "ubuntu"    => Distro::Ubuntu,
        "debian"    => Distro::Debian,
        "raspbian"  => Distro::RaspberryPiOS,
        "rhel"      => Distro::RHEL,
        "centos"    => Distro::CentOS,
        "rocky"     => Distro::Rocky,
        "almalinux" => Distro::AlmaLinux,
        "fedora"    => Distro::Fedora,
        "arch"      => Distro::Arch,
        "manjaro"   => Distro::Manjaro,
        "alpine"    => Distro::Alpine,
        "amzn"      => Distro::Amazon,
        "opensuse-leap" | "opensuse-tumbleweed" | "sles" => Distro::SUSE,
        _ if id_like.contains("debian") || id_like.contains("ubuntu") => Distro::Debian,
        _ if id_like.contains("rhel")  || id_like.contains("fedora") => Distro::RHEL,
        _ => Distro::Unknown(id),
    };

    // ── Package manager ────────────────────────────────────────────────
    let pkg_mgr = if which::which("apt-get").is_ok() {
        PkgManager::Apt
    } else if which::which("dnf").is_ok() {
        PkgManager::Dnf
    } else if which::which("yum").is_ok() {
        PkgManager::Yum
    } else if which::which("pacman").is_ok() {
        PkgManager::Pacman
    } else if which::which("apk").is_ok() {
        PkgManager::Apk
    } else {
        bail!("No recognized package manager found")
    };

    // ── Init system ────────────────────────────────────────────────────
    let init = if std::path::Path::new("/run/systemd/system").exists() {
        Init::Systemd
    } else if std::path::Path::new("/run/openrc").exists() {
        Init::OpenRC
    } else {
        Init::SysV
    };

    Ok(SystemInfo { distro, arch, pkg_mgr, init, kernel })
}

fn extract_os_release_field(content: &str, field: &str) -> String {
    for line in content.lines() {
        if let Some(val) = line.strip_prefix(&format!("{}=", field)) {
            return val.trim_matches('"').to_string();
        }
    }
    String::new()
}
```

### Per-Distro osquery Installer

```rust
pub async fn install_osquery(info: &SystemInfo) -> Result<()> {
    if which::which("osqueryd").is_ok() {
        tracing::info!("osquery already installed, skipping");
        return Ok(());
    }

    tracing::info!("Installing osquery for {:?} {:?}", info.distro, info.arch);

    match info.pkg_mgr {
        // ── Debian / Ubuntu ────────────────────────────────────────────
        PkgManager::Apt => {
            let gpg_key_url = "https://pkg.osquery.io/deb/pubkey.gpg";
            let keyring_path = "/usr/share/keyrings/osquery-keyring.gpg";
            let repo_line = "deb [signed-by=/usr/share/keyrings/osquery-keyring.gpg] \
                             https://pkg.osquery.io/deb deb main";

            // Download and install GPG key
            let key_bytes = reqwest::get(gpg_key_url).await?.bytes().await?;
            // Dearmor if necessary (GPG ASCII armor → binary)
            std::fs::write(keyring_path, &key_bytes)?;

            // Add repository
            std::fs::write(
                "/etc/apt/sources.list.d/osquery.list",
                repo_line
            )?;

            // Install
            run_cmd("apt-get", &["update", "-qq"])?;
            run_cmd("apt-get", &["install", "-y", "-qq", "osquery"])?;
        }

        // ── RHEL / CentOS / Rocky / Fedora ────────────────────────────
        PkgManager::Yum | PkgManager::Dnf => {
            let repo_content = r#"[osquery-s3-rpm-release]
name=osquery-s3-rpm-release
baseurl=https://pkg.osquery.io/rpm
enabled=1
repo_gpgcheck=1
gpgcheck=0
gpgkey=https://pkg.osquery.io/rpm/GPG
"#;
            std::fs::write("/etc/yum.repos.d/osquery.repo", repo_content)?;

            let cmd = if matches!(info.pkg_mgr, PkgManager::Dnf) {
                "dnf"
            } else {
                "yum"
            };
            run_cmd(cmd, &["install", "-y", "osquery"])?;
        }

        // ── Arch Linux / Manjaro ───────────────────────────────────────
        PkgManager::Pacman => {
            // osquery is in AUR; use pre-built binary from GitHub releases
            // or have the user install from AUR
            download_osquery_binary(info).await?;
        }

        // ── Alpine Linux ───────────────────────────────────────────────
        PkgManager::Apk => {
            // osquery is not in Alpine repos; download static binary
            download_osquery_binary(info).await?;
        }
    }

    // ── Post-install setup ─────────────────────────────────────────────
    stop_and_disable_auditd()?;
    create_osquery_directories()?;
    mask_journald_audit_socket()?;
    set_kernel_tunables()?;

    tracing::info!("osquery installed successfully");
    Ok(())
}

async fn download_osquery_binary(info: &SystemInfo) -> Result<()> {
    // Fetch latest release from GitHub API
    let releases_url = "https://api.github.com/repos/osquery/osquery/releases/latest";
    let client = reqwest::Client::builder()
        .user_agent("edr-agent/1.0")
        .build()?;

    let release: serde_json::Value = client.get(releases_url)
        .send().await?
        .json().await?;

    let arch_str = match info.arch {
        Arch::X86_64  => "linux_amd64",
        Arch::Aarch64 => "linux_aarch64",
        Arch::Armv7   => "linux_armhf",
    };

    // Find the .tar.gz asset for this arch
    let asset_url = release["assets"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No assets in release"))?
        .iter()
        .find(|a| {
            a["name"].as_str().map(|n| n.contains(arch_str) && n.ends_with(".tar.gz"))
                .unwrap_or(false)
        })
        .and_then(|a| a["browser_download_url"].as_str())
        .ok_or_else(|| anyhow::anyhow!("No binary for arch: {}", arch_str))?
        .to_string();

    tracing::info!("Downloading osquery from: {}", asset_url);

    // Download with progress
    let response = client.get(&asset_url).send().await?;
    let bytes = response.bytes().await?;

    // Extract to /opt/osquery/
    std::fs::create_dir_all("/opt/osquery/bin")?;
    // ... tar extraction logic ...

    // Symlink
    let _ = std::fs::remove_file("/usr/local/bin/osqueryd");
    std::os::unix::fs::symlink("/opt/osquery/bin/osqueryd", "/usr/local/bin/osqueryd")?;

    Ok(())
}

fn stop_and_disable_auditd() -> Result<()> {
    // auditd conflicts with osquery's audit netlink socket
    let _ = Command::new("systemctl").args(["stop",    "auditd"]).output();
    let _ = Command::new("systemctl").args(["disable", "auditd"]).output();
    let _ = Command::new("systemctl").args(["mask",    "auditd"]).output();
    Ok(())
}

fn mask_journald_audit_socket() -> Result<()> {
    // journald also consumes audit events; mask it to avoid splits
    let _ = Command::new("systemctl")
        .args(["mask", "--now", "systemd-journald-audit.socket"])
        .output();
    Ok(())
}

fn set_kernel_tunables() -> Result<()> {
    let tunables = [
        ("fs.inotify.max_user_watches", "524288"),
        ("fs.inotify.max_user_instances", "256"),
        ("fs.inotify.max_queued_events", "32768"),
        ("vm.overcommit_memory", "1"),
    ];

    let sysctl_conf = tunables
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write("/etc/sysctl.d/60-edr-agent.conf", sysctl_conf)?;
    Command::new("sysctl").arg("-p")
        .arg("/etc/sysctl.d/60-edr-agent.conf")
        .output()?;
    Ok(())
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd).args(args).status()?;
    if !status.success() {
        bail!("{} {} failed with status: {}", cmd, args.join(" "), status);
    }
    Ok(())
}
```

---

## 12. Packaging & Delivery Strategy

### What NOT to Deliver

- ❌ AppImage — overly large, not suited for server-side EDR
- ❌ Snap/Flatpak — sandboxed, conflicts with kernel-level access
- ❌ Docker container — you lose access to host namespaces

### What to Deliver

```
Recommended delivery artifacts (from GitHub Releases):

edr-agent-linux-amd64.tar.gz     (~8 MB, musl-linked static binary)
edr-agent-linux-aarch64.tar.gz   (~8 MB, musl-linked static binary)
install.sh                        (downloads correct binary, installs)
```

**Why static musl binary?**
- Zero runtime dependencies (no `libc.so`, no `libssl.so`)
- Works on ANY Linux distro: Alpine (musl native), Ubuntu, RHEL, Arch
- Same binary runs on kernel 3.10 through 6.x
- Distribution is a single file — `scp` it and run it

### Install Script: `install/install.sh`

```bash
#!/usr/bin/env bash
# EDR Agent Installer
# Usage: curl -fsSL https://your.server/install.sh | sudo bash -s -- --secret YOUR_SECRET
# Or: sudo bash install.sh --secret YOUR_SECRET --fleet-url grpcs://fleet.example.com:8443

set -euo pipefail

FLEET_URL="${EDR_FLEET_URL:-}"
ENROLL_SECRET="${EDR_ENROLL_SECRET:-}"
AGENT_VERSION="${EDR_AGENT_VERSION:-latest}"
INSTALL_DIR="/opt/edr-agent"
CONFIG_DIR="/etc/edr-agent"
LOG_DIR="/var/log/edr-agent"
DATA_DIR="/var/lib/edr-agent"

# ── Parse arguments ────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --secret)     ENROLL_SECRET="$2"; shift 2 ;;
        --fleet-url)  FLEET_URL="$2";     shift 2 ;;
        --version)    AGENT_VERSION="$2"; shift 2 ;;
        *) echo "Unknown argument: $1"; exit 1 ;;
    esac
done

[[ -z "$ENROLL_SECRET" ]] && { echo "ERROR: --secret is required"; exit 1; }
[[ -z "$FLEET_URL"     ]] && { echo "ERROR: --fleet-url is required"; exit 1; }

# ── Detect architecture ────────────────────────────────────────────────
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  ARCH_TAG="amd64"   ;;
    aarch64) ARCH_TAG="aarch64" ;;
    armv7*)  ARCH_TAG="armhf"   ;;
    *)       echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

echo "==> Detected arch: $ARCH_TAG"

# ── Download agent binary ──────────────────────────────────────────────
DOWNLOAD_URL="https://github.com/your-org/edr-agent/releases/download/${AGENT_VERSION}/edr-agent-linux-${ARCH_TAG}.tar.gz"
CHECKSUM_URL="${DOWNLOAD_URL}.sha256"

echo "==> Downloading edr-agent ${AGENT_VERSION}..."
curl -fsSL "$DOWNLOAD_URL" -o /tmp/edr-agent.tar.gz
curl -fsSL "$CHECKSUM_URL" -o /tmp/edr-agent.tar.gz.sha256

# Verify checksum
echo "==> Verifying checksum..."
(cd /tmp && sha256sum -c edr-agent.tar.gz.sha256)

# ── Install ────────────────────────────────────────────────────────────
mkdir -p "$INSTALL_DIR" "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
mkdir -p /usr/lib/edr-agent/extensions

tar -xzf /tmp/edr-agent.tar.gz -C /tmp/
install -o root -g root -m 0755 /tmp/edr-agent "$INSTALL_DIR/edr-agent"

# ── Write config.toml ──────────────────────────────────────────────────
cat > "$CONFIG_DIR/config.toml" << EOF
[fleet]
url            = "$FLEET_URL"
enroll_secret  = "$ENROLL_SECRET"

[osquery]
socket_path    = "/var/osquery/osquery.em"
flags_path     = "/etc/osquery/osquery.flags"
conf_path      = "/etc/osquery/osquery.conf"
log_path       = "/var/log/osquery/osqueryd.results.log"
auto_install   = true

[buffer]
path           = "$DATA_DIR/events.redb"
max_events     = 500000
max_size_mb    = 512

[agent]
log_level      = "info"
log_file       = "$LOG_DIR/agent.log"
data_dir       = "$DATA_DIR"
heartbeat_interval_secs = 30
metrics_interval_secs   = 10
EOF

chmod 600 "$CONFIG_DIR/config.toml"
echo "==> Config written to $CONFIG_DIR/config.toml"

# ── Install systemd service ────────────────────────────────────────────
cat > /etc/systemd/system/edr-agent.service << 'UNIT'
[Unit]
Description=EDR Agent
Documentation=https://github.com/your-org/edr-agent
After=network-online.target
Wants=network-online.target
Conflicts=auditd.service

[Service]
Type=notify
User=root
Group=root
ExecStart=/opt/edr-agent/edr-agent --config /etc/edr-agent/config.toml
Restart=always
RestartSec=5
StartLimitBurst=10
StartLimitInterval=60s
WatchdogSec=60s
KillMode=process
TimeoutStopSec=30
LimitNOFILE=65536
SyslogIdentifier=edr-agent

[Install]
WantedBy=multi-user.target
UNIT

systemctl daemon-reload
systemctl enable --now edr-agent

# ── Let the agent's own installer handle osquery ───────────────────────
# The agent binary detects distro and installs osquery on first run
echo ""
echo "======================================"
echo " EDR Agent installed and started."
echo " Check status: systemctl status edr-agent"
echo " View logs:    journalctl -u edr-agent -f"
echo "======================================"
```

### Fleet-Wide Deployment (no manual SSH per node)

For deploying to hundreds of nodes, use configuration management:

**Ansible playbook:**
```yaml
# deploy-agent.yml
---
- name: Deploy EDR Agent
  hosts: all
  become: yes
  vars:
    agent_version: "v1.2.3"
    fleet_url:     "grpcs://fleet.example.com:8443"

  tasks:
    - name: Run EDR agent installer
      shell: |
        curl -fsSL https://your.server/install.sh | bash -s -- \
          --secret "{{ lookup('env', 'EDR_ENROLL_SECRET') }}" \
          --fleet-url "{{ fleet_url }}" \
          --version "{{ agent_version }}"
      args:
        creates: /opt/edr-agent/edr-agent

    - name: Ensure agent is running
      systemd:
        name: edr-agent
        state: started
        enabled: yes
```

---

## 13. Fleet-Based Config Deployment — config.toml

Each node gets a `config.toml`. The enrollment secret and fleet URL are the
only values that differ between nodes. Everything else can be fleet-default.

```toml
# /etc/edr-agent/config.toml
# This file is managed by EDR Agent. Do not edit manually.
# Changes made by the fleet server are applied on next reload.

[fleet]
url                   = "grpcs://fleet.example.com:8443"
enroll_secret         = "your-32-char-enrollment-secret"
node_key              = ""    # filled by agent after enrollment
heartbeat_interval_s  = 30
command_poll_interval_s = 5

[tls]
# After enrollment, agent writes its certificate here
client_cert_path = "/var/lib/edr-agent/agent.crt"
client_key_path  = "/var/lib/edr-agent/agent.key"
ca_cert_path     = "/var/lib/edr-agent/fleet-ca.crt"
# TLS verification: "strict" | "skip" (never skip in production)
verify           = "strict"

[osquery]
socket_path      = "/var/osquery/osquery.em"
flags_path       = "/etc/osquery/osquery.flags"
conf_path        = "/etc/osquery/osquery.conf"
log_path         = "/var/log/osquery/osqueryd.results.log"
log_poll_ms      = 100          # how often to poll the log file for new lines
auto_install     = true         # agent installs osquery if missing
auto_configure   = true         # agent writes flags + conf files

[buffer]
path             = "/var/lib/edr-agent/events.redb"
max_events       = 500_000      # max events in local buffer
max_size_mb      = 512
flush_interval_s = 5            # drain buffer to fleet every N seconds
batch_size       = 500          # events per gRPC batch

[agent]
log_level        = "info"       # trace | debug | info | warn | error
log_file         = "/var/log/edr-agent/agent.log"
log_max_size_mb  = 100
log_max_files    = 10
data_dir         = "/var/lib/edr-agent"
extensions_dir   = "/usr/lib/edr-agent/extensions"

[metrics]
# Metrics sent to fleet in each heartbeat
cpu_interval_s   = 10
disk_interval_s  = 30
include_processes = true

[isolation]
# nftables isolation: only enabled when fleet server sends ISOLATE command
fleet_server_ip  = "10.0.0.1"
fleet_port       = 8443
agent_port       = 8444         # incoming port agent listens on (if any)
```

### Config Hot-Reload

```rust
// src/config.rs
use notify::{Watcher, RecursiveMode, Event};
use tokio::sync::watch;
use std::path::PathBuf;

pub async fn config_watcher_task(
    config_path: PathBuf,
    tx: watch::Sender<AgentConfig>,
) -> anyhow::Result<()> {
    let (inotify_tx, mut inotify_rx) = tokio::sync::mpsc::channel(8);

    // Use notify crate (wraps inotify on Linux)
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(event) = res {
            if event.kind.is_modify() || event.kind.is_create() {
                let _ = inotify_tx.blocking_send(());
            }
        }
    })?;
    watcher.watch(&config_path, RecursiveMode::NonRecursive)?;

    loop {
        // Wait for file change notification
        inotify_rx.recv().await;
        // Debounce: wait 500ms for writes to settle
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        // Drain any extra notifications
        while inotify_rx.try_recv().is_ok() {}

        match load_config(&config_path) {
            Ok(new_config) => {
                tracing::info!("Config reloaded from {:?}", config_path);
                let _ = tx.send(new_config);
            }
            Err(e) => {
                tracing::error!("Config reload failed (keeping old config): {}", e);
            }
        }
    }
}

pub fn load_config(path: &std::path::Path) -> anyhow::Result<AgentConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: AgentConfig = toml::from_str(&content)?;
    config.validate()?;
    Ok(config)
}
```

---

## 14. Systemd Service — Full Setup

```ini
# /etc/systemd/system/edr-agent.service
[Unit]
Description=EDR Agent (osquery-based endpoint detection)
Documentation=https://github.com/your-org/edr-agent
# Start after network is online AND osqueryd is running
After=network-online.target osqueryd.service
Wants=network-online.target
Requires=osqueryd.service
# Conflict with auditd — osquery owns the audit socket
Conflicts=auditd.service

[Service]
Type=notify
# MUST run as root: kernel audit, BPF, /proc/<pid>/environ, shadow
User=root
Group=root
WorkingDirectory=/opt/edr-agent

ExecStart=/opt/edr-agent/edr-agent \
    --config /etc/edr-agent/config.toml

# Reload config on SIGHUP (no restart needed for config changes)
ExecReload=/bin/kill -HUP $MAINPID

# ── Restart Policy ─────────────────────────────────────────────────────
# Restart on ANY failure (including OOM kill)
Restart=always
RestartSec=5s
# Allow 10 restarts in 60 seconds before giving up
StartLimitBurst=10
StartLimitInterval=60s

# ── Watchdog ────────────────────────────────────────────────────────────
# systemd kills the process if it doesn't ping within WatchdogSec
# Agent must call sd_notify(WATCHDOG=1) every ~30s
WatchdogSec=60s
# After watchdog timeout, SIGABRT (generates core dump) then SIGKILL
WatchdogSignal=SIGABRT

# ── Resource Limits ─────────────────────────────────────────────────────
LimitNOFILE=65536
LimitNPROC=4096
# Allow core dumps for debugging crashes
LimitCORE=infinity

# ── Process Group ────────────────────────────────────────────────────────
# Kill the entire process group on stop (catches orphaned child processes)
KillMode=control-group
TimeoutStopSec=30

# ── Logging ─────────────────────────────────────────────────────────────
SyslogIdentifier=edr-agent
StandardOutput=journal
StandardError=journal

# ── Environment ─────────────────────────────────────────────────────────
# Load secrets from file instead of inline (more secure)
EnvironmentFile=-/etc/edr-agent/environment
Environment="RUST_LOG=info"
Environment="RUST_BACKTRACE=1"

[Install]
WantedBy=multi-user.target
```

```ini
# /etc/systemd/system/edr-agent.service.d/override.conf
# Overrides for production environments (higher resource limits)
[Service]
LimitNOFILE=524288
CPUQuota=30%        # don't starve other services
MemoryMax=512M      # OOM protection
```

### What Happens If the Service Is Killed?

- **SIGTERM (graceful)**: systemd sends SIGTERM, agent gets 30s to flush buffer,
  close gRPC stream, call `sd_notify(STOPPING=1)`. Then SIGKILL.
- **OOM kill**: kernel sends SIGKILL directly. `Restart=always` triggers.
  redb WAL ensures no data loss (transactions were already committed).
- **`kill -9 <pid>`**: same as OOM kill. Restart=always handles it.
- **systemctl kill edr-agent**: same.
- **Kernel panic / hard reboot**: redb survives because it uses `fsync` after
  each committed transaction. Events not yet `commit()`-ed are lost (< 1 batch,
  configurable).

The key insight: **the buffer is your durability guarantee**. Any event that
was written to `redb` before the crash will be retried after restart.

---

## 15. Startup Sequence & Boot Ordering

```
Boot sequence:
─────────────────────────────────────────────────────────────────────
1. kernel init
2. systemd starts
3. network-online.target reached
4. osqueryd.service starts
   └─ osquery loads flags + conf
   └─ osquery opens audit netlink socket
   └─ osquery starts ExtensionManager on /var/osquery/osquery.em
5. edr-agent.service starts (Requires=osqueryd.service)
   └─ agent loads config.toml
   └─ agent checks if osqueryd socket is ready (retry loop)
   └─ agent enrolls with fleet server (or uses cached node_key)
   └─ agent spawns extension binary (/usr/lib/edr-agent/extensions/*.ext)
   └─ agent calls sd_notify(READY=1)
   └─ agent starts all tokio tasks
6. systemd marks edr-agent.service as active
─────────────────────────────────────────────────────────────────────
```

```rust
// src/main.rs — startup sequence
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Parse CLI
    let cli = Cli::parse();

    // 2. Initialize logging FIRST (so all subsequent steps are logged)
    init_logging(&cli.config)?;

    tracing::info!("EDR Agent {} starting", env!("CARGO_PKG_VERSION"));

    // 3. Load config
    let config = load_config(&cli.config)?;

    // 4. Run installation/setup if needed
    if config.osquery.auto_install {
        let sys_info = detect_system()?;
        install_osquery(&sys_info).await?;
        configure_osquery(&config).await?;
    }

    // 5. Wait for osqueryd socket
    let osquery = OsqueryClient::new(&config.osquery.socket_path);
    osquery.wait_for_socket(Duration::from_secs(30)).await?;
    tracing::info!("osquery socket available");

    // 6. Enroll with fleet (or load cached credentials)
    let creds = enrollment::enroll_or_load(&config).await?;

    // 7. Run agent main loop (tasks)
    agent::run(config, creds).await?;

    Ok(())
}
```

---

## 16. Logging with tracing

```rust
// src/main.rs
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling;

pub fn init_logging(config_path: &str) -> anyhow::Result<()> {
    let config = load_config_minimal(config_path)?;

    // Non-blocking file appender (doesn't block tokio runtime)
    let file_appender = rolling::daily(
        &config.agent.log_dir,
        "agent.log"
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // _guard must live for the lifetime of the program

    // JSON format for structured logging (easier to parse by SIEM)
    let file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_current_span(true)
        .with_span_list(true);

    // Human-readable for stderr/journal
    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .pretty();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_env("RUST_LOG")
            .unwrap_or_else(|_| EnvFilter::new(&config.agent.log_level)))
        .with(file_layer)
        .with(stderr_layer)
        .init();

    Ok(())
}
```

### Log Levels and What to Log

```rust
// Use spans for context-rich logs
async fn process_event(event: &OsqueryEvent) -> anyhow::Result<()> {
    let span = tracing::info_span!(
        "process_event",
        query_name = %event.query_name,
        action = %event.action,
        pid = event.columns.get("pid").map(|s| s.as_str()).unwrap_or("-"),
    );
    let _enter = span.enter();

    tracing::debug!("Processing event");

    // Log key EDR-relevant events at INFO
    if event.query_name == "bpf_process_events" {
        tracing::info!(
            path = ?event.columns.get("path"),
            cmdline = ?event.columns.get("cmdline"),
            "Process execution event"
        );
    }

    // Log anomalies at WARN
    if is_suspicious(&event) {
        tracing::warn!(
            event = ?event,
            "Suspicious activity detected"
        );
    }

    Ok(())
}
```

---

## 17. Heartbeat & System Metrics to Fleet

```rust
// src/heartbeat.rs
use sysinfo::{System, SystemExt, CpuExt, DiskExt, NetworkExt};
use tokio::time::{interval, Duration};

pub async fn heartbeat_task(
    state: Arc<AgentState>,
    mut grpc: FleetGrpcClient,
) -> anyhow::Result<()> {
    let mut ticker = interval(Duration::from_secs(30));
    let mut sys = System::new_all();

    loop {
        ticker.tick().await;

        // Refresh system info (sysinfo caches; call refresh periodically)
        sys.refresh_cpu();
        sys.refresh_memory();
        sys.refresh_disks_list();
        sys.refresh_networks();

        let cpu_percent = sys.global_cpu_info().cpu_usage();
        let ram_used  = sys.used_memory();
        let ram_total = sys.total_memory();

        // Disk I/O delta since last measurement
        let (disk_read, disk_write) = sum_disk_io(&sys);
        // Network I/O delta
        let (net_rx, net_tx) = sum_network_io(&sys);

        let config = state.config.read();
        let osquery_healthy = check_osquery_health(&config.osquery.socket_path);

        let heartbeat = Heartbeat {
            node_key:  state.node_key.read().clone(),
            timestamp: chrono::Utc::now().timestamp(),
            metrics: Some(SystemMetrics {
                cpu_percent,
                ram_used_bytes:  ram_used,
                ram_total_bytes: ram_total,
                ram_percent: (ram_used as f32 / ram_total as f32) * 100.0,
                disk_read_bytes_sec:  disk_read,
                disk_write_bytes_sec: disk_write,
                net_rx_bytes_sec: net_rx,
                net_tx_bytes_sec: net_tx,
                process_count: sys.processes().len() as u32,
                uptime_seconds: sys.uptime(),
            }),
            status: Some(AgentStatus {
                agent_version:   env!("CARGO_PKG_VERSION").to_string(),
                osquery_version: get_osquery_version(),
                osquery_healthy,
                events_queued:   state.events_queued.load(
                    std::sync::atomic::Ordering::Relaxed),
                buffer_size_bytes: buffer_size_bytes().await,
                isolated: state.is_isolated.load(
                    std::sync::atomic::Ordering::Relaxed),
                ..Default::default()
            }),
        };

        match grpc.send_heartbeat(heartbeat).await {
            Ok(_)  => tracing::debug!("Heartbeat sent"),
            Err(e) => tracing::warn!("Heartbeat failed: {}", e),
        }

        // Also ping systemd watchdog here
        let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]);
    }
}
```

---

## 18. Network Isolation with nftables

When the fleet server sends an `ISOLATE_NETWORK` command, the agent uses
`nftables` to block all traffic **except** to the fleet server.

```rust
// src/isolation.rs
use nftables::{
    batch::Batch,
    expr::{Expression, Meta, MetaKey},
    helper,
    schema::{Chain, NfListObject, Nftables, Rule, Table},
    stmt::{Counter, Drop, Statement},
    types::{NfChainPolicy, NfChainType, NfFamily, NfHook},
};

const EDR_TABLE: &str  = "edr_isolation";
const FLEET_SERVER_IP: &str = "10.0.0.1";
const FLEET_PORT: u16       = 8443;

pub async fn isolate_node(fleet_ip: &str, fleet_port: u16) -> anyhow::Result<()> {
    tracing::warn!("ISOLATING NODE: blocking all traffic except fleet server {}:{}",
                    fleet_ip, fleet_port);

    // Build nftables ruleset via JSON API
    let nft_input = serde_json::json!({
        "nftables": [
            // Create isolation table
            { "add": { "table": { "family": "ip", "name": EDR_TABLE } } },

            // Input chain: DROP by default, ALLOW from fleet only
            { "add": { "chain": {
                "family": "ip", "table": EDR_TABLE,
                "name": "input",
                "type": "filter", "hook": "input",
                "policy": "drop", "prio": 0
            }}},

            // Allow established connections
            { "add": { "rule": {
                "family": "ip", "table": EDR_TABLE, "chain": "input",
                "expr": [
                    { "match": { "op": "==", "left": {"ct": {"key": "state"}},
                                 "right": {"set": ["established", "related"]} }},
                    { "accept": null }
                ]
            }}},

            // Allow fleet server inbound
            { "add": { "rule": {
                "family": "ip", "table": EDR_TABLE, "chain": "input",
                "expr": [
                    { "match": { "op": "==", "left": {"payload": {"protocol": "ip", "field": "saddr"}},
                                 "right": fleet_ip }},
                    { "accept": null }
                ]
            }}},

            // Allow loopback
            { "add": { "rule": {
                "family": "ip", "table": EDR_TABLE, "chain": "input",
                "expr": [
                    { "match": { "op": "==", "left": {"meta": {"key": "iifname"}},
                                 "right": "lo" }},
                    { "accept": null }
                ]
            }}},

            // Output chain: DROP by default, ALLOW to fleet only
            { "add": { "chain": {
                "family": "ip", "table": EDR_TABLE,
                "name": "output",
                "type": "filter", "hook": "output",
                "policy": "drop", "prio": 0
            }}},

            // Allow fleet server outbound
            { "add": { "rule": {
                "family": "ip", "table": EDR_TABLE, "chain": "output",
                "expr": [
                    { "match": { "op": "==", "left": {"payload": {"protocol": "ip", "field": "daddr"}},
                                 "right": fleet_ip }},
                    { "match": { "op": "==", "left": {"payload": {"protocol": "tcp", "field": "dport"}},
                                 "right": fleet_port }},
                    { "accept": null }
                ]
            }}},

            // Allow loopback output
            { "add": { "rule": {
                "family": "ip", "table": EDR_TABLE, "chain": "output",
                "expr": [
                    { "match": { "op": "==", "left": {"meta": {"key": "oifname"}},
                                 "right": "lo" }},
                    { "accept": null }
                ]
            }}}
        ]
    });

    // Apply via nft JSON API (requires nft binary or libnftables)
    apply_nft_rules(&nft_input.to_string())?;
    tracing::warn!("Node isolated. Only fleet server {}:{} is reachable", fleet_ip, fleet_port);
    Ok(())
}

pub async fn deisolate_node() -> anyhow::Result<()> {
    tracing::info!("Removing network isolation");

    let nft_input = serde_json::json!({
        "nftables": [
            { "delete": { "table": { "family": "ip", "name": EDR_TABLE } } }
        ]
    });
    apply_nft_rules(&nft_input.to_string())?;
    tracing::info!("Network isolation removed");
    Ok(())
}

fn apply_nft_rules(json_rules: &str) -> anyhow::Result<()> {
    // Write to temp file and pass to nft
    let tmp = tempfile::NamedTempFile::new()?;
    std::fs::write(tmp.path(), json_rules)?;

    let output = std::process::Command::new("nft")
        .args(["-j", "-f", tmp.path().to_str().unwrap()])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("nft failed: {}", err);
    }
    Ok(())
}
```

> **Why nftables over iptables?** iptables is deprecated in all modern distros
> (RHEL 9+, Ubuntu 22.04+, Debian 11+). nftables is the kernel's supported
> packet filtering subsystem. Our agent uses nftables directly, which works on
> any distro with kernel >= 3.13.

---

## 19. Enrollment Secret & mTLS Authentication

### Enrollment Flow

```
Node boots → agent loads config.toml with enrollment_secret
→ agent generates RSA-2048 or EC-P256 key pair
→ agent creates CSR (Certificate Signing Request)
→ agent sends EnrollRequest{secret, hostname, arch, csr} to fleet
→ fleet verifies secret, signs CSR, returns signed cert + CA cert
→ agent saves cert, key, CA cert to /var/lib/edr-agent/
→ agent writes node_key to config.toml
→ subsequent connections use mTLS with the signed cert
```

```rust
// src/enrollment.rs
use rcgen::{CertificateParams, DistinguishedName, PKCS_ECDSA_P256_SHA256};
use anyhow::Result;

pub async fn enroll_or_load(config: &AgentConfig) -> Result<AgentCredentials> {
    let cert_path = std::path::Path::new(&config.tls.client_cert_path);

    // If cert exists and not expired, use it
    if cert_path.exists() && !is_cert_expiring_soon(&config.tls.client_cert_path)? {
        tracing::info!("Loading existing enrollment credentials");
        return load_credentials(config);
    }

    tracing::info!("Enrolling with fleet server...");
    enroll_new(config).await
}

async fn enroll_new(config: &AgentConfig) -> Result<AgentCredentials> {
    // Generate EC-P256 key pair
    let mut params = CertificateParams::new(vec![
        hostname::get()?.to_string_lossy().to_string()
    ])?;

    params.alg = &PKCS_ECDSA_P256_SHA256;

    let mut dn = DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, hostname::get()?.to_string_lossy());
    dn.push(rcgen::DnType::OrganizationName, "EDR Agent");
    params.distinguished_name = dn;

    let cert = rcgen::Certificate::from_params(params)?;
    let csr_der  = cert.serialize_request_der()?;
    let key_pem  = cert.serialize_private_key_pem();

    // Get system info for enrollment request
    let sysinfo = crate::osquery::installer::detect_system()?;
    let host_uuid = get_host_uuid()?;

    // Build enrollment request
    let req = EnrollRequest {
        enrollment_secret: config.fleet.enroll_secret.clone(),
        host_uuid:         host_uuid.clone(),
        hostname:          hostname::get()?.to_string_lossy().to_string(),
        os_platform:       format!("{:?}", sysinfo.distro),
        os_version:        get_os_version(),
        arch:              format!("{:?}", sysinfo.arch),
        agent_version:     env!("CARGO_PKG_VERSION").to_string(),
        csr:               csr_der,
    };

    // Connect without mTLS for enrollment (using server CA only)
    let mut enroll_client = build_enroll_client(config).await?;
    let response = enroll_client
        .enroll_service_client()
        .enroll(tonic::Request::new(req))
        .await?
        .into_inner();

    // Persist credentials
    let cert_dir = std::path::Path::new(&config.tls.client_cert_path)
        .parent()
        .unwrap();
    std::fs::create_dir_all(cert_dir)?;
    std::fs::write(&config.tls.client_cert_path, &response.client_cert)?;
    std::fs::write(&config.tls.client_key_path, key_pem.as_bytes())?;
    std::fs::write(&config.tls.ca_cert_path, &response.ca_cert)?;

    // Set strict permissions on key
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(
        &config.tls.client_key_path,
        std::fs::Permissions::from_mode(0o600)
    )?;

    // Update config with node_key
    update_config_node_key(&config.path, &response.node_key)?;

    tracing::info!("Enrolled successfully. Node key: {}", &response.node_key[..8]);

    Ok(AgentCredentials {
        node_key:    response.node_key,
        client_cert: response.client_cert.into(),
        client_key:  key_pem.into_bytes(),
        ca_cert:     response.ca_cert.into(),
    })
}

fn get_host_uuid() -> Result<String> {
    // Read SMBIOS UUID from DMI
    let uuid = std::fs::read_to_string("/sys/class/dmi/id/product_uuid")
        .or_else(|_| std::fs::read_to_string("/proc/sys/kernel/random/boot_id"))
        .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
    Ok(uuid.trim().to_string())
}
```

---

## 20. Config Hot-Reload from Fleet Server

The fleet server can push config updates via the `UPDATE_CONFIG` command.
The agent writes the new config to `/etc/edr-agent/config.toml`, then
the inotify watcher (from §13) picks it up and reloads without restart.

```rust
// src/transport/grpc.rs — command handler
async fn handle_command(
    cmd: RemoteCommand,
    state: Arc<AgentState>,
) -> anyhow::Result<CommandResult> {
    match cmd.r#type() {
        CommandType::UpdateConfig => {
            let new_config: AgentConfig = serde_json::from_slice(&cmd.payload)?;
            new_config.validate()?;

            // Atomic write: write to tmp, then rename
            let tmp = tempfile::NamedTempFile::new_in("/etc/edr-agent")?;
            let toml = toml::to_string_pretty(&new_config)?;
            std::io::Write::write_all(&mut tmp.as_file(), toml.as_bytes())?;
            tmp.persist("/etc/edr-agent/config.toml")?;

            tracing::info!("Config updated by fleet server");
            Ok(CommandResult {
                command_id:  cmd.command_id.clone(),
                success:     true,
                output:      "Config updated".to_string(),
                executed_at: chrono::Utc::now().timestamp(),
            })
        }

        CommandType::IsolateNetwork => {
            let fleet_ip = state.config.read().isolation.fleet_server_ip.clone();
            let fleet_port = state.config.read().fleet.url
                .split(':').last()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8443);

            crate::isolation::isolate_node(&fleet_ip, fleet_port).await?;
            state.is_isolated.store(true, std::sync::atomic::Ordering::Relaxed);

            Ok(CommandResult { command_id: cmd.command_id, success: true,
                              output: "Node isolated".to_string(),
                              executed_at: chrono::Utc::now().timestamp() })
        }

        CommandType::DeisolateNetwork => {
            crate::isolation::deisolate_node().await?;
            state.is_isolated.store(false, std::sync::atomic::Ordering::Relaxed);
            Ok(CommandResult { command_id: cmd.command_id, success: true,
                              output: "Isolation removed".to_string(),
                              executed_at: chrono::Utc::now().timestamp() })
        }

        CommandType::RestartOsquery => {
            std::process::Command::new("systemctl")
                .args(["restart", "osqueryd"])
                .status()?;
            Ok(CommandResult { command_id: cmd.command_id, success: true,
                              output: "osqueryd restarted".to_string(),
                              executed_at: chrono::Utc::now().timestamp() })
        }

        CommandType::KillProcess => {
            let pid: i32 = serde_json::from_slice(&cmd.payload)?;
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid),
                nix::sys::signal::Signal::SIGKILL,
            )?;
            Ok(CommandResult { command_id: cmd.command_id, success: true,
                              output: format!("Killed PID {}", pid),
                              executed_at: chrono::Utc::now().timestamp() })
        }

        _ => {
            tracing::warn!("Unknown command type: {:?}", cmd.r#type());
            Ok(CommandResult { command_id: cmd.command_id, success: false,
                              output: "Unknown command".to_string(),
                              executed_at: chrono::Utc::now().timestamp() })
        }
    }
}
```

---

## 21. GitHub Actions — CI + Cross-Compilation + Releases

### `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  RUST_BACKTRACE: 1
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-22.04, ubuntu-24.04]

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Install protoc
        run: |
          sudo apt-get update -qq
          sudo apt-get install -y -qq protobuf-compiler libprotobuf-dev

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Test
        run: cargo test --all-features --workspace

      - name: Install osquery (for integration tests)
        run: |
          curl -fsSL https://pkg.osquery.io/deb/pubkey.gpg \
            | gpg --dearmor > /usr/share/keyrings/osquery.gpg
          echo "deb [signed-by=/usr/share/keyrings/osquery.gpg] \
            https://pkg.osquery.io/deb deb main" \
            > /etc/apt/sources.list.d/osquery.list
          sudo apt-get update -qq && sudo apt-get install -y -qq osquery

      - name: Integration tests (with osquery)
        run: cargo test --test integration -- --test-threads=1
        env:
          OSQUERY_SOCKET: /tmp/test-osquery.em
```

### `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'

permissions:
  contents: write   # create GitHub release + upload assets

jobs:
  # ── Build for all targets ─────────────────────────────────────────────
  build:
    name: Build ${{ matrix.target }}
    runs-on: ubuntu-24.04
    strategy:
      fail-fast: false
      matrix:
        include:
          # Linux x86_64 (musl = fully static, runs on any distro)
          - target: x86_64-unknown-linux-musl
            binary_name: edr-agent-linux-amd64
            pkg_deb_arch: amd64
            pkg_rpm_arch: x86_64

          # Linux ARM64 (musl static)
          - target: aarch64-unknown-linux-musl
            binary_name: edr-agent-linux-aarch64
            pkg_deb_arch: arm64
            pkg_rpm_arch: aarch64

          # Linux ARMv7 (Raspberry Pi 3 and older)
          - target: armv7-unknown-linux-musleabihf
            binary_name: edr-agent-linux-armhf
            pkg_deb_arch: armhf
            pkg_rpm_arch: armhf

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install protoc
        run: sudo apt-get install -y protobuf-compiler libprotobuf-dev

      - name: Install cross
        uses: taiki-e/install-action@v2
        with:
          tool: cross

      - name: Build (cross-compile with musl)
        env:
          CROSS_NO_WARNINGS: "0"
        run: |
          cross build \
            --release \
            --target ${{ matrix.target }} \
            --bin edr-agent \
            --locked

      - name: Package binary
        run: |
          mkdir -p dist
          cp target/${{ matrix.target }}/release/edr-agent dist/edr-agent
          cp install/install.sh dist/
          cp install/config.toml.example dist/

          # Create tarball
          tar -czf ${{ matrix.binary_name }}.tar.gz \
            -C dist \
            edr-agent install.sh config.toml.example

          # SHA256 checksum
          sha256sum ${{ matrix.binary_name }}.tar.gz \
            > ${{ matrix.binary_name }}.tar.gz.sha256

      # ── Build .deb package (for Debian/Ubuntu) ─────────────────────
      - name: Build .deb (Debian/Ubuntu)
        if: matrix.pkg_deb_arch != ''
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          cargo install cargo-deb --locked 2>/dev/null || true

          # cargo-deb reads from Cargo.toml [package.metadata.deb]
          # Configure in Cargo.toml:
          # [package.metadata.deb]
          # maintainer = "Your Team <sec@company.com>"
          # depends = ""        # empty: fully static binary
          # section = "utils"
          # priority = "optional"
          # assets = [
          #   ["target/release/edr-agent", "/opt/edr-agent/edr-agent", "755"],
          #   ["install/edr-agent.service", "/etc/systemd/system/", "644"],
          #   ["install/config.toml.example", "/etc/edr-agent/config.toml.example", "644"],
          # ]
          # maintainerscripts = "install/maintainer-scripts"

          cargo deb \
            --target ${{ matrix.target }} \
            --no-build \
            -o edr-agent_${VERSION}_${{ matrix.pkg_deb_arch }}.deb

      # ── Build .rpm package (for RHEL/Fedora) ──────────────────────
      - name: Build .rpm (RHEL/Fedora)
        if: matrix.pkg_rpm_arch == 'x86_64' || matrix.pkg_rpm_arch == 'aarch64'
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          sudo apt-get install -y rpm
          cargo install cargo-rpm --locked 2>/dev/null || true
          cargo rpm build --target ${{ matrix.target }} || true

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.target }}
          path: |
            ${{ matrix.binary_name }}.tar.gz
            ${{ matrix.binary_name }}.tar.gz.sha256
            *.deb
            *.rpm
          retention-days: 1

  # ── Create GitHub Release ─────────────────────────────────────────────
  release:
    name: Create Release
    needs: build
    runs-on: ubuntu-24.04

    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts
          pattern: binaries-*
          merge-multiple: true

      - name: Generate release notes
        id: notes
        run: |
          echo "## EDR Agent ${GITHUB_REF_NAME}" > RELEASE_NOTES.md
          echo "" >> RELEASE_NOTES.md
          echo "### Downloads" >> RELEASE_NOTES.md
          echo "| Platform | File | Checksum |" >> RELEASE_NOTES.md
          echo "|---|---|---|" >> RELEASE_NOTES.md
          for f in artifacts/*.tar.gz; do
            name=$(basename $f)
            sha=$(cat ${f}.sha256 | cut -d' ' -f1)
            echo "| ${name} | \`${name}\` | \`${sha:0:16}...\` |" >> RELEASE_NOTES.md
          done

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          name: "EDR Agent ${{ github.ref_name }}"
          body_path: RELEASE_NOTES.md
          draft: false
          prerelease: ${{ contains(github.ref_name, '-rc') || contains(github.ref_name, '-beta') }}
          files: |
            artifacts/*.tar.gz
            artifacts/*.tar.gz.sha256
            artifacts/*.deb
            artifacts/*.rpm
```

### Cross.toml (for cross-rs)

```toml
# Cross.toml — placed at repo root
[build.env]
passthrough = [
    "PROTOC",
    "PROTOC_INCLUDE",
]

[target.aarch64-unknown-linux-musl]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-musl:main"
pre-build = [
    "apt-get install -y -qq protobuf-compiler libprotobuf-dev",
]

[target.armv7-unknown-linux-musleabihf]
image = "ghcr.io/cross-rs/armv7-unknown-linux-musleabihf:main"
pre-build = [
    "apt-get install -y -qq protobuf-compiler libprotobuf-dev",
]
```

---

## 22. Benchmarking the Agent

### What to Benchmark

```bash
# Install hyperfine and criterion deps
cargo install hyperfine

# Benchmark: time to execute a query and receive results
hyperfine \
  'osqueryi --line "SELECT COUNT(*) FROM processes;"' \
  --runs 20 \
  --warmup 3
```

```rust
// benches/query_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;

fn bench_osquery_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(async {
        OsqueryClient::new("/var/osquery/osquery.em")
    });

    let mut group = c.benchmark_group("osquery_queries");

    // Benchmark lightweight query
    group.bench_function("processes_count", |b| {
        b.to_async(&rt).iter(|| async {
            client.query("SELECT COUNT(*) FROM processes;").unwrap()
        });
    });

    // Benchmark event table drain (simulates real EDR workload)
    group.bench_function("bpf_process_events_drain", |b| {
        b.to_async(&rt).iter(|| async {
            client.query("SELECT * FROM bpf_process_events;").unwrap()
        });
    });

    // Benchmark full process list with JOINs
    group.bench_function("processes_with_listeners", |b| {
        b.to_async(&rt).iter(|| async {
            client.query(
                "SELECT p.pid, p.name, lp.port FROM processes p \
                 LEFT JOIN listening_ports lp USING(pid);"
            ).unwrap()
        });
    });

    group.finish();
}

// Benchmark gRPC throughput
fn bench_grpc_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_with_input(
        BenchmarkId::new("grpc_send_batch", "500_events"),
        &500usize,
        |b, &batch_size| {
            b.to_async(&rt).iter(|| async move {
                let batch = generate_test_batch(batch_size);
                grpc_client.send_events(batch).await.unwrap();
            });
        },
    );
}

// Benchmark redb write throughput
fn bench_buffer_write(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let mut buf = EventBuffer::open(tmp.path().join("bench.redb")).unwrap();

    let mut group = c.benchmark_group("buffer");

    for batch_size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("write_batch", batch_size),
            batch_size,
            |b, &size| {
                let events = generate_test_events(size);
                b.iter(|| {
                    buf.write_batch(&events).unwrap()
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches,
    bench_osquery_query,
    bench_grpc_throughput,
    bench_buffer_write,
);
criterion_main!(benches);
```

### Key Metrics to Track

```bash
# Memory usage
/usr/bin/time -v /opt/edr-agent/edr-agent --config /etc/edr-agent/config.toml &
PID=$!
sleep 60
cat /proc/$PID/status | grep -E "VmRSS|VmPeak|Threads"

# CPU usage under load
pidstat -p $PID -d 1 30

# Event throughput
journalctl -u edr-agent -o json --since "1 min ago" \
  | jq 'select(.MESSAGE | test("events_sent"))' \
  | jq '.events_sent' \
  | awk '{sum += $1} END {print "avg events/s:", sum/NR}'

# gRPC latency (add tracing spans to measure)
tokio_console  # https://github.com/tokio-rs/console
```

---

## 23. eBPF Event Loss — Probability & Mitigation

### How osquery's BPF Events Work Internally

osquery uses the **perf event ring buffer** (not the newer BPF ring buffer).
The perf ring buffer is **per-CPU** and **SPSC** (single-producer:
single-consumer):

```
CPU 0 kernel: execve() syscall
    → BPF program fires (attached to kprobe/tracepoint)
    → bpf_perf_event_output() writes event to CPU 0's ring buffer
    └─ if ring buffer full: event DROPPED, probe_error=1 on next event

osquery userspace thread:
    → epoll_wait() on all per-CPU ring buffer fds
    → reads and empties each ring buffer
    → writes to RocksDB
```

### When Events Are Dropped

| Condition | Drop Probability | Mitigation |
|---|---|---|
| Burst of 10,000+ execve/s (e.g. build system) | HIGH | Increase `bpf_perf_event_array_exp`, reduce osquery drain interval |
| osquery worker is slow (complex JOIN) | MEDIUM | Set `--events_optimize=true`, drain event tables frequently |
| Watchdog kills worker (CPU spike) | HIGH | Set `"denylist": false` on event queries |
| `events_max` exceeded | MEDIUM | Increase `--events_max`, query more often |
| VM with hotswap CPUs (128 possible) | MEDIUM | Set `--bpf_buffer_storage_size=64` |

### Detection: How to Know You're Losing Events

```sql
-- Check probe_error column in bpf_process_events
SELECT pid, path, probe_error, time FROM bpf_process_events
WHERE probe_error = 1;

-- Track the lost_events counter from osquery internals
-- (There's no direct table for this, but check log for warnings)
-- grep the logs:
grep -i "lost\|dropped\|overflow" /var/log/osquery/osqueryd.INFO
```

In the osquery source (`bpf_publisher.cpp`), when a perf event slot is full,
the `probe_error` field is set to `1` on the NEXT captured event, indicating
that between the previous event and this one, some events were dropped. This
is a **lagging indicator** — you see the error on a successfully captured
event, not on the dropped ones.

### Mitigation Strategy

```ini
# For high-throughput servers (>500 exec/s):
--bpf_perf_event_array_exp=12        # 4096 slots per CPU (was 1024)
--bpf_buffer_storage_size=1024       # 1024 × 4096 bytes per pool per CPU

# Drain event tables more frequently
# In osquery.conf schedule:
"bpf_process_events": {
  "query": "SELECT * FROM bpf_process_events;",
  "interval": 10,     ← was 30s; more frequent = smaller batches = less overflow
  "denylist": false
}

# Alternative for extreme throughput: use Audit instead of BPF
# Audit does NOT use ring buffers; it uses the kernel audit kthread
# and backlog queue, which is easier to tune:
--audit_backlog_limit=16384
```

### BPF Ring Buffer vs Perf Event Array

If you build your own BPF-based component (using `aya` crate), prefer the
**BPF ring buffer** (kernel >= 5.8) over perf event arrays:

| | BPF Ring Buffer | Perf Event Array |
|---|---|---|
| Kernel minimum | 5.8 | 4.1 |
| Memory layout | Single shared buffer | Per-CPU |
| Event ordering | Guaranteed | Not guaranteed |
| Memory efficiency | Better | Worse (x CPUs) |
| Event loss indicator | Explicit discard count | `probe_error` field |
| osquery uses | ❌ (still perf) | ✅ |

---

## 24. Debugging Deep Dive

### "A Table Returns Empty"

```bash
# Step 1: Run osqueryi manually with verbose
sudo osqueryi --verbose \
  --flagfile /etc/osquery/osquery.flags \
  "SELECT * FROM bpf_process_events LIMIT 5;"

# Step 2: Check publisher/subscriber state
sudo osqueryi --flagfile /etc/osquery/osquery.flags \
  "SELECT name, type, subscriptions, events, active FROM osquery_events;"
# active=0 for a publisher means it failed to initialize

# Step 3: Check flags actually loaded
sudo osqueryi --flagfile /etc/osquery/osquery.flags \
  "SELECT name, value, default_value FROM osquery_flags WHERE default_value <> value;"

# Step 4: Verify audit socket ownership
auditctl -s 2>/dev/null | grep "^pid"
# pid should match osqueryd's PID
ps aux | grep osqueryd

# Step 5: Check if auditd is running
systemctl is-active auditd && echo "PROBLEM: auditd running, must be stopped"

# Step 6: Check kernel audit is enabled
cat /proc/sys/kernel/audit   # should be 1

# Step 7: Trigger events and check
( ls /tmp; sleep 1; sudo osqueryi --flagfile /etc/osquery/osquery.flags \
  "SELECT pid, path, cmdline FROM process_events LIMIT 5;" )
```

### "Buffer / Ring Buffer Overflow" Debugging

```bash
# Check kernel audit drop rate
auditctl -s | grep -E "lost|backlog"
# lost > 0: increase --audit_backlog_limit
# backlog > 0.8 * backlog_limit: you're near overflow

# Check eBPF ring buffer drops
# osquery logs these at WARNING level:
grep -i "perf.*lost\|dropped\|overflow" /var/log/osquery/osqueryd.INFO

# Measure execve rate on your system
sudo perf stat -e syscalls:sys_enter_execve -a sleep 10
# If > 500/s: you need larger BPF buffers
```

### "Extension Not Registering"

```bash
# Verify permissions (must be owned root:root, not world-writable)
ls -la /usr/lib/edr-agent/extensions/
# Should be: -rwx------ root root ... my_ext.ext

# Check extension log (extension writes to its own stderr → journald)
journalctl -t edr-custom-tables -f

# Run extension manually to see errors:
sudo /usr/lib/edr-agent/extensions/edr_custom_tables.ext \
  --socket /var/osquery/osquery.em \
  --timeout 3 \
  --interval 3 \
  --verbose

# Verify extension is registered in osquery:
sudo osqueryi "SELECT * FROM osquery_extensions;"
# Should show your extension with active state

# Verify your table exists:
sudo osqueryi ".tables" | grep hidden_processes
```

### "Thrift Connection Errors"

```bash
# Common error: "connection refused" or "socket not ready"
# Check socket exists:
ls -la /var/osquery/osquery.em

# Check socket permissions (extension must be able to connect)
stat /var/osquery/osquery.em
# Usually: srw-rw---- root osquery (extension needs to be root)

# Test with nc (netcat doesn't work on UDS, but strace does):
sudo strace -e trace=connect,socket,read,write \
  osqueryi "SELECT 1;" 2>&1 | grep -E "connect|socket|AF_UNIX"
# Should see: connect(fd, {sa_family=AF_UNIX, sun_path="/var/osquery/osquery.em"}, ...)

# Test Thrift handshake manually:
# The osquery Thrift protocol: framed binary
# Frame format: [4-byte big-endian length][Thrift binary message]
# A valid PING request is ~30 bytes. You can craft it with Python or Rust.
```

### "gRPC Connection Issues"

```bash
# Test gRPC endpoint:
grpcurl -insecure -proto proto/edr.proto \
  fleet.example.com:8443 \
  edr.v1.HeartbeatService/SendHeartbeat

# Check TLS certificate chain:
openssl s_client -connect fleet.example.com:8443 \
  -cert /var/lib/edr-agent/agent.crt \
  -key /var/lib/edr-agent/agent.key \
  -CAfile /var/lib/edr-agent/fleet-ca.crt

# Check certificate expiry:
openssl x509 -in /var/lib/edr-agent/agent.crt -noout -dates

# Enable tonic debug logging:
RUST_LOG=tonic=debug,h2=debug edr-agent --config /etc/edr-agent/config.toml
```

### "osquery Fails to Parse / Abstractions"

osquery uses **SQLite's virtual table API** as its abstraction layer. Each
table is a SQLite virtual module. Understanding this explains many failure modes:

```bash
# "no such table" = table not built for this platform, or extension not loaded
sudo osqueryi ".tables" | grep <table_name>

# "not supported on this platform" in results = table compiled but not available
sudo osqueryi ".schema bpf_process_events"
# Shows column types and notes

# Table returns empty due to constraint mismatch:
# Some tables REQUIRE constraints (called "index" columns)
# Without constraints, they do a full scan that may return nothing
# Example: hash table requires path constraint
sudo osqueryi "SELECT * FROM hash;"                    # empty (no path given)
sudo osqueryi "SELECT * FROM hash WHERE path='/etc/passwd';"  # works

# Use the EXPLAIN keyword to see the query plan:
sudo osqueryi "EXPLAIN QUERY PLAN SELECT * FROM processes WHERE pid=1234;"
# Look for "SCAN" vs "SEARCH" — SEARCH means the table uses the constraint
```

### Tokio Debugging with tokio-console

```toml
# Cargo.toml — add tokio-console support
[dependencies]
console-subscriber = "0.3"   # only in dev builds

# Enable with:
# TOKIO_CONSOLE_BIND=127.0.0.1:6669 cargo run
```

```rust
// main.rs — conditional console subscriber
#[cfg(debug_assertions)]
{
    console_subscriber::init();
}
```

```bash
# Install tokio-console
cargo install --locked tokio-console

# Run agent with console support
RUST_LOG=info TOKIO_CONSOLE_BIND=127.0.0.1:6669 ./edr-agent &

# Connect console (shows all tasks, their state, wakeups, blocking calls)
tokio-console http://127.0.0.1:6669
```

---

## 25. Security Hardening

### Agent Binary

```toml
# Cargo.toml — security-hardening compile options
[profile.release]
overflow-checks = true   # detect integer overflow in release builds
```

```bash
# Verify binary hardening:
checksec --file=./target/release/edr-agent
# Should show: FULL RELRO, STACK CANARY, NX, PIE, FORTIFY

# For musl-linked binaries, PIE depends on compile flags
# Add to .cargo/config.toml:
# [target.x86_64-unknown-linux-musl]
# rustflags = ["-C", "relocation-model=pie"]
```

```ini
# /etc/systemd/system/edr-agent.service — security limits
[Service]
# File system
ProtectHome=true           # /home is hidden
ProtectSystem=false        # agent needs /etc, /var, /proc write access
PrivateTmp=false           # agent may need /tmp (osquery ext socket)
ReadWritePaths=/var/lib/edr-agent /var/log/edr-agent /etc/edr-agent /etc/osquery
# Capabilities (agent needs root for kernel access)
# NoNewPrivileges=false  ← cannot set for root services with BPF
# InaccessiblePaths=/home/user1   ← isolate user directories
```

### Secret Storage

```bash
# Never store enrollment secrets in environment variables (visible in /proc/<pid>/environ)
# Use a secrets file with strict permissions:
chmod 600 /etc/edr-agent/config.toml
chown root:root /etc/edr-agent/config.toml

# Or use systemd credentials (systemd >= 250):
systemd-creds encrypt --name=enroll-secret - /etc/credstore/enroll-secret.cred
# Reference in service: LoadCredential=enroll-secret:/etc/credstore/enroll-secret.cred
# Access in Rust: std::fs::read_to_string("/run/credentials/edr-agent.service/enroll-secret")
```

---

## 26. Resources for Further Deep Dive

### Rust Ecosystem

| Resource | URL |
|---|---|
| Tokio documentation | https://tokio.rs/tokio/tutorial |
| Tonic (gRPC) | https://github.com/hyperium/tonic |
| `redb` embedded database | https://github.com/cberner/redb |
| `aya` (Rust eBPF framework) | https://aya-rs.dev/book/ |
| `nftables` Rust crate | https://docs.rs/nftables |
| `sd-notify` crate | https://docs.rs/sd-notify |
| `osquery-rust-ng` | https://crates.io/crates/osquery-rust-ng |
| `osquery-rs` (query executor) | https://github.com/AbdulRhmanAlfaifi/osquery-rs |
| `cross` (cross-compilation) | https://github.com/cross-rs/cross |
| `cargo-deb` | https://github.com/kornelski/cargo-deb |
| `tokio-console` | https://github.com/tokio-rs/console |
| Criterion (benchmarking) | https://bheisler.github.io/criterion.rs/book/ |

### osquery & Thrift

| Resource | URL |
|---|---|
| osquery Thrift IDL | https://github.com/osquery/osquery-python/blob/master/osquery.thrift |
| osquery SDK docs | https://osquery.readthedocs.io/en/stable/development/osquery-sdk/ |
| Extensions deployment | https://osquery.readthedocs.io/en/stable/deployment/extensions/ |
| osquery-go (reference impl) | https://github.com/osquery/osquery-go |
| osquery Slack | https://slack.osquery.io |
| Palantir osquery config | https://github.com/palantir/osquery-configuration |
| Apache Thrift Rust | https://docs.rs/thrift |

### Linux Internals

| Resource | URL |
|---|---|
| Linux Audit internals | https://linux-audit.com/linux-audit-framework-basics/ |
| BPF ring buffer design | https://nakryiko.com/posts/bpf-ringbuf/ |
| eBPF verifier | https://ebpf.io/what-is-ebpf/ |
| nftables wiki | https://wiki.nftables.org |
| `inotify(7)` man page | `man 7 inotify` |
| `/proc` filesystem | https://www.kernel.org/doc/html/latest/filesystems/proc.html |
| netlink audit socket | https://www.kernel.org/doc/html/latest/userspace-api/audit/ |

### Packaging & Deployment

| Resource | URL |
|---|---|
| `cargo-deb` guide | https://github.com/kornelski/cargo-deb#readme |
| `cargo-rpm` | https://github.com/rpm-rs/rpm |
| GitHub Actions cross-rs | https://github.com/marketplace/actions/build-rust-projects-with-cross |
| systemd unit reference | https://www.freedesktop.org/software/systemd/man/systemd.service.html |
| systemd credentials | https://systemd.io/CREDENTIALS/ |
| musl libc | https://musl.libc.org |

---

*Guide written against Rust 1.78+, osquery 5.x, tokio 1.x, tonic 0.12, redb 2.x — June 2026.*
