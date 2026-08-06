# osquery EDR — Complete Bare-Metal Linux Setup Guide

> **Persona**: Senior Linux/osquery engineer with deep kernel internals and security engineering
> experience. This guide covers everything from flags → events → eBPF → extensions → EDR
> scheduled queries, including *exactly* why each table may return empty results.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [System Prerequisites & Kernel Requirements](#2-system-prerequisites--kernel-requirements)
3. [Installation](#3-installation)
4. [Critical System Tunables](#4-critical-system-tunables)
5. [The osquery.flags File](#5-the-osqueryflagsfile) ← **The production flags file**
6. [The osquery.conf File](#6-the-osqueryconf-file) ← **The production conf file**
7. [Complete Linux Tables Reference](#7-complete-linux-tables-reference)
   - 7.1 [Always-Available Tables (no dependencies)](#71-always-available-tables)
   - 7.2 [Tables That Require Root](#72-tables-that-require-root)
   - 7.3 [Audit-Based Event Tables](#73-audit-based-event-tables)
   - 7.4 [eBPF Event Tables](#74-ebpf-event-tables)
   - 7.5 [FIM / inotify Tables](#75-fim--inotify-tables)
   - 7.6 [Package Manager Tables](#76-package-manager-tables)
   - 7.7 [Docker / Container Tables](#77-docker--container-tables)
   - 7.8 [LXD Tables](#78-lxd-tables)
   - 7.9 [Networking Tables and iptables vs nftables](#79-networking-tables-and-iptables-vs-nftables)
   - 7.10 [LSM Tables — AppArmor & SELinux](#710-lsm-tables--apparmor--selinux)
   - 7.11 [YARA Tables](#711-yara-tables)
8. [Why a Table Returns Empty Results — Root-Cause Diagnostics](#8-why-a-table-returns-empty-results)
9. [Audit Events: Deep Setup](#9-audit-events-deep-setup)
10. [eBPF Events: Deep Setup](#10-ebpf-events-deep-setup)
11. [File Integrity Monitoring Setup](#11-file-integrity-monitoring-setup)
12. [Extensions Setup](#12-extensions-setup)
13. [Systemd Service Configuration](#13-systemd-service-configuration)
14. [Scheduled Queries — EDR Pack Design](#14-scheduled-queries--edr-pack-design)
15. [Log Format & Processing](#15-log-format--processing)
16. [Verification & Testing](#16-verification--testing)
17. [Performance Tuning & Watchdog](#17-performance-tuning--watchdog)
18. [Troubleshooting Runbook](#18-troubleshooting-runbook)

---

## 1. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Bare-Metal Linux Host                         │
│                                                                       │
│  ┌──────────────┐  Thrift/UDS  ┌─────────────────────────────────┐  │
│  │  Extension   │◄────────────►│            osqueryd              │  │
│  │  Processes   │              │  ┌─────────────────────────────┐ │  │
│  │  (.ext)      │              │  │  SQLite Virtual Table Layer │ │  │
│  └──────────────┘              │  └──────┬──────┬──────┬───────┘ │  │
│                                │         │      │      │         │  │
│  ┌─────────────────┐           │  ┌──────▼──┐ ┌─▼────┐ ┌──▼──┐  │  │
│  │   /etc/osquery/ │           │  │ /proc   │ │Audit │ │ BPF │  │  │
│  │   osquery.conf  │──────────►│  │  /sys   │ │netln │ │perf │  │  │
│  │   osquery.flags │           │  │  /dev   │ │ sock │ │ buf │  │  │
│  └─────────────────┘           │  └─────────┘ └──────┘ └─────┘  │  │
│                                │  ┌──────────────────────────────┐│  │
│  ┌─────────────────┐           │  │      RocksDB Backing Store   ││  │
│  │/var/log/osquery/│◄──────────│  │  (event buffers, diffs)      ││  │
│  │  results.log    │           │  └──────────────────────────────┘│  │
│  │  status.log     │           └─────────────────────────────────┘  │
│  └─────────────────┘                                                 │
└──────────────────────────────────────────────────────────────────────┘
```

osqueryd operates as two tiers:

- **Worker process** — executes queries, runs table implementations, talks to kernel
- **Watchdog process** — monitors worker's CPU/memory, restarts it if limits are exceeded

Event publishers (Audit netlink, eBPF perf ring buffer, inotify) push events into
RocksDB. Subscribers (table implementations) drain RocksDB when a `SELECT` runs.

---

## 2. System Prerequisites & Kernel Requirements

### Hard Requirements

| Requirement | Minimum | Recommended (EDR) |
|---|---|---|
| Kernel version (Audit events) | 2.6.x | 5.x |
| Kernel version (BPF events) | **4.18** | 5.10+ |
| Architecture | x86_64 | x86_64 / arm64 |
| RAM | 256 MB | 1 GB+ |
| Disk (logs) | 1 GB | 20 GB+ |
| Run as | root | root |

### Kernel Feature Checks

```bash
# Check kernel version
uname -r

# Verify Audit subsystem is in kernel
grep -i "audit" /boot/config-$(uname -r) | grep "CONFIG_AUDIT=y"
# Or:
cat /proc/sys/kernel/audit                 # returns 1 if enabled

# Verify eBPF support (kernel >= 4.18 is NOT enough alone)
zcat /proc/config.gz 2>/dev/null | grep -E "CONFIG_BPF=y|CONFIG_BPF_SYSCALL=y|CONFIG_PERF_EVENTS=y"
# Alternative:
grep -E "CONFIG_BPF=y|CONFIG_BPF_SYSCALL=y" /boot/config-$(uname -r)

# Verify inotify for FIM
cat /proc/sys/fs/inotify/max_user_watches
# Should be > 0; default 8192 is too low for EDR

# Check if auditd is running (MUST be stopped for osquery audit)
systemctl is-active auditd
ps aux | grep auditd

# Check iptables vs nftables backend
ls /proc/net/ip_tables_names 2>&1
# If this file does NOT exist, iptables table in osquery will return empty
nft list ruleset 2>/dev/null | head -5
```

### Packages to Install

```bash
# ─── Ubuntu/Debian ────────────────────────────────────────────────────
apt-get update
apt-get install -y \
  wget \
  curl \
  gnupg2 \
  lsb-release \
  libcap2 \
  libudev1 \
  libblkid1 \
  libaudit1 \
  libssl3 \
  # For Docker tables:
  docker.io \
  # For YARA scanning:
  libyara-dev \
  # For Augeas tables:
  augeas-tools \
  libaugeas0 \
  # For SMART disk info:
  smartmontools \
  # For iptables table (if on nftables host):
  iptables

# ─── RHEL/CentOS/Rocky ────────────────────────────────────────────────
yum install -y \
  wget \
  curl \
  libcap \
  libudev \
  libblkid \
  audit-libs \
  openssl-libs \
  docker-ce \
  smartmontools \
  iptables \
  augeas-libs

# Stop and PERMANENTLY DISABLE auditd — osquery owns the audit socket
systemctl stop auditd
systemctl disable auditd
systemctl mask auditd

# Mask journald audit socket to avoid log flooding (Linux-specific fix):
systemctl mask --now systemd-journald-audit.socket
```

> **Why disable auditd?** osquery claims the kernel audit netlink socket
> exclusively. The socket is a single-consumer resource — only ONE process can
> `SO_BIND` to `NETLINK_AUDIT`. If `auditd` is running when osqueryd starts,
> osquery will get `EACCES` on the netlink socket, all `_events` tables will
> silently return zero rows, and no error is printed unless `--verbose` is set.

---

## 3. Installation

### Using Official Packages (Recommended)

```bash
# ─── Ubuntu/Debian ────────────────────────────────────────────────────
export OSQUERY_KEY=1484120AC4E9F8A1A577AEEE97A80C63C9D8B80B
gpg --recv-keys --keyserver hkps://keys.openpgp.org $OSQUERY_KEY 2>/dev/null || \
  curl -fsSL https://pkg.osquery.io/deb/pubkey.gpg | gpg --dearmor -o /usr/share/keyrings/osquery.gpg

echo "deb [signed-by=/usr/share/keyrings/osquery.gpg] https://pkg.osquery.io/deb deb main" \
  | tee /etc/apt/sources.list.d/osquery.list

apt-get update
apt-get install osquery

# ─── RHEL/CentOS/Rocky/Fedora ─────────────────────────────────────────
curl -L https://pkg.osquery.io/rpm/GPG | tee /etc/pki/rpm-gpg/RPM-GPG-KEY-osquery

cat > /etc/yum.repos.d/osquery.repo << 'EOF'
[osquery-s3-rpm-release]
name=osquery-s3-rpm-release
baseurl=https://pkg.osquery.io/rpm
enabled=1
repo_gpgcheck=1
gpgcheck=0
gpgkey=https://pkg.osquery.io/rpm/GPG
EOF

yum install osquery

# ─── Verify installation ──────────────────────────────────────────────
osqueryd --version
osqueryi --version
```

### Directory Setup

```bash
mkdir -p /etc/osquery/packs
mkdir -p /etc/osquery/extensions.load.d
mkdir -p /var/log/osquery
mkdir -p /var/osquery
mkdir -p /usr/lib/osquery/extensions

# Set permissions
chmod 700 /etc/osquery
chmod 755 /var/log/osquery
chmod 755 /var/osquery

# Create the extensions.load file (empty initially)
touch /etc/osquery/extensions.load
```

---

## 4. Critical System Tunables

These kernel parameters must be applied **before** starting osqueryd.
Add them to `/etc/sysctl.d/60-osquery-edr.conf`:

```bash
cat > /etc/sysctl.d/60-osquery-edr.conf << 'EOF'
# ── inotify limits (for FIM / file_events table) ──────────────────────
# Default 8192 is grossly insufficient for EDR file monitoring
fs.inotify.max_user_watches = 524288
fs.inotify.max_user_instances = 256
fs.inotify.max_queued_events = 32768

# ── Audit backlog (for process_events / socket_events via Audit) ───────
# Kernel audit queue — more headroom before events are dropped
# kernel.audit_backlog_limit is set by osquery --audit_backlog_limit flag
# at runtime; this sysctl is a fallback floor
# kernel.audit_backlog_limit = 8192    # let osquery manage this

# ── Perf event buffer (for eBPF events) ───────────────────────────────
# Allow unprivileged perf events; osquery runs as root so not strictly needed
# kernel.perf_event_paranoid = -1
# kernel.perf_event_max_sample_rate = 100000

# ── Memory management ─────────────────────────────────────────────────
# Reduce OOM risk for RocksDB under sustained event load
vm.overcommit_memory = 1
EOF

sysctl -p /etc/sysctl.d/60-osquery-edr.conf
```

---

## 5. The osquery.flags File

Save to `/etc/osquery/osquery.flags`. This file is the **single source of truth**
for osquery daemon initialization.

> **Rule**: Flags in this file control startup behaviour and override
> matching `options` keys in the conf. Never wrap values in quotes.
> No shell variable expansion occurs inside flagfiles.

```ini
# ==============================================================================
# /etc/osquery/osquery.flags
# Bare-Metal Linux EDR — Production Flags File
# Tested with osquery 5.x on Ubuntu 20.04/22.04, RHEL 8/9, Rocky 8/9
# ==============================================================================


# ─────────────────────────── Configuration Plugin ─────────────────────────────
--config_plugin=filesystem
--config_path=/etc/osquery/osquery.conf
# Hot-reload config every 5 minutes. Useful if your EDR manager rewrites the conf.
--config_refresh=300
# If config endpoint unreachable, retry faster (120s)
--config_accelerated_refresh=120


# ─────────────────────────── Logging ──────────────────────────────────────────
--logger_plugin=filesystem
--logger_path=/var/log/osquery/
# Log file mode: 0640 = root:adm readable; adjust to match your SIEM agent uid
--logger_mode=0640
# Rotate logs when they hit 25 MB
--logger_rotate=true
--logger_rotate_size=26214400
# Keep 25 rotated files (~625 MB max per log type)
--logger_rotate_max_files=25
# Each scheduled query result row is emitted as an individual JSON event
--logger_event_type=true
# Snapshot query rows also emitted individually (better for streaming pipelines)
--logger_snapshot_event_type=true
# Minimum log level: 0=INFO, 1=WARNING, 2=ERROR, 3=NONE
--logger_min_status=0
# Mirror status logs to stderr (useful if you capture journald output too)
--logger_stderr=true


# ─────────────────────────── Storage ──────────────────────────────────────────
--database_path=/var/osquery/osquery.db


# ─────────────────────────── Daemon Process Control ───────────────────────────
--pidfile=/var/osquery/osqueryd.pidfile
# Kill stale osqueryd on start if pidfile exists
--force=true


# ─────────────────────────── Watchdog ─────────────────────────────────────────
# Level 0 = default limits: 200 MB RAM, 10% CPU for 12s
# We override both below to suit EDR workloads
--watchdog_level=0
# Allow up to 400 MB RAM (EDR event tables buffer a lot in RocksDB)
--watchdog_memory_limit=400
# Allow up to 25% sustained CPU over the check window
--watchdog_utilization_limit=25
# Give the worker 90s at startup before watchdog enforces limits
# (initial queries + audit rule load spike can breach limits momentarily)
--watchdog_delay=90
# After graceful shutdown request, force-kill after 4s
--watchdog_forced_shutdown_delay=4
# Print CPU/memory stats every 3s in --verbose mode
--enable_watchdog_debug=false


# ─────────────────────────── Extensions ───────────────────────────────────────
--disable_extensions=false
--extensions_socket=/var/osquery/osquery.em
# Load any extensions listed in this file (one path per line, .ext binaries)
--extensions_autoload=/etc/osquery/extensions.load
# Wait up to 3s for each extension to register its plugins
--extensions_timeout=3
# Check extension health every 3s
--extensions_interval=3


# ─────────────────────────── Events Subsystem (Master Toggle) ─────────────────
# MUST be false to enable ANY event-based table (*_events tables)
--disable_events=false
# Events older than this many seconds are expired from RocksDB after a SELECT
--events_expiry=3600
# Maximum events buffered in RocksDB per subscriber before oldest are dropped
--events_max=50000
# Optimization: subsequent SELECT queries on event tables only return NEW events
# (saves bandwidth; safe for EDR because we always consume immediately)
--events_optimize=true
# Don't denylist event-table queries (watchdog should not suppress EDR queries)
--events_enforce_denylist=false


# ─────────────────────────── Linux Audit Subsystem ────────────────────────────
# MASTER toggle — must be false to open the audit netlink socket
# PRE-CONDITION: auditd must be stopped and disabled!
--disable_audit=false
# Let osquery manage audit rules and kernel enable flag
--audit_allow_config=true
# If another process briefly steals the netlink socket, re-claim it
--audit_persist=true
# Kernel audit backlog ring buffer size (number of events, not bytes)
# Each event is ~100-4000 bytes. 8192 is safe; increase to 16384 on busy hosts
--audit_backlog_limit=8192


# ─── Audit Subscribers (each enables a specific *_events table) ───────────────

# process_events — records execve()/execveat() syscalls
--audit_allow_process_events=true

# socket_events — records bind()/connect()/accept() syscalls
# WARNING: high volume on busy servers; tune events_max appropriately
--audit_allow_sockets=true

# user_events — records PAM/auth events (login, su, sudo)
--audit_allow_user_events=true

# file_events via Audit (process_file_events table) — records open/write
# on paths defined in file_paths conf section
--audit_allow_fim_events=true

# process_events gains SIGKILL/SIGTERM signal data when this is enabled
--audit_allow_kill_process_events=true

# Fork events (clone syscall) added to process_events
--audit_allow_fork_process_events=true

# apparmor_events — only relevant if AppArmor is loaded (Ubuntu/Debian)
# Has zero overhead if AppArmor is not in kernel; safe to leave enabled
--audit_allow_apparmor_events=true

# selinux_events — only relevant if SELinux is enforcing (RHEL/CentOS/Rocky)
# Has zero overhead if SELinux is not loaded; safe to leave enabled
--audit_allow_selinux_events=true

# seccomp_events — VERY HIGH VOLUME (Firefox, Chrome, systemd all use seccomp)
# Disabled by default. Enable only if you have specific seccomp analysis needs
# and have tuned events_max/expiry accordingly.
--audit_allow_seccomp_events=false

# Unix domain socket events — EXTREMELY HIGH VOLUME (basically everything)
# Leave disabled unless you are specifically analyzing IPC patterns
# Note: if you enable this, you MUST explicitly SELECT the 'socket' column
# (hidden by default)
# --audit_allow_unix=false  (hidden flag, commented out)


# ─────────────────────────── eBPF Events (kernel >= 4.18) ─────────────────────
# BPF provides a PARALLEL event stream to Audit.
# bpf_process_events and bpf_socket_events are SEPARATE tables from the
# audit-based process_events/socket_events. You can run BOTH simultaneously.
#
# BPF advantages over Audit:
#   - Captures more syscalls (execve, execveat, fork, vfork, clone, clone3)
#   - Includes cgroup ID for container awareness
#   - Captures syscall duration (nanoseconds)
#   - More resilient to event drops under load (perf ring buffer)
#   - Does NOT require disabling auditd if you only use BPF tables
#
# BPF disadvantages:
#   - Kernel >= 4.18 required (kernel >= 5.x strongly recommended)
#   - Higher memory usage (configurable via bpf_buffer_storage_size)
#   - Does NOT cover user_events, apparmor_events, selinux_events (those need Audit)
#
--enable_bpf_events=true

# BPF perf event array size as power-of-two (default: 10 = 1024 slots per CPU)
# Increase to 12 (4096 slots) on high-throughput hosts
# --bpf_perf_event_array_exp=10

# BPF per-CPU memory pool slots (each slot = 4096 bytes)
# Default is 512 slots. On VMs with CPU hotswap (VMware/KVM), use 64 to reduce RAM
# --bpf_buffer_storage_size=512


# ─────────────────────────── File Integrity Monitoring (inotify) ──────────────
# Enables the file_events table via Linux inotify
# Separate from process_file_events (which uses Audit)
--enable_file_events=true


# ─────────────────────────── Host Identity ────────────────────────────────────
# uuid uses DMTF/SMBIOS UUID — stable across reboots, unique per physical host
# Options: hostname | uuid | instance | ephemeral | specified
--host_identifier=uuid


# ─────────────────────────── Schedule Behaviour ───────────────────────────────
# Apply ±10% random splay to each query's interval at startup
--schedule_splay_percent=10
# Max allowed drift before splay is reset (seconds)
--schedule_max_drift=60
# Default interval for queries with no explicit interval set
--schedule_default_interval=3600


# ─────────────────────────── Docker Integration ───────────────────────────────
# Path to Docker UNIX socket. osqueryd user must have read access.
--docker_socket=/var/run/docker.sock


# ─────────────────────────── Augeas (config parsing) ──────────────────────────
--augeas_lenses=/opt/osquery/share/osquery/lenses


# ─────────────────────────── Performance Tweaks ───────────────────────────────
# Milliseconds to sleep between table calls in a JOIN (reduces CPU spikes)
--table_delay=0
# malloc_trim threshold (MB); frees glibc allocator cache to reduce RSS
# prevents watchdog from killing worker due to retained allocator memory
--malloc_trim_threshold=200
# Drop noisy UDEV partition events from hardware_events table
--hardware_disabled_types=partition
# Max file read size (50 MB default is fine for EDR)
--read_max=52428800
# File hash cache entries (per-inode, invalidated on inode change)
--hash_cache_max=500
# Milliseconds between hash() calls when scanning directories
--hash_delay=20
# Max column value length (increase if you see truncated cmdlines)
--value_max=512


# ─────────────────────────── Logging Metadata ─────────────────────────────────
--decorations_top_level=false
--verbose=false
```

---

## 6. The osquery.conf File

Save to `/etc/osquery/osquery.conf`. This contains the query schedule,
FIM paths, decorators, and references to packs.

```json
{
  "options": {
    "host_identifier": "uuid",
    "schedule_splay_percent": 10,
    "events_expiry": 3600,
    "events_max": 50000
  },

  "decorators": {
    "load": [
      "SELECT uuid AS host_uuid FROM system_info;",
      "SELECT hostname AS hostname FROM system_info;",
      "SELECT version AS osquery_version FROM osquery_info;",
      "SELECT version AS os_version, platform AS os_platform, codename AS os_codename FROM os_version;"
    ],
    "always": [
      "SELECT user AS active_user FROM logged_in_users WHERE user != '' AND type = 'user' ORDER BY time DESC LIMIT 1;"
    ],
    "interval": {
      "3600": [
        "SELECT total_seconds AS uptime_seconds FROM uptime;"
      ]
    }
  },

  "schedule": {

    "──── TIER 1: Real-Time Event Drain (30s) — EDR Core ────": {
      "query": "SELECT 1;", "interval": 9999999, "description": "section marker — ignore"
    },

    "bpf_process_events": {
      "query": "SELECT tid, pid, parent, uid, gid, cid, exit_code, probe_error, syscall, path, cwd, cmdline, duration, ntime, time FROM bpf_process_events;",
      "interval": 30,
      "removed": false,
      "description": "Process executions via eBPF (kernel >= 4.18). Columns: cid=cgroup_id for container awareness.",
      "platform": "linux",
      "denylist": false
    },

    "bpf_socket_events": {
      "query": "SELECT tid, pid, parent, uid, gid, cid, exit_code, probe_error, syscall, path, local_address, remote_address, local_port, remote_port, time FROM bpf_socket_events;",
      "interval": 30,
      "removed": false,
      "description": "Network connections via eBPF. Covers bind/connect/accept/accept4.",
      "platform": "linux",
      "denylist": false
    },

    "process_events_audit": {
      "query": "SELECT pid, ppid, uid, gid, euid, egid, auid, path, cmdline, cwd, time, syscall FROM process_events;",
      "interval": 30,
      "removed": false,
      "description": "Process executions via Linux Audit (fallback / parallel to BPF).",
      "platform": "linux",
      "denylist": false
    },

    "socket_events_audit": {
      "query": "SELECT pid, path, fd, auid, family, protocol, local_address, remote_address, local_port, remote_port, status, time FROM socket_events;",
      "interval": 30,
      "removed": false,
      "description": "Network socket events via Linux Audit.",
      "platform": "linux",
      "denylist": false
    },

    "user_events": {
      "query": "SELECT uid, auid, pid, message, type, path, time FROM user_events;",
      "interval": 30,
      "removed": false,
      "description": "PAM/auth events: login, logout, sudo, su, passwd changes.",
      "platform": "linux",
      "denylist": false
    },

    "apparmor_events": {
      "query": "SELECT time, uptime, eid, apparmor, operation, parent, profile, name, pid, denied_mask, capname, fsuid, ouid FROM apparmor_events;",
      "interval": 30,
      "removed": false,
      "description": "AppArmor LSM events (Debian/Ubuntu only; empty if AppArmor not loaded).",
      "platform": "linux",
      "denylist": false
    },

    "selinux_events": {
      "query": "SELECT time, uptime, scontext, tcontext, tclass, type, message FROM selinux_events;",
      "interval": 30,
      "removed": false,
      "description": "SELinux AVC events (RHEL/CentOS/Rocky only; empty if SELinux not loaded).",
      "platform": "linux",
      "denylist": false
    },

    "file_events_fim": {
      "query": "SELECT target_path, category, action, transaction_id, inode, uid, gid, time, md5, sha1, sha256 FROM file_events;",
      "interval": 60,
      "removed": false,
      "description": "inotify-based FIM events on paths defined in file_paths below.",
      "platform": "linux",
      "denylist": false
    },

    "process_file_events_audit": {
      "query": "SELECT operation, pid, ppid, uid, gid, auid, euid, egid, exe, path, time FROM process_file_events;",
      "interval": 60,
      "removed": false,
      "description": "File open/write events tied to process context via Audit (more reliable than pure inotify for EDR).",
      "platform": "linux",
      "denylist": false
    },

    "──── TIER 2: System State Differential (300s) ────": {
      "query": "SELECT 1;", "interval": 9999999, "description": "section marker — ignore"
    },

    "running_processes": {
      "query": "SELECT pid, ppid, name, path, cmdline, state, uid, gid, euid, egid, start_time, resident_size, user_time, system_time, disk_bytes_read, disk_bytes_written, on_disk FROM processes;",
      "interval": 300,
      "description": "Snapshot of all running processes. Differential catches new/exited processes."
    },

    "listening_ports": {
      "query": "SELECT pid, port, protocol, family, address, processes.name, processes.path, processes.cmdline FROM listening_ports LEFT JOIN processes USING (pid);",
      "interval": 300,
      "description": "Open listening ports with owning process."
    },

    "open_sockets": {
      "query": "SELECT pid, fd, socket, family, protocol, local_address, remote_address, local_port, remote_port, path, state FROM process_open_sockets WHERE family != 1;",
      "interval": 300,
      "description": "Open network sockets per process (excludes UNIX domain sockets for noise reduction)."
    },

    "logged_in_users": {
      "query": "SELECT liu.type, liu.user, liu.tty, liu.host, liu.time, liu.pid FROM logged_in_users liu;",
      "interval": 300,
      "description": "Currently logged-in users via utmp."
    },

    "process_open_files": {
      "query": "SELECT pid, fd, path FROM process_open_files WHERE path NOT LIKE '/proc/%' AND path NOT LIKE '/dev/%' ORDER BY pid;",
      "interval": 300,
      "description": "Open file descriptors per process (excluding proc/dev noise)."
    },

    "deleted_binaries": {
      "query": "SELECT pid, name, path, cmdline, uid, gid FROM processes WHERE on_disk = 0;",
      "interval": 300,
      "description": "Processes whose executable has been deleted from disk — classic attacker TTP.",
      "removed": false,
      "snapshot": true
    },

    "memory_mapped_executables": {
      "query": "SELECT pid, start, end, permissions, offset, path FROM process_memory_map WHERE permissions LIKE '%x%' AND path != '' AND path NOT LIKE '/usr/%' AND path NOT LIKE '/lib%';",
      "interval": 300,
      "description": "Executable memory regions outside standard system paths (shellcode / injected libs)."
    },

    "crontab_snapshot": {
      "query": "SELECT command, path, minute, hour, day_of_month, month, day_of_week FROM crontab;",
      "interval": 300,
      "snapshot": true,
      "description": "Full crontab snapshot every 5 minutes."
    },

    "docker_containers": {
      "query": "SELECT id, name, image, image_id, command, created, state, status FROM docker_containers;",
      "interval": 300,
      "description": "Running Docker containers. Requires Docker socket access."
    },

    "──── TIER 3: Configuration & Persistence Baseline (3600s) ────": {
      "query": "SELECT 1;", "interval": 9999999, "description": "section marker — ignore"
    },

    "users_snapshot": {
      "query": "SELECT uid, gid, username, description, directory, shell, type FROM users;",
      "interval": 3600,
      "snapshot": true,
      "description": "All local user accounts. Snapshot for baseline; differential for change detection."
    },

    "groups_snapshot": {
      "query": "SELECT gid, gid_signed, groupname FROM groups;",
      "interval": 3600,
      "snapshot": true,
      "description": "All local groups."
    },

    "sudoers": {
      "query": "SELECT header, rule_details FROM sudoers;",
      "interval": 3600,
      "description": "Sudoers configuration. Changes may indicate privilege escalation."
    },

    "authorized_keys": {
      "query": "SELECT users.username, authorized_keys.key, authorized_keys.key_type, authorized_keys.comment, authorized_keys.key_file FROM users JOIN authorized_keys USING (uid);",
      "interval": 3600,
      "description": "SSH authorized keys for all users. New keys = potential backdoor."
    },

    "user_ssh_keys": {
      "query": "SELECT uid, path, encrypted FROM user_ssh_keys;",
      "interval": 3600,
      "description": "Private SSH keys on disk. Unencrypted keys are a finding."
    },

    "kernel_modules": {
      "query": "SELECT name, size, used_by, status, address FROM kernel_modules;",
      "interval": 3600,
      "description": "Loaded kernel modules. Unexpected modules = potential rootkit."
    },

    "iptables_rules": {
      "query": "SELECT filter_name, chain, policy, target, protocol, src_ip, dst_ip, src_port, dst_port, match, packets, bytes FROM iptables;",
      "interval": 3600,
      "description": "iptables rules. EMPTY on systems using pure nftables — see table notes."
    },

    "deb_packages_snapshot": {
      "query": "SELECT name, version, source, arch, revision, status FROM deb_packages;",
      "interval": 3600,
      "snapshot": true,
      "platform": "linux",
      "description": "Installed Debian packages. Snapshot; differential for package install/remove events."
    },

    "rpm_packages_snapshot": {
      "query": "SELECT name, version, release, arch, epoch, install_time, vendor, package_group, source FROM rpm_packages;",
      "interval": 3600,
      "snapshot": true,
      "platform": "linux",
      "description": "Installed RPM packages."
    },

    "apt_sources": {
      "query": "SELECT name, source, base_uri, release, version, maintainer, type, components, architectures FROM apt_sources;",
      "interval": 3600,
      "description": "APT repository sources. Malicious repos = supply chain risk."
    },

    "yum_sources": {
      "query": "SELECT name, baseurl, enabled, gpgcheck FROM yum_sources;",
      "interval": 3600,
      "description": "YUM/DNF repository sources."
    },

    "suid_bins": {
      "query": "SELECT path, username, groupname, permissions, pid_with_namespace FROM suid_bin;",
      "interval": 3600,
      "description": "SUID/SGID binaries. New entries = privilege escalation vector."
    },

    "startup_items": {
      "query": "SELECT name, type, path, args, status FROM startup_items;",
      "interval": 3600,
      "description": "System startup items (systemd services, init scripts, cron)."
    },

    "systemd_units": {
      "query": "SELECT id, description, load_state, active_state, sub_state, following, object_path, job_id, job_type, job_object_path FROM systemd_units WHERE active_state = 'active';",
      "interval": 3600,
      "description": "Active systemd units."
    },

    "mounts": {
      "query": "SELECT device, device_alias, path, type, flags FROM mounts;",
      "interval": 3600,
      "description": "Mounted filesystems. Unexpected mounts = potential pivot."
    },

    "interface_details": {
      "query": "SELECT interface, mac, ip_mask, flags, type, last_change FROM interface_details;",
      "interval": 3600,
      "snapshot": true,
      "description": "Network interface details."
    },

    "routes": {
      "query": "SELECT destination, netmask, gateway, source, flags, interface, mtu, metric FROM routes;",
      "interval": 3600,
      "description": "Kernel routing table."
    },

    "apparmor_profiles": {
      "query": "SELECT path, name, attach, mode FROM apparmor_profiles;",
      "interval": 3600,
      "snapshot": true,
      "description": "AppArmor profiles. Changes may indicate LSM bypass attempts."
    },

    "selinux_settings": {
      "query": "SELECT scope, key, value FROM selinux_settings;",
      "interval": 3600,
      "snapshot": true,
      "description": "SELinux policy settings."
    },

    "hash_etc": {
      "query": "SELECT path, sha256 FROM hash WHERE path IN ('/etc/passwd', '/etc/shadow', '/etc/sudoers', '/etc/crontab', '/etc/hosts', '/etc/resolv.conf', '/etc/nsswitch.conf', '/etc/pam.conf', '/etc/ssh/sshd_config');",
      "interval": 3600,
      "description": "SHA-256 hashes of critical config files. Changes = potential tampering."
    },

    "osquery_health": {
      "query": "SELECT name, interval, executions, output_size, wall_time, (user_time/executions) AS avg_user_ms, (system_time/executions) AS avg_system_ms, average_memory, last_executed FROM osquery_schedule;",
      "interval": 3600,
      "snapshot": true,
      "description": "Performance stats per scheduled query. Use to detect expensive queries."
    },

    "──── TIER 4: Deep Audit Snapshots (86400s) ────": {
      "query": "SELECT 1;", "interval": 9999999, "description": "section marker — ignore"
    },

    "system_info_snapshot": {
      "query": "SELECT hostname, cpu_brand, cpu_physical_cores, cpu_logical_cores, physical_memory, hardware_vendor, hardware_model, hardware_serial, hardware_version, computer_name, local_hostname FROM system_info;",
      "interval": 86400,
      "snapshot": true,
      "description": "Hardware inventory snapshot."
    },

    "disk_info": {
      "query": "SELECT disk_index, type, id, partition_table_type, disk_size, hardware_model, name, serial, vendor FROM disk_info;",
      "interval": 86400,
      "snapshot": true,
      "description": "Physical disk inventory."
    },

    "os_version_snapshot": {
      "query": "SELECT name, version, major, minor, patch, build, platform, platform_like, codename FROM os_version;",
      "interval": 86400,
      "snapshot": true,
      "description": "OS version details."
    },

    "kernel_info": {
      "query": "SELECT version, arguments, path, device FROM kernel_info;",
      "interval": 86400,
      "snapshot": true,
      "description": "Kernel version and boot arguments."
    },

    "last_logins": {
      "query": "SELECT username, tty, pid, type, time, host FROM last;",
      "interval": 86400,
      "snapshot": true,
      "description": "Historical login records from /var/log/wtmp."
    },

    "shadow_hashes": {
      "query": "SELECT username, password_status, last_change, expire, flag FROM shadow;",
      "interval": 86400,
      "snapshot": true,
      "description": "Password hash status. Requires root. 'NP'=no password = critical finding."
    },

    "cpu_info": {
      "query": "SELECT number_of_cores, physical_cores, logical_cores, cpu_brand, cpu_type, cpu_subtype, cpu_brand_string, cpu_microcode FROM cpu_info;",
      "interval": 86400,
      "snapshot": true,
      "description": "CPU details."
    }
  },

  "file_paths": {
    "system_binaries": [
      "/usr/bin/%%",
      "/usr/sbin/%%",
      "/bin/%%",
      "/sbin/%%",
      "/usr/local/bin/%%"
    ],
    "etc_configs": [
      "/etc/%%"
    ],
    "ssh_keys": [
      "/root/.ssh/%%",
      "/home/%/.ssh/%%"
    ],
    "cron_dirs": [
      "/etc/cron.d/%%",
      "/etc/cron.daily/%%",
      "/etc/cron.hourly/%%",
      "/etc/cron.monthly/%%",
      "/etc/cron.weekly/%%",
      "/var/spool/cron/%%"
    ],
    "systemd_units": [
      "/etc/systemd/system/%%",
      "/usr/lib/systemd/system/%%",
      "/usr/local/lib/systemd/system/%%"
    ],
    "startup_scripts": [
      "/etc/init.d/%%",
      "/etc/rc.d/%%",
      "/etc/profile.d/%%",
      "/etc/ld.so.conf.d/%%"
    ],
    "tmp_exec": [
      "/tmp/%%",
      "/var/tmp/%%",
      "/dev/shm/%%"
    ]
  },

  "exclude_paths": {
    "etc_configs": [
      "/etc/mtab",
      "/etc/fstab",
      "/etc/sysfs.conf",
      "/etc/udev/rules.d/%%"
    ],
    "tmp_exec": [
      "/tmp/ssh-%%"
    ]
  },

  "file_accesses": [
    "ssh_keys",
    "etc_configs"
  ],

  "packs": {
    "edr-linux": "/etc/osquery/packs/edr-linux.conf",
    "hardware-monitoring": "/etc/osquery/packs/hardware-monitoring.conf"
  },

  "events": {
    "disable_subscribers": []
  }
}
```

> **Section marker queries** (`"query": "SELECT 1;"` with huge interval) are
> harmless — osquery runs them once then never again effectively. They are a
> readability trick and can be removed without impact.

---

## 7. Complete Linux Tables Reference

### 7.1 Always-Available Tables

These tables read from `/proc`, `/sys`, or make direct syscalls. They work
as a normal user (though some columns are root-only).

| Table | Data Source | Root Needed? | Notes |
|---|---|---|---|
| `processes` | `/proc/[pid]/status`, `/proc/[pid]/cmdline` | No (own procs) / Yes (all) | `on_disk=0` detects deleted binaries |
| `process_open_files` | `/proc/[pid]/fd/` | Root for all pids | |
| `process_open_sockets` | `/proc/[pid]/net/tcp*` | Root for all pids | |
| `process_memory_map` | `/proc/[pid]/maps` | Root for all pids | |
| `process_envs` | `/proc/[pid]/environ` | Root for all pids | |
| `users` | `/etc/passwd`, `getpwent()` | No | |
| `groups` | `/etc/group`, `getgrent()` | No | |
| `logged_in_users` | `/var/run/utmp` | No | |
| `last` | `/var/log/wtmp` | No | |
| `crontab` | `/etc/crontab`, `/var/spool/cron/` | Root for all users | |
| `listening_ports` | `/proc/net/tcp`, `/proc/net/tcp6`, etc. | No | |
| `interface_addresses` | `getifaddrs()` | No | |
| `interface_details` | `ioctl(SIOCGIFCONF)` | No | |
| `routes` | `/proc/net/route`, `netlink` | No | |
| `arp_cache` | `/proc/net/arp` | No | |
| `dns_resolvers` | `/etc/resolv.conf`, `nsswitch.conf` | No | |
| `mounts` | `/proc/mounts` | No | |
| `disk_info` | `/sys/block/*/` | No | |
| `block_devices` | `/sys/block/*/` | No | |
| `system_info` | `sysinfo()`, `/proc/cpuinfo`, DMI | No | |
| `cpu_info` | `/proc/cpuinfo` | No | |
| `memory_info` | `/proc/meminfo` | No | |
| `os_version` | `/etc/os-release` | No | |
| `kernel_info` | `/proc/version`, `/proc/cmdline` | No | |
| `uptime` | `sysinfo()` | No | |
| `time` | `gettimeofday()` | No | |
| `hostname` | `gethostname()` | No | |
| `environment` | `/proc/[pid]/environ` | Root | |
| `file` | `stat(2)` | No (world-readable paths) | |
| `hash` | `open(2)`, `read(2)` | No (world-readable paths) | |
| `suid_bin` | `nftw()` traversal of PATH dirs | No | Scans slowly; cache results |
| `authorized_keys` | `~/.ssh/authorized_keys` | Root for all users | |
| `user_ssh_keys` | `~/.ssh/id_*` (private keys) | Root for all users | |
| `startup_items` | systemd, initd, cron | No | |
| `systemd_units` | D-Bus / systemd socket | No | |
| `process_namespaces` | `/proc/[pid]/ns/` | Root | |
| `osquery_info` | osquery internals | No | |
| `osquery_flags` | osquery internals | No | |
| `osquery_schedule` | osquery internals | No | |
| `osquery_events` | osquery internals | No | |

### 7.2 Tables That Require Root

| Table | Why Root | Failure Mode |
|---|---|---|
| `shadow` | `/etc/shadow` is `root:shadow 640` | Empty result set, no error |
| `process_envs` | `/proc/[pid]/environ` owned by process owner | Own process only |
| `memory_info` | Some columns | Most columns work without root |
| `kernel_keys` | `/proc/keys` requires `CAP_SYS_ADMIN` or owner | Empty |
| `hardware_events` | udev netlink requires root | Empty without root |
| `acpi_tables` | `/sys/firmware/acpi/tables` | Empty without root |

### 7.3 Audit-Based Event Tables

These tables require:
1. `--disable_events=false`
2. `--disable_audit=false`
3. The corresponding `--audit_allow_*=true` flag
4. **auditd must NOT be running**

| Table | Flag Required | Additional Notes |
|---|---|---|
| `process_events` | `--audit_allow_process_events=true` | Records `execve`, `execveat` |
| `socket_events` | `--audit_allow_sockets=true` | Records `bind`, `connect`, `accept` |
| `process_file_events` | `--audit_allow_fim_events=true` + `file_paths` in conf | Records `open`, `write` on watched paths |
| `user_events` | `--audit_allow_user_events=true` | PAM/auth events |
| `apparmor_events` | `--audit_allow_apparmor_events=true` + AppArmor in kernel | Ubuntu/Debian |
| `selinux_events` | `--audit_allow_selinux_events=true` + SELinux enforcing | RHEL/CentOS/Rocky |

**Critical internal detail**: osquery opens `/proc/net/netlink` and binds to
`NETLINK_AUDIT` (family 9). It then calls `audit_set_pid()` to register itself
as the sole consumer. The kernel then forwards all audit records directly to
osquery's socket. There is no file or pipe involved — it is a raw kernel
netlink multicast, and the "single consumer" rule is enforced in
`kernel/audit.c:audit_set_pid()`.

### 7.4 eBPF Event Tables

Require kernel >= 4.18 and `--enable_bpf_events=true`:

| Table | Syscalls Instrumented | Extra Columns vs Audit |
|---|---|---|
| `bpf_process_events` | execve, execveat, fork, vfork, clone, clone3 | `cid` (cgroup), `duration` (nsec), `json_cmdline` (hidden) |
| `bpf_socket_events` | bind, connect, accept, accept4 | `cid` (cgroup), `duration` (nsec) |

**How BPF events work internally**: osquery loads a compiled eBPF program via
`bpf()` syscall. The program attaches kprobes (or tracepoints) to the target
syscall entry/exit points. On each syscall, the BPF program reads arguments
from the kernel stack (using `bpf_probe_read_kernel`), builds an event struct,
and writes it to a **per-CPU perf event ring buffer** (via `perf_event_output`).
The osquery user-space thread polls the ring buffer via `epoll_wait()` and
converts raw structs into table rows.

**Memory usage formula**:
```
perf_bytes = (2 ^ bpf_perf_event_array_exp) * online_cpu_count * 4096
pool_bytes = 6 * bpf_buffer_storage_size * 4096 * possible_cpu_count
```

On a 4-core physical machine with `bpf_perf_event_array_exp=10` and
`bpf_buffer_storage_size=512`: `(1024 * 4 * 4096) + (6 * 512 * 4096 * 4)`
≈ 16 MB + 50 MB = ~66 MB extra RSS. On VMware with 128 possible CPUs:
the pool blows up to ~1.5 GB. Fix: `--bpf_buffer_storage_size=64`.

### 7.5 FIM / inotify Tables

Requires `--enable_file_events=true` + `file_paths` in conf:

| Table | Mechanism | Notes |
|---|---|---|
| `file_events` | Linux inotify | Real-time; limited by `max_user_watches` |
| `process_file_events` | Linux Audit + file_paths | Process context on open/write; needs `--audit_allow_fim_events=true` |

**Why `file_events` returns empty**: The most common cause is that `file_paths`
in your conf is empty or misconfigured. The second most common cause is hitting
the inotify watch limit (`max_user_watches`). inotify places one watch per
directory. If you watch `/etc/%%`, osquery creates one watch per subdirectory
under `/etc`, not just one for the whole tree. A large recursive path (like
`/usr/%%`) can easily exhaust 8192 default watches.

### 7.6 Package Manager Tables

| Table | Required | Empty When |
|---|---|---|
| `deb_packages` | dpkg database | Non-Debian system / dpkg not installed |
| `deb_package_files` | dpkg database | Same as above |
| `apt_sources` | `/etc/apt/sources.list` | Non-Debian |
| `rpm_packages` | RPM database (`rpmdb`) | Non-RPM system |
| `rpm_package_files` | RPM database | Non-RPM system |
| `yum_sources` | `/etc/yum.repos.d/` | Non-RPM system |

**Debian/Ubuntu:** `deb_packages` reads the dpkg status file at
`/var/lib/dpkg/status`. On Ubuntu minimal images, this file may be partial.

**RHEL/RPM:** `rpm_packages` uses `librpm` internally (linked at compile time
in the official package). The RPM database is at `/var/lib/rpm/`. If it is
corrupted (common after failed updates), `rpm --rebuilddb` fixes it.

### 7.7 Docker / Container Tables

These tables connect to the Docker daemon via UNIX socket.

| Table | Requirement |
|---|---|
| `docker_containers` | Docker socket at `--docker_socket` |
| `docker_container_processes` | Docker socket + container running |
| `docker_container_stats` | Docker socket + container running |
| `docker_container_ports` | Docker socket |
| `docker_container_labels` | Docker socket |
| `docker_container_networks` | Docker socket |
| `docker_container_mounts` | Docker socket |
| `docker_images` | Docker socket |
| `docker_networks` | Docker socket |
| `docker_volumes` | Docker socket |
| `docker_info` | Docker socket |

**Why empty**: The osqueryd process must have read permission on the Docker
socket. Default socket permissions are `srw-rw----` owned by `root:docker`.
Either run osqueryd as root (recommended for EDR) or add the osquery user
to the `docker` group.

```bash
usermod -aG docker osquery
# OR run osqueryd as root (EDR standard)
```

### 7.8 LXD Tables

| Table | Requirement |
|---|---|
| `lxd_certificates` | LXD daemon running |
| `lxd_cluster` | LXD in cluster mode |
| `lxd_cluster_members` | LXD in cluster mode |
| `lxd_containers` | LXD daemon running |
| `lxd_images` | LXD daemon running |
| `lxd_instance_config` | LXD daemon running |
| `lxd_instance_devices` | LXD daemon running |
| `lxd_networks` | LXD daemon running |
| `lxd_storage_pools` | LXD daemon running |

**Why empty**: LXD uses a UNIX socket at `/var/snap/lxd/common/lxd/unix.socket`
(snap install) or `/var/lib/lxd/unix.socket` (package install). osquery tries
both paths. If LXD is not installed, these tables are always empty — no error.

### 7.9 Networking Tables and iptables vs nftables

> **This is the most common source of confusion in modern Linux EDR deployments.**

The `iptables` table reads `/proc/net/ip_tables_names` to discover which tables
are loaded. This proc file **does not exist** when the kernel is using the
`nf_tables` subsystem (modern netfilter). It only exists when the legacy
`xt_tables` / `ip_tables` kernel module is loaded.

**Affected distros (iptables table will be empty)**:
- Ubuntu 21.04+ (UFW now uses nftables backend by default)
- Debian 11 (Bullseye)+
- RHEL/CentOS/Rocky 9+
- Fedora 33+

**Fix options**:

```bash
# Option A: Load the legacy iptables kernel module
modprobe ip_tables ip6_tables
# Make permanent:
echo "ip_tables" >> /etc/modules-load.d/osquery.conf
echo "ip6_tables" >> /etc/modules-load.d/osquery.conf

# Option B: Switch iptables binary to legacy mode (Debian/Ubuntu)
update-alternatives --set iptables /usr/sbin/iptables-legacy
update-alternatives --set ip6tables /usr/sbin/ip6tables-legacy

# Verify the proc file appears:
cat /proc/net/ip_tables_names
# Should output: filter  nat  mangle  raw  security
```

For nftables-native environments, use Automatic Table Construction (ATC) to
parse `nft list ruleset --json` output, or build a custom extension.

### 7.10 LSM Tables — AppArmor & SELinux

**AppArmor** (Ubuntu/Debian):
```bash
# Verify AppArmor is in kernel
aa-status
cat /sys/module/apparmor/parameters/enabled  # should print "Y"

# Verify audit is emitting AppArmor events
journalctl -k | grep -i apparmor | tail -5
# OR
ausearch -m AVC 2>/dev/null | tail -5
```

**SELinux** (RHEL/CentOS/Rocky):
```bash
getenforce   # Must be "Enforcing" or "Permissive" for events to flow
sestatus
```

**Common mistake**: Enabling `--audit_allow_apparmor_events=true` on a RHEL
system that has SELinux. The flag is harmless but the table stays empty because
there is no AppArmor module in the kernel. Similarly, enabling
`--audit_allow_selinux_events=true` on Ubuntu produces no rows.

### 7.11 YARA Tables

| Table | Requirement |
|---|---|
| `yara` | libyara, signature files defined in conf `yara` section |
| `yara_events` | `disable_events=false` + FIM enabled + yara conf section |

osquery ships with libyara statically linked. You do NOT need to install the
system yara package. However, you must define signature paths in your conf:

```json
{
  "yara": {
    "signatures": {
      "malware_sigs": ["/etc/osquery/yara/malware.yar"]
    },
    "file_paths": {
      "tmp_exec": ["malware_sigs"]
    }
  }
}
```

The `yara_events` subscriber fires whenever `file_events` triggers on a path
covered by both `file_paths` and the YARA signature mapping.

---

## 8. Why a Table Returns Empty Results

This is the most important diagnostic section. Senior engineers memorize these.

| Symptom | Root Cause | Fix |
|---|---|---|
| `process_events` empty | `auditd` is running | `systemctl stop auditd && systemctl disable auditd` |
| `process_events` empty | `--disable_audit=false` not set | Add to flags file |
| `process_events` empty | `--disable_events=false` not set | Add to flags file |
| `process_events` empty | `--audit_allow_process_events=true` not set | Add to flags file |
| `bpf_process_events` empty | Kernel < 4.18 | Upgrade kernel or use audit tables |
| `bpf_process_events` empty | `--enable_bpf_events=true` not set | Add to flags file |
| `bpf_process_events` empty | `CONFIG_BPF_SYSCALL` not in kernel | Check `grep CONFIG_BPF_SYSCALL /boot/config-$(uname -r)` |
| `file_events` empty | `file_paths` not defined in conf | Add `file_paths` section |
| `file_events` empty | `--enable_file_events=true` not set | Add to flags file |
| `file_events` empty | inotify watch limit exhausted | Increase `fs.inotify.max_user_watches` |
| `iptables` empty | Host uses nftables (no `/proc/net/ip_tables_names`) | Load `ip_tables` module |
| `deb_packages` empty | RHEL/RPM host | Normal; use `rpm_packages` |
| `rpm_packages` empty | Debian/Ubuntu host | Normal; use `deb_packages` |
| `docker_*` empty | Docker socket not accessible | Run osqueryd as root or add to docker group |
| `apparmor_events` empty | AppArmor not loaded | Check `aa-status`; only Ubuntu/Debian |
| `selinux_events` empty | SELinux not loaded/enforcing | Check `getenforce`; only RHEL/CentOS |
| `shadow` empty | Not running as root | Run osqueryd as root |
| `lxd_*` empty | LXD not installed | Install LXD or ignore |
| `process_file_events` empty | `--audit_allow_fim_events=true` missing | Add to flags file |
| `process_file_events` empty | No `file_paths` in conf | Add `file_paths` section |
| `kernel_modules` empty | Not running as root | Run as root (always for EDR) |
| `socket_events` empty | `--audit_allow_sockets=true` not set | Add to flags file |
| `smart_drive_info` empty | smartmontools not installed / disk no S.M.A.R.T | `apt install smartmontools` |
| `user_events` empty | `--audit_allow_user_events=true` not set | Add to flags file |
| Any `*_events` table empty | `--disable_events=false` not set | **Master toggle** — must be set |
| Event table returns old data | `events_optimize=true` means only NEW events returned | This is correct behaviour for EDR |
| Event table never has rows | Scheduled query interval too long vs `events_expiry` | Set `interval` < `events_expiry` |
| High CPU on event drain | `events_max` too large; large batch processed at once | Reduce `events_max` or drain more frequently |

---

## 9. Audit Events: Deep Setup

### Architecture

```
Kernel Syscall → audit_log_exit() → kernel ring buffer → audit kthread
    → netlink socket (NETLINK_AUDIT) → osqueryd parser
    → RocksDB subscriber queue → SELECT drains rows
```

### Pre-flight Checks

```bash
# Confirm auditd is gone
systemctl status auditd
pgrep auditd && echo "PROBLEM: auditd running" || echo "OK"

# Check current audit status after osqueryd starts
auditctl -s 2>/dev/null
# Look for: enabled 2, pid <osqueryd_pid>
# enabled=1 means auditd was running; enabled=2 means osquery took control

# Check what audit rules osquery loaded
auditctl -l
# Should see rules for execve, bind, connect depending on flags set
```

### Performance Tuning for Audit

The audit backlog is the kernel-side ring buffer between syscall execution and
the netlink consumer (osquery). When the backlog overflows, events are dropped
silently. You can see drop counts:

```bash
auditctl -s | grep -E "backlog|lost|enabled|failure"
```

**Tuning rules**:
- Start with `--audit_backlog_limit=8192`
- On high-throughput servers (>1000 processes/sec): increase to `16384` or `32768`
- Watch `lost` counter — if it grows, increase the backlog limit
- Keep in mind: the backlog lives in non-swappable kernel memory.
  Each slot is ~4 KB, so 8192 slots ≈ 32 MB kernel RAM.

### Conflict with journald

journald also reads the audit netlink socket by default on systemd systems.
This creates two readers competing for the socket, leading to lost events:

```bash
# Permanent fix: mask the journald audit socket unit
systemctl mask --now systemd-journald-audit.socket

# Verify:
systemctl status systemd-journald-audit.socket
# Should show: loaded but masked
```

---

## 10. eBPF Events: Deep Setup

### Kernel Requirements

```bash
# Minimum: kernel >= 4.18
# Strongly recommended: kernel >= 5.4 (stable BPF JIT, BTF support)
# Best: kernel >= 5.10 LTS (full BPF ringbuf, improved tracepoints)

# Check kernel BPF features:
cat /boot/config-$(uname -r) | grep -E \
  "CONFIG_BPF=|CONFIG_BPF_SYSCALL=|CONFIG_BPF_JIT=|CONFIG_PERF_EVENTS=|CONFIG_KPROBES=|CONFIG_HAVE_KPROBES=|CONFIG_BPF_EVENTS="

# All of the following should be "=y":
# CONFIG_BPF=y
# CONFIG_BPF_SYSCALL=y
# CONFIG_BPF_JIT=y
# CONFIG_PERF_EVENTS=y
# CONFIG_KPROBES=y
# CONFIG_BPF_EVENTS=y (or CONFIG_FTRACE_SYSCALLS=y)
```

### Memory Calculation for BPF

On VMware or KVM with CPU hotswap enabled, `possible_cpu_count` may be 128:

```bash
cat /sys/devices/system/cpu/possible    # e.g. "0-127"
cat /sys/devices/system/cpu/online      # e.g. "0-7"
```

If `possible >> online`, use a small `bpf_buffer_storage_size`:

```ini
# In osquery.flags, add these for VMs:
--bpf_buffer_storage_size=64
--bpf_perf_event_array_exp=10
```

For physical bare-metal servers where `possible == online`:

```ini
--bpf_buffer_storage_size=512
--bpf_perf_event_array_exp=12
```

### Running BPF and Audit Together

You CAN and SHOULD run both for maximum coverage:

- `bpf_process_events` + `bpf_socket_events` → via eBPF (more data, more reliable)
- `process_file_events` + `user_events` + `apparmor_events` + `selinux_events` → via Audit

The flags file already handles this: both `--enable_bpf_events=true` and
`--disable_audit=false` are set simultaneously. The two publishers are
completely independent and do not conflict.

---

## 11. File Integrity Monitoring Setup

### inotify FIM (`file_events` table)

inotify watches are placed on directories, not files. When watching `/etc/%%`,
osquery places one inotify watch on every subdirectory under `/etc/` recursively.
Each watch consumes ~340 bytes of non-swappable kernel memory.

```bash
# Check current watch usage
cat /proc/sys/fs/inotify/max_user_watches
# Increase permanently (already done via sysctl in Section 4)
sysctl -w fs.inotify.max_user_watches=524288
```

**Limitation**: inotify does NOT watch for new directories created after osquery
starts. If `/tmp/attacker/` is created after osquery starts watching `/tmp/%%`,
the new directory is not watched until osquery is restarted.

### Audit FIM (`process_file_events` table)

This is more powerful for EDR because it:
1. Captures the PID and process context of the access (not just the file change)
2. Does not suffer from the "new directory" limitation (Audit watches are syscall-level)
3. Has no inotify watch count limitation

The tradeoff: it generates significantly more events on busy paths.

### Recommended FIM Strategy for EDR

Use BOTH:
- `file_events` (inotify) for `system_binaries`, `etc_configs`, `ssh_keys`, `systemd_units`
- `process_file_events` (audit) for `etc_configs` and `ssh_keys` to get process context

The conf in Section 6 already sets this up correctly.

---

## 12. Extensions Setup

Extensions are separate processes that communicate with osqueryd over a Thrift
UNIX domain socket (`--extensions_socket`). They register additional virtual tables.

### Extension File Conventions

```bash
# Extensions must have the .ext filename extension
# They must be owned by root with permissions 0700 or 0500
# (osquery refuses to load world-writable or group-writable extensions)

# Example: custom EDR table written in Go
chmod 0700 /usr/lib/osquery/extensions/my_edr.ext
chown root:root /usr/lib/osquery/extensions/my_edr.ext

# Add to extensions.load (one path per line)
echo "/usr/lib/osquery/extensions/my_edr.ext" >> /etc/osquery/extensions.load
```

### Writing an Extension in Python

```bash
pip3 install osquery
```

```python
#!/usr/bin/env python3
# /usr/lib/osquery/extensions/custom_edr.py
import osquery

@osquery.register_plugin
class KernelKeyringTable(osquery.TablePlugin):
    """Exposes /proc/keys as a queryable table."""

    def name(self):
        return "kernel_keyring_extended"

    def columns(self):
        return [
            osquery.TableColumn(name="serial", type=osquery.STRING),
            osquery.TableColumn(name="flags", type=osquery.STRING),
            osquery.TableColumn(name="usage", type=osquery.INTEGER),
            osquery.TableColumn(name="expiry", type=osquery.STRING),
            osquery.TableColumn(name="permissions", type=osquery.STRING),
            osquery.TableColumn(name="uid", type=osquery.INTEGER),
            osquery.TableColumn(name="gid", type=osquery.INTEGER),
            osquery.TableColumn(name="type", type=osquery.STRING),
            osquery.TableColumn(name="description", type=osquery.STRING),
        ]

    def generate(self, context):
        rows = []
        try:
            with open("/proc/keys", "r") as f:
                for line in f:
                    parts = line.strip().split(None, 8)
                    if len(parts) >= 9:
                        rows.append({
                            "serial": parts[0],
                            "flags": parts[1],
                            "usage": parts[2],
                            "expiry": parts[3],
                            "permissions": parts[4],
                            "uid": parts[5],
                            "gid": parts[6],
                            "type": parts[7],
                            "description": parts[8],
                        })
        except Exception:
            pass
        return rows

if __name__ == "__main__":
    osquery.start_extension(name="custom_edr", version="1.0.0")
```

```bash
chmod +x /usr/lib/osquery/extensions/custom_edr.py
ln -s /usr/lib/osquery/extensions/custom_edr.py \
      /usr/lib/osquery/extensions/custom_edr.ext
echo "/usr/lib/osquery/extensions/custom_edr.ext" >> /etc/osquery/extensions.load
```

### Writing an Extension in Go

```bash
go get github.com/osquery/osquery-go
```

```go
// main.go
package main

import (
    "context"
    "log"
    "os"
    "time"

    "github.com/osquery/osquery-go"
    "github.com/osquery/osquery-go/plugin/table"
)

func main() {
    flServerTimeout := osquery.ServerTimeout(2 * time.Second)
    server, err := osquery.NewExtensionManagerServer(
        "custom_linux_edr",
        os.Args[1],  // socket path passed by osquery
        flServerTimeout,
    )
    if err != nil {
        log.Fatal("Error creating extension manager:", err)
    }
    server.RegisterPlugin(table.NewPlugin(
        "custom_process_namespaces",
        []table.ColumnDefinition{
            table.BigIntColumn("pid"),
            table.TextColumn("ns_type"),
            table.BigIntColumn("inode"),
        },
        generateNamespaces,
    ))
    if err := server.Run(); err != nil {
        log.Fatal(err)
    }
}

func generateNamespaces(ctx context.Context,
    queryContext table.QueryContext) ([]map[string]string, error) {
    // Implementation: read /proc/[pid]/ns/*
    return []map[string]string{}, nil
}
```

### Community Extensions Worth Deploying

| Extension | Language | What It Adds | URL |
|---|---|---|---|
| `osquery-go` SDK | Go | SDK for writing Go extensions | github.com/osquery/osquery-go |
| `osquery-python` SDK | Python | SDK for Python extensions | github.com/osquery/osquery-python |
| `osquery-rust-ng` | Rust | Rust SDK with config+logger plugins | crates.io/crates/osquery-rust-ng |
| Polylogy Extensions | C++ | DNS events, HTTP events, more Linux tables | github.com/polylogyx/osq-ext-bin |

---

## 13. Systemd Service Configuration

```bash
cat > /etc/systemd/system/osqueryd.service << 'EOF'
[Unit]
Description=osquery daemon
Documentation=https://osquery.readthedocs.io
After=network.target auditd.service
# Explicitly conflict with auditd — if auditd is somehow started after us,
# we want osqueryd to be restarted (it will re-claim the audit socket via
# --audit_persist=true)
Conflicts=auditd.service

[Service]
Type=simple
# Run as root — required for: shadow, kernel_modules, BPF, audit netlink
User=root
Group=root
ExecStartPre=/bin/sh -c 'systemctl is-active auditd && systemctl stop auditd || true'
ExecStart=/opt/osquery/bin/osqueryd \
  --flagfile /etc/osquery/osquery.flags \
  --config_path /etc/osquery/osquery.conf
ExecReload=/bin/kill -HUP $MAINPID
# Restart on any failure, with a 5s delay
Restart=on-failure
RestartSec=5
# Don't restart more than 3 times in 30s (crash loop protection)
StartLimitBurst=3
StartLimitInterval=30s
# Kill both watchdog and worker processes
KillMode=process
TimeoutStopSec=15
# Security hardening (compatible with root execution)
NoNewPrivileges=false
ProtectSystem=false
PrivateTmp=false
# Allow access to /proc (required for many tables)
ProcSubset=all
# Resource limits
LimitNOFILE=65536
# Log to journal with this identifier
SyslogIdentifier=osqueryd

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable osqueryd
systemctl start osqueryd

# Watch startup for errors:
journalctl -u osqueryd -f --no-hostname
```

---

## 14. Scheduled Queries — EDR Pack Design

### EDR Linux Pack: `/etc/osquery/packs/edr-linux.conf`

```json
{
  "queries": {

    "shell_in_unusual_dir": {
      "query": "SELECT pid, ppid, name, path, cmdline, uid, gid FROM processes WHERE (path LIKE '/tmp/%' OR path LIKE '/dev/shm/%' OR path LIKE '/run/%' OR path LIKE '/var/tmp/%') AND name IN ('sh', 'bash', 'zsh', 'fish', 'dash', 'ksh');",
      "interval": 60,
      "description": "Shells executing from non-standard directories — common post-exploitation pattern.",
      "platform": "linux"
    },

    "process_with_deleted_binary": {
      "query": "SELECT pid, name, path, cmdline, uid, gid, start_time FROM processes WHERE on_disk = 0;",
      "interval": 120,
      "snapshot": true,
      "description": "Processes whose executable has been deleted from disk (fileless malware indicator).",
      "platform": "linux"
    },

    "ptrace_activity": {
      "query": "SELECT p1.pid AS tracer_pid, p1.name AS tracer_name, p1.cmdline AS tracer_cmd, p2.pid AS tracee_pid, p2.name AS tracee_name FROM processes p1 JOIN processes p2 ON p1.pid = p2.parent WHERE p2.state = 'T';",
      "interval": 60,
      "description": "Processes being traced (ptrace/gdb) — potential memory dumping or injection.",
      "platform": "linux"
    },

    "listening_on_all_interfaces": {
      "query": "SELECT lp.pid, lp.port, lp.protocol, lp.address, p.name, p.path, p.cmdline, p.uid FROM listening_ports lp JOIN processes p USING (pid) WHERE lp.address = '0.0.0.0' OR lp.address = '::';",
      "interval": 300,
      "description": "Processes listening on all interfaces — may indicate C2 backdoor.",
      "platform": "linux"
    },

    "suid_writable": {
      "query": "SELECT DISTINCT sb.path, sb.username, sb.groupname, sb.permissions, f.size FROM suid_bin sb JOIN file f ON sb.path = f.path WHERE f.writable = 1;",
      "interval": 3600,
      "description": "SUID binaries that are world-writable — privilege escalation vector.",
      "platform": "linux"
    },

    "new_users_since_baseline": {
      "query": "SELECT uid, gid, username, description, directory, shell FROM users WHERE uid >= 1000 AND shell NOT IN ('/sbin/nologin', '/bin/false', '/usr/sbin/nologin');",
      "interval": 3600,
      "description": "Interactive user accounts — unexpected entries = account creation backdoor.",
      "platform": "linux"
    },

    "uid_0_accounts": {
      "query": "SELECT username, description, directory, shell FROM users WHERE uid = 0;",
      "interval": 3600,
      "description": "Accounts with UID 0 (root-equivalent). Should ONLY be 'root'.",
      "platform": "linux"
    },

    "world_writable_dirs_in_path": {
      "query": "SELECT directory, path FROM file WHERE directory IN (SELECT DISTINCT substring(value, 0, instr(value||':', ':')) FROM process_envs WHERE key='PATH' AND pid=1) AND writable = 1 AND type='directory';",
      "interval": 3600,
      "description": "World-writable directories in $PATH — PATH hijacking vector.",
      "platform": "linux"
    },

    "kernel_module_unsigned": {
      "query": "SELECT name, address, used_by FROM kernel_modules WHERE status != 'Live' OR name NOT IN (SELECT name FROM kernel_modules WHERE status = 'Live');",
      "interval": 3600,
      "description": "Kernel modules not in Live state (unusual load state).",
      "platform": "linux"
    },

    "etc_hosts_changes": {
      "query": "SELECT * FROM etc_hosts;",
      "interval": 3600,
      "description": "Contents of /etc/hosts. Malicious entries can redirect DNS lookups.",
      "platform": "linux"
    },

    "unexpected_network_listeners_snapshot": {
      "query": "SELECT lp.pid, lp.port, lp.protocol, lp.address, lp.socket, p.name, p.path FROM listening_ports lp JOIN processes p USING (pid) WHERE lp.port NOT IN (22, 80, 443, 8080, 8443, 9090) ORDER BY lp.port;",
      "interval": 3600,
      "snapshot": true,
      "description": "Snapshot of non-standard ports with owning process.",
      "platform": "linux"
    },

    "crontab_unusual": {
      "query": "SELECT command, path, minute, hour FROM crontab WHERE command LIKE '%curl%' OR command LIKE '%wget%' OR command LIKE '%nc %' OR command LIKE '%bash -i%' OR command LIKE '%/tmp/%' OR command LIKE '%python%' OR command LIKE '%perl%';",
      "interval": 3600,
      "description": "Crontab entries with network or shell patterns.",
      "platform": "linux"
    },

    "ssh_authorized_keys_snapshot": {
      "query": "SELECT users.username, users.uid, authorized_keys.key, authorized_keys.key_type, authorized_keys.comment, authorized_keys.key_file FROM users JOIN authorized_keys USING (uid);",
      "interval": 3600,
      "snapshot": true,
      "description": "Full snapshot of all SSH authorized keys. New keys = potential backdoor.",
      "platform": "linux"
    },

    "environment_ld_preload": {
      "query": "SELECT pid, key, value, processes.name, processes.path FROM process_envs JOIN processes USING (pid) WHERE key IN ('LD_PRELOAD', 'LD_LIBRARY_PATH', 'LD_AUDIT', 'DYLD_INSERT_LIBRARIES');",
      "interval": 300,
      "description": "Processes using LD_PRELOAD or similar — dynamic linker hijacking indicator.",
      "platform": "linux"
    },

    "docker_privileged_containers": {
      "query": "SELECT dc.id, dc.name, dc.image, dc.status, dcs.privileged FROM docker_containers dc JOIN docker_container_security_options dcs ON dc.id = dcs.id WHERE dcs.privileged = 1;",
      "interval": 3600,
      "description": "Privileged Docker containers — container escape risk.",
      "platform": "linux"
    },

    "namespace_escapes": {
      "query": "SELECT p.pid, p.name, p.cmdline, pn.host_ipc, pn.host_net, pn.host_pid, pn.host_uts FROM processes p JOIN process_namespaces pn USING (pid) WHERE pn.host_pid = 1 OR pn.host_net = 1 OR pn.host_ipc = 1;",
      "interval": 300,
      "description": "Processes sharing host namespaces (potential container escapes).",
      "platform": "linux"
    },

    "memory_stats": {
      "query": "SELECT memory_total, memory_free, memory_available, buffers, cached, swap_total, swap_free, swap_cached FROM memory_info;",
      "interval": 300,
      "snapshot": true,
      "description": "Memory utilization snapshot.",
      "platform": "linux"
    },

    "recent_logins": {
      "query": "SELECT username, tty, pid, type, time, host FROM last WHERE time > (strftime('%s', 'now') - 86400) ORDER BY time DESC;",
      "interval": 3600,
      "snapshot": true,
      "description": "Logins within the last 24 hours.",
      "platform": "linux"
    }
  }
}
```

### Hardware Monitoring Pack: `/etc/osquery/packs/hardware-monitoring.conf`

```json
{
  "queries": {
    "hardware_events_usb": {
      "query": "SELECT time, action, model, serial, vendor_id, model_id, path FROM hardware_events WHERE subsystem = 'usb';",
      "interval": 30,
      "removed": false,
      "description": "USB device attach/detach events.",
      "platform": "linux"
    },
    "usb_devices": {
      "query": "SELECT usb_address, usb_port, vendor_id, model_id, model, serial, subclass, protocol, removable FROM usb_devices;",
      "interval": 300,
      "description": "Currently attached USB devices.",
      "platform": "linux"
    },
    "smart_drive_info": {
      "query": "SELECT device, driver, model, serial_number, firmware_version, healthy, self_assessment FROM smart_drive_info;",
      "interval": 3600,
      "snapshot": true,
      "description": "SMART disk health. Requires smartmontools.",
      "platform": "linux"
    }
  }
}
```

---

## 15. Log Format & Processing

osquery writes two types of log to `/var/log/osquery/`:

### `osqueryd.results.log` — Query Results

```json
{
  "name": "bpf_process_events",
  "hostIdentifier": "550e8400-e29b-41d4-a716-446655440000",
  "calendarTime": "Thu Jun  4 12:00:00 2026 UTC",
  "unixTime": 1748996400,
  "epoch": 0,
  "counter": 0,
  "numerics": false,
  "decorations": {
    "host_uuid": "550e8400-e29b-41d4-a716-446655440000",
    "hostname": "prod-server-01",
    "osquery_version": "5.14.0",
    "os_version": "22.04",
    "os_platform": "ubuntu",
    "active_user": "root"
  },
  "action": "added",
  "columns": {
    "tid": "12345",
    "pid": "12345",
    "parent": "1",
    "uid": "0",
    "gid": "0",
    "cid": "0",
    "exit_code": "0",
    "syscall": "execve",
    "path": "/usr/bin/curl",
    "cwd": "/root",
    "cmdline": "curl https://example.com",
    "duration": "1234567",
    "time": "1748996400"
  }
}
```

### `osqueryd.INFO` — Status Log

```
I0604 12:00:00.000000 12345 init.cpp:200] osqueryd initialized [version=5.14.0]
```

### Ingesting into a SIEM

**Filebeat / Elastic Agent**:
```yaml
filebeat.inputs:
  - type: filestream
    id: osquery-results
    paths:
      - /var/log/osquery/osqueryd.results.log
    parsers:
      - ndjson:
          target: osquery
          add_error_key: true
    tags: ["osquery", "edr"]

output.elasticsearch:
  hosts: ["https://elasticsearch:9200"]
  index: "osquery-edr-%{+yyyy.MM.dd}"
```

**Fluentd / Fluent Bit**:
```ini
[INPUT]
    Name              tail
    Path              /var/log/osquery/osqueryd.results.log
    Tag               osquery.results
    Parser            json
    Read_from_Head    false

[OUTPUT]
    Name              forward
    Host              fluentd-aggregator
    Port              24224
```

---

## 16. Verification & Testing

### Step 1: Verify osqueryd is running

```bash
systemctl status osqueryd
pgrep -a osqueryd
```

### Step 2: Check all flags loaded correctly

```bash
osqueryi --flagfile /etc/osquery/osquery.flags \
  "SELECT name, value, default_value FROM osquery_flags WHERE default_value <> value;"
```

### Step 3: Verify event publishers are active

```bash
osqueryi --flagfile /etc/osquery/osquery.flags \
  "SELECT name, publisher, type, subscriptions, events, active FROM osquery_events;"
# Look for: active=1 for process_events, socket_events, file_events, bpf_process_events
```

### Step 4: Trigger a test process event

```bash
# In one terminal, watch:
osqueryi --disable_events=false --disable_audit=false \
  --audit_allow_config=true --audit_persist=true \
  --audit_allow_process_events=true --events_expiry=1 \
  "SELECT pid, path, cmdline FROM process_events;"

# In another terminal:
ls -la /tmp  # Trigger execve
touch /etc/passwd  # Trigger file event
```

### Step 5: Verify BPF events work

```bash
# Test BPF process events
osqueryi --disable_events=false --enable_bpf_events=true \
  "SELECT pid, path, cmdline, cid FROM bpf_process_events LIMIT 5;"
# Should see results within seconds on an active system
```

### Step 6: Check FIM events

```bash
# Touch a watched file and check file_events:
touch /etc/hosts
osqueryi --flagfile /etc/osquery/osquery.flags \
  "SELECT target_path, action, time FROM file_events LIMIT 5;"
```

### Step 7: Verify log output

```bash
tail -f /var/log/osquery/osqueryd.results.log | python3 -m json.tool
# Should see formatted JSON results from scheduled queries
```

### Step 8: Validate config

```bash
osqueryd --config_path /etc/osquery/osquery.conf --config_check
# Exit code 0 = valid
echo "Config check exit code: $?"
```

---

## 17. Performance Tuning & Watchdog

### Watchdog Internals

osquery uses a **two-process model**:
- **Watchdog (parent)**: monitors worker via `waitpid()`, tracks CPU with
  `getrusage()` every 3 seconds
- **Worker (child)**: runs queries, manages events, writes logs

If the worker exceeds `watchdog_memory_limit` MB of RSS or
`watchdog_utilization_limit`% CPU for more than 12 seconds (the latency limit),
the watchdog SIGKILLS it and restarts. This means:
- Expensive queries may be killed mid-execution
- If a query is killed 5 times, it's "denylisted" for 24 hours
- Set `"denylist": false` on critical EDR queries to prevent denylisting

### Per-Query Performance Profiling

```sql
-- Run in osqueryi to see expensive queries:
SELECT name, executions, wall_time, user_time, system_time,
       average_memory, output_size,
       (wall_time / executions) AS avg_wall_ms
FROM osquery_schedule
ORDER BY avg_wall_ms DESC;
```

### Expensive Table Patterns to Avoid

```sql
-- BAD: full table scan with no constraint — hashes ALL files
SELECT * FROM hash WHERE path LIKE '/usr/%';

-- GOOD: constraint-pushed specific files
SELECT path, sha256 FROM hash WHERE path IN ('/usr/bin/sudo', '/usr/bin/su');

-- BAD: processes JOIN on event table without LIMIT
SELECT * FROM process_events JOIN processes ON process_events.pid = processes.pid;

-- GOOD: event tables are already differential; no JOIN needed unless enriching
SELECT pid, path, cmdline, time FROM process_events;
```

### RocksDB Maintenance

```bash
# Check RocksDB size:
du -sh /var/osquery/osquery.db/

# If it grows too large, reduce events_max or events_expiry
# Emergency cleanup (stop osqueryd first):
systemctl stop osqueryd
rm -rf /var/osquery/osquery.db/
systemctl start osqueryd
# Note: all differential state is lost; next queries will be "baseline" snapshots
```

---

## 18. Troubleshooting Runbook

### Problem: osqueryd won't start

```bash
# Check systemd logs:
journalctl -u osqueryd -n 50 --no-pager

# Run manually with verbose:
/opt/osquery/bin/osqueryd \
  --flagfile /etc/osquery/osquery.flags \
  --verbose \
  --logger_plugin=filesystem \
  --logger_path=/tmp/osquery-debug/ \
  --disable_watchdog=true
```

### Problem: All event tables empty after startup

```bash
# 1. Verify master event toggle
osqueryi "SELECT value FROM osquery_flags WHERE name='disable_events';"
# Must show "false"

# 2. Verify audit toggle
osqueryi "SELECT value FROM osquery_flags WHERE name='disable_audit';"
# Must show "false"

# 3. Check if auditd is running
systemctl is-active auditd && echo "PROBLEM: stop auditd"

# 4. Check audit socket ownership
auditctl -s 2>/dev/null | grep pid
# PID should match osqueryd's pid

# 5. Check osquery_events table
osqueryi "SELECT name, active, events FROM osquery_events WHERE type='publisher';"
```

### Problem: BPF events empty / error in logs

```bash
# Check kernel version
uname -r   # Must be >= 4.18

# Check BPF syscall permission
strace -e bpf /opt/osquery/bin/osqueryi \
  --disable_events=false --enable_bpf_events=true \
  "SELECT 1;" 2>&1 | grep "bpf(" | head -5

# Check for EPERM on bpf() call
# If you see EPERM, check:
cat /proc/sys/kernel/unprivileged_bpf_disabled
# If not 0, BPF is restricted to root; run osqueryd as root
```

### Problem: `iptables` table is empty

```bash
ls /proc/net/ip_tables_names  # File must exist
# If missing:
modprobe ip_tables
# Verify:
cat /proc/net/ip_tables_names  # Should show: filter nat mangle ...
```

### Problem: `file_events` empty

```bash
# 1. Check inotify watches:
cat /proc/sys/fs/inotify/max_user_watches
sysctl fs.inotify.max_user_watches=524288

# 2. Verify file_paths in config:
osqueryi --config_path /etc/osquery/osquery.conf \
  "SELECT name FROM osquery_events WHERE type='subscriber' AND name='file_events';"

# 3. Verify enable_file_events flag:
osqueryi "SELECT value FROM osquery_flags WHERE name='enable_file_events';"
# Must be "true"

# 4. Trigger a change and immediately query (events_optimize prevents "old" events):
touch /etc/hosts
sleep 1
osqueryi --disable_events=false --enable_file_events=true \
  --flagfile /etc/osquery/osquery.flags \
  "SELECT * FROM file_events LIMIT 5;"
```

### Problem: High CPU / Watchdog kills worker

```bash
# Identify the most expensive queries:
osqueryi "SELECT name, avg_memory, wall_time, executions FROM osquery_schedule \
  ORDER BY wall_time/executions DESC LIMIT 10;"

# Options:
# 1. Increase watchdog_memory_limit and watchdog_utilization_limit
# 2. Set "denylist": false on critical queries
# 3. Add table_delay to slow down JOINs
# 4. Reduce interval on expensive queries
# 5. Add constraints to avoid full table scans
```

### Problem: `deb_packages` or `rpm_packages` empty on correct distro

```bash
# Debian/Ubuntu: check dpkg database
ls -la /var/lib/dpkg/status
dpkg --get-selections | wc -l   # Should match osquery row count

# RHEL/CentOS: check RPM database
rpm -qa | wc -l
# If corrupted:
rpm --rebuilddb
```

### Quick Diagnostic Script

```bash
#!/bin/bash
# Save as /usr/local/bin/osquery-diag.sh

echo "=== osquery EDR Diagnostics ==="
echo ""

echo "--- Version ---"
osqueryd --version 2>/dev/null

echo ""
echo "--- Service Status ---"
systemctl is-active osqueryd

echo ""
echo "--- auditd Conflict Check ---"
systemctl is-active auditd && \
  echo "⚠️  WARNING: auditd is running! Process/socket events will not work." || \
  echo "✓  auditd is stopped"

echo ""
echo "--- Audit Socket ---"
auditctl -s 2>/dev/null | grep -E "enabled|pid|backlog|lost" || echo "auditctl not available"

echo ""
echo "--- inotify Limits ---"
echo "max_user_watches: $(cat /proc/sys/fs/inotify/max_user_watches)"
echo "max_user_instances: $(cat /proc/sys/fs/inotify/max_user_instances)"

echo ""
echo "--- BPF Support ---"
uname -r
cat /boot/config-$(uname -r) 2>/dev/null | grep -E "CONFIG_BPF_SYSCALL=|CONFIG_KPROBES=" | head -5

echo ""
echo "--- iptables proc ---"
[ -f /proc/net/ip_tables_names ] && \
  echo "✓  /proc/net/ip_tables_names exists: $(cat /proc/net/ip_tables_names)" || \
  echo "⚠️  /proc/net/ip_tables_names missing — iptables table will be empty"

echo ""
echo "--- Log Output (last 5 results) ---"
tail -n 5 /var/log/osquery/osqueryd.results.log 2>/dev/null | python3 -m json.tool 2>/dev/null || \
  echo "No results yet"
```

---

## Appendix A: Quick Reference — All Linux-Specific Tables

### Linux-Only Tables (not on Windows/macOS)

```
apparmor_events         apparmor_profiles       apt_sources
bpf_process_events      bpf_socket_events       deb_package_files
deb_packages            iptables                kernel_keys
kernel_modules          lxd_certificates        lxd_cluster
lxd_cluster_members     lxd_containers          lxd_images
lxd_instance_config     lxd_instance_devices    lxd_networks
lxd_storage_pools       process_events          process_file_events
rpm_package_files       rpm_packages            selinux_events
selinux_settings        socket_events           user_events
yum_sources
```

### Cross-Platform Tables Available on Linux

```
acpi_tables             arp_cache               augeas
block_devices           carbon_black_info       certificates
cpu_info                crontab                 curl
curl_certificate        dbus_packages           disk_encryption
disk_info               dns_resolvers           docker_*
environment             etc_hosts               etc_protocols
etc_services            file                    file_events (inotify)
groups                  hardware_events         hash
hostname                interface_addresses     interface_details
last                    listening_ports         logged_in_users
memory_info             mounts                  npm_packages
nvram                   os_version              osquery_*
pam                     pipes                   platform_info
process_envs            process_memory_map      process_namespaces
process_open_files      process_open_sockets    processes
python_packages         routes                  shadow
shared_memory           shell_history           smart_drive_info
startup_items           sudoers                 suid_bin
system_info             systemd_units           time
ulimit_info             uptime                  usb_devices
user_ssh_keys           users                   yara
yara_events             ycloud_instance_metadata
```

---

## Appendix B: Minimal EDR flags (Kernel < 4.18, Audit Only)

For older kernels that don't support BPF, use this minimal flags subset:

```ini
# Minimal flags for kernel 2.6.x – 4.17.x (Audit-only mode)
--config_plugin=filesystem
--config_path=/etc/osquery/osquery.conf
--logger_plugin=filesystem
--logger_path=/var/log/osquery/
--database_path=/var/osquery/osquery.db
--pidfile=/var/osquery/osqueryd.pidfile
--disable_events=false
--disable_audit=false
--audit_allow_config=true
--audit_persist=true
--audit_allow_process_events=true
--audit_allow_sockets=true
--audit_allow_user_events=true
--audit_allow_fim_events=true
--audit_allow_kill_process_events=true
--audit_backlog_limit=8192
--enable_file_events=true
--watchdog_memory_limit=300
--watchdog_utilization_limit=25
--watchdog_delay=60
--host_identifier=uuid
--events_expiry=3600
--events_max=50000
--events_optimize=true
--logger_rotate=true
--logger_rotate_size=26214400
--logger_rotate_max_files=25
```

And use `process_events` / `socket_events` in your schedule instead of
`bpf_process_events` / `bpf_socket_events`.

---

*Guide written against osquery 5.x — June 2026.
For the latest table schema: https://osquery.io/schema/
Official documentation: https://osquery.readthedocs.io/en/stable/*
