# Aigis-Zero Agent — Installation Guide

Two installation methods are documented here:

- **Method A** — [Musl Tarball](#method-a-musl-tarball-recommended) (recommended for production, uses pre-built static binary from GitHub Releases)
- **Method B** — [Build from Repository](#method-b-build-from-repository) (for development or custom builds)

---

## Prerequisites (both methods)

### Kernel Requirements

| Feature | Minimum | Recommended |
|---|---|---|
| Kernel version | 4.18 | 5.10+ |
| eBPF (`CONFIG_BPF_SYSCALL`) | required | required |
| inotify (FIM / `file_events`) | required | required |
| Architecture | x86_64 | x86_64 / aarch64 |

Verify eBPF support:
```bash
# Kernel version
uname -r

# eBPF must be enabled in kernel config
grep -E "CONFIG_BPF=y|CONFIG_BPF_SYSCALL=y" /boot/config-$(uname -r) 2>/dev/null || \
  zcat /proc/config.gz 2>/dev/null | grep -E "CONFIG_BPF=y|CONFIG_BPF_SYSCALL=y"

# BTF (optional but recommended for CO-RE)
ls /sys/kernel/btf/vmlinux && echo "BTF present"
```

### Required: Stop auditd

Aigis-Zero runs osquery in eBPF-only mode (`--disable_audit=true`). The Linux
audit netlink socket is **single-consumer** — if `auditd` holds it, osquery
cannot open it even in disabled mode (causes startup warnings). Mask it:

```bash
# Stop and permanently mask auditd and journald audit socket
sudo systemctl stop auditd 2>/dev/null || true
sudo systemctl disable auditd 2>/dev/null || true
sudo systemctl mask auditd 2>/dev/null || true
sudo systemctl mask --now systemd-journald-audit.socket
```

---

## Method A: Musl Tarball (Recommended)

The GitHub Release tarball contains a **statically-linked musl binary** — zero
runtime C library dependencies. It works on any Linux distro without library
version conflicts.

### Download

```bash
# Latest release (replace VERSION with the tag, e.g. agent-v0.1.0)
VERSION=agent-v0.1.0

# Auto-detect architecture
ARCH=$(uname -m)   # x86_64 or aarch64

# Download tarball
curl -fsSL \
  "https://github.com/swar09/project-edr/releases/download/${VERSION}/aigis-zero-agent-linux-${ARCH}.tar.gz" \
  -o aigis-zero-agent.tar.gz
```

> **Note:** The current GitHub Actions workflow (`agent-release.yml`) builds
> `x86_64` only. The `aarch64` tarball will be available once the aarch64
> build target is added to the release workflow.

### Extract and Install

```bash
# Extract
tar -xzf aigis-zero-agent.tar.gz

# The tarball extracts to:
#   aigis-zero-agent/
#     aigis-zero        ← the binary
#     install.sh
#     uninstall.sh
#     agent.toml
#     osquery/
#     sysctl/
#     limits/
#     systemd/

cd aigis-zero-agent

# Install (handles dependencies, osquery, permissions, and services)
sudo bash install.sh
```

### What `install.sh` Does

The installer runs **9 steps**:

| Step | Action |
|---|---|
| 1 | Detect host architecture (x86_64 / aarch64) |
| 2 | Detect Linux distribution |
| 3 | Install runtime dependencies + osquery via official repo |
| 4 | Stop existing services (safe if not installed yet) |
| 5 | Install agent binary to `/usr/sbin/aigis-zero` |
| 6 | Create agent dirs (`/etc/aigis-zero`, `/var/lib/aigis-zero`, `/var/log/aigis-zero`) |
| 7 | Apply kernel tunables (`/etc/sysctl.d/60-aigis-zero.conf`) |
| 8 | Apply ulimits (`/etc/security/limits.d/99-aigis-zero.conf`) |
| 9 | Set up osquery dirs + configs with correct permissions |

Then enables and starts both services independently.

### Installed File Locations

```
/usr/sbin/aigis-zero                         # agent binary (0755 root:root)

/etc/aigis-zero/                             # agent config (700 root:root)
  config.toml                                # agent config (640 root:root)

/var/lib/aigis-zero/                         # agent data dir (700 root:root)
/var/log/aigis-zero/                         # agent logs (755 root:root)

/etc/osquery/                                # osquery config dir (755 root:root)
  osquery.conf                               # query schedule (644 root:root)
  osquery.flags                              # startup flags (644 root:root)
  extensions.load                            # empty; required (644 root:root)

/var/osquery/                                # osquery data (750 root:root)
  osquery.db                                 # RocksDB event store
  osqueryd.pidfile                           # pid file
  osquery.em                                 # extension socket (at runtime)

/var/log/osquery/                            # osquery logs (755 root:root)
  osqueryd.results.log
  osqueryd.snapshots.log

/run/osquery/                                # runtime dir (755 root:root)

/etc/systemd/system/aigis-zero.service       # aigis-zero unit (644)
/etc/systemd/system/osqueryd.service.d/
  aigis-zero.conf                            # resource limits drop-in (644)

/etc/sysctl.d/60-aigis-zero.conf             # kernel tunables (644)
/etc/security/limits.d/99-aigis-zero.conf    # ulimits (644)
```

### Service Management

The two services are **fully independent** — neither requires the other to be
running:

```bash
# Check status
systemctl status osqueryd
systemctl status aigis-zero

# View logs
journalctl -u osqueryd   -f
journalctl -u aigis-zero -f

# Restart independently
systemctl restart osqueryd
systemctl restart aigis-zero

# Stop one without affecting the other
systemctl stop osqueryd     # aigis-zero keeps running
systemctl stop aigis-zero   # osqueryd keeps running
```

### Uninstall

```bash
# Run from inside the extracted tarball directory
sudo bash uninstall.sh
```

---

## Method B: Build from Repository

Use this method for development, custom patches, or when you want to build
from source. There is **no `install.sh` for this method** — follow the manual
steps below.

### 1. Install Rust Toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Verify
rustc --version   # should be stable >= 1.75
cargo --version
```

### 2. Install Build Dependencies

```bash
# ── Debian / Ubuntu ───────────────────────────────────────────────────────
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  protobuf-compiler \
  libssl-dev \
  libaudit-dev \
  libudev-dev \
  libblkid-dev \
  libcap-dev

# ── Fedora ────────────────────────────────────────────────────────────────
sudo dnf install -y \
  gcc \
  pkg-config \
  protobuf-compiler \
  openssl-devel \
  audit-libs-devel \
  systemd-devel \
  util-linux-devel \
  libcap-devel

# ── RHEL / CentOS / Rocky / AlmaLinux ────────────────────────────────────
sudo dnf install -y \
  gcc \
  pkg-config \
  protobuf-compiler \
  openssl-devel \
  audit-libs-devel \
  systemd-devel \
  util-linux-devel \
  libcap-devel
```

### 3. Clone and Build

```bash
git clone https://github.com/swar09/project-edr.git
cd project-edr

# Native build (dynamically linked against system glibc)
cargo build --release --bin edr-agent

# Binary location after native build:
#   target/release/edr-agent
```

#### Optional: Musl Static Build (matches release artifacts)

```bash
# Install musl target
rustup target add x86_64-unknown-linux-musl

# Install musl linker
sudo apt-get install -y musl-tools    # Debian/Ubuntu
# or: sudo dnf install -y musl-gcc   # Fedora

# Cross-compile with musl
cargo build --release --target x86_64-unknown-linux-musl --bin edr-agent

# Binary location after musl build:
#   target/x86_64-unknown-linux-musl/release/edr-agent

# For aarch64 musl (requires cross):
cargo install cross --git https://github.com/cross-rs/cross
cross build --release --target aarch64-unknown-linux-musl --bin edr-agent
#   target/aarch64-unknown-linux-musl/release/edr-agent
```

### 4. Install osquery

Download and extract the official osquery tarball, and create its systemd service unit:

```bash
# Download the osquery tarball
curl -fsSL https://pkg.osquery.io/linux/osquery-5.23.0_1.linux_x86_64.tar.gz -o osquery-5.23.0_1.linux_x86_64.tar.gz

# Extract to the system root (places files in /usr/bin/, /usr/share/osquery/, etc.)
sudo tar -xzf osquery-5.23.0_1.linux_x86_64.tar.gz -C /

# Create the osqueryd systemd service unit
sudo tee /etc/systemd/system/osqueryd.service << 'EOF'
[Unit]
Description=The osquery Daemon
After=network.target syslog.target

[Service]
Type=simple
TimeoutStartSec=0
EnvironmentFile=-/etc/sysconfig/osqueryd
EnvironmentFile=-/etc/default/osqueryd
ExecStartPre=/bin/mkdir -p /run/osquery
ExecStart=/usr/bin/osqueryd \
  --flagfile=/etc/osquery/osquery.flags \
  --config_path=/etc/osquery/osquery.conf
Restart=on-failure
KillMode=control-group

[Install]
WantedBy=multi-user.target
EOF
```



### 5. Manual Installation Steps

```bash
# From the project-edr root:

# 1. Binary
sudo install -o root -g root -m 0755 \
  target/release/edr-agent \       # native build
  /usr/sbin/aigis-zero
# or for musl:
# target/x86_64-unknown-linux-musl/release/edr-agent

# 2. Agent directories
sudo mkdir -p /etc/aigis-zero /var/lib/aigis-zero /var/log/aigis-zero
sudo chown root:root /etc/aigis-zero /var/lib/aigis-zero /var/log/aigis-zero
sudo chmod 700 /etc/aigis-zero /var/lib/aigis-zero
sudo chmod 755 /var/log/aigis-zero

# 3. Agent config
sudo install -o root -g root -m 640 \
  agent/agent.toml /etc/aigis-zero/config.toml

# 4. Kernel tunables
sudo install -o root -g root -m 644 \
  agent/sysctl/60-aigis-zero.conf /etc/sysctl.d/
sudo sysctl --system

# 5. Ulimits
sudo install -o root -g root -m 644 \
  agent/limits/99-aigis-zero.conf /etc/security/limits.d/

# 6. osquery directories and configs
sudo mkdir -p /etc/osquery /var/osquery /var/log/osquery /run/osquery

sudo chown root:root /etc/osquery /var/osquery /var/log/osquery /run/osquery
sudo chmod 755 /etc/osquery
sudo chmod 750 /var/osquery
sudo chmod 755 /var/log/osquery
sudo chmod 755 /run/osquery

sudo install -o root -g root -m 644 \
  agent/osquery/osquery.conf  /etc/osquery/osquery.conf
sudo install -o root -g root -m 644 \
  agent/osquery/osquery.flags /etc/osquery/osquery.flags

sudo touch /etc/osquery/extensions.load
sudo chmod 644 /etc/osquery/extensions.load

# 7. Systemd units
sudo install -o root -g root -m 644 \
  agent/systemd/aigis-zero.service /etc/systemd/system/

sudo mkdir -p /etc/systemd/system/osqueryd.service.d
sudo install -o root -g root -m 644 \
  agent/systemd/osqueryd.service.d/aigis-zero.conf \
  /etc/systemd/system/osqueryd.service.d/aigis-zero.conf

sudo systemctl daemon-reload

# 8. Enable and start
sudo systemctl enable osqueryd
sudo systemctl enable aigis-zero
sudo systemctl start osqueryd
sudo systemctl start aigis-zero

# 9. Verify
sudo systemctl status osqueryd
sudo systemctl status aigis-zero
```

### Build Artifacts Comparison

| Build | Binary Path | Distro Dependency | Size |
|---|---|---|---|
| Native (`cargo build --release`) | `target/release/edr-agent` | System glibc version must match | ~15-25 MB |
| Musl x86_64 | `target/x86_64-unknown-linux-musl/release/edr-agent` | None (static) | ~8-12 MB |
| Musl aarch64 | `target/aarch64-unknown-linux-musl/release/edr-agent` | None (static) | ~8-12 MB |

> **Tip:** For production deployments, always use the musl build. It runs on
> any kernel 4.18+ regardless of the system's glibc version.

---

## Troubleshooting

### osqueryd fails to start — "perf_event_open failed"
```bash
# Check kernel eBPF support
uname -r   # must be >= 4.18, ideally >= 5.10
grep CONFIG_BPF_SYSCALL /boot/config-$(uname -r)

# Check bpf_jit_enable was applied
sysctl net.core.bpf_jit_enable   # must be 1
```

### file_events table returns empty
```bash
# Check inotify limit
sysctl fs.inotify.max_user_watches   # must be >= 524288

# Reapply if low
sudo sysctl -w fs.inotify.max_user_watches=524288
```

### aigis-zero fails — "connection refused" on osquery socket
This is expected if osqueryd hasn't started yet. The agent retries the socket
connection internally. Wait for osqueryd to be fully up:
```bash
journalctl -u osqueryd -f
# Look for: "Extension manager started" or "osqueryd started"
```

### Permission denied on `/var/osquery`
```bash
# Verify permissions
ls -la /etc/osquery /var/osquery /run/osquery

# Should be:
#   /etc/osquery  755 root root
#   /var/osquery  750 root root
#   /run/osquery  755 root root
#   /etc/osquery/osquery.conf  644 root root
#   /etc/osquery/osquery.flags 644 root root

# Fix if wrong
sudo chown -R root:root /etc/osquery /var/osquery
sudo chmod 755 /etc/osquery && sudo chmod 750 /var/osquery
sudo chmod 644 /etc/osquery/osquery.conf /etc/osquery/osquery.flags
```

---

## Uninstallation

### Method A — Musl Tarball: Using `uninstall.sh`

Run from inside the extracted tarball directory (same place you ran `install.sh`):

```bash
# Standard uninstall — keeps osquery package and log files
sudo bash uninstall.sh

# Also remove the osquery package and its repo from this system
sudo bash uninstall.sh --remove-osquery

# Also delete all log directories (irreversible)
sudo bash uninstall.sh --purge-logs

# Full purge — removes everything
sudo bash uninstall.sh --remove-osquery --purge-logs
```

#### What `uninstall.sh` removes

| Step | What is removed |
|---|---|
| 1 | Stops + disables `aigis-zero.service` and `osqueryd.service` |
| 2 | `/usr/sbin/aigis-zero` (agent binary) |
| 3 | `/etc/aigis-zero/` `/var/lib/aigis-zero/` (+ `/var/log/aigis-zero/` with `--purge-logs`) |
| 4 | `/etc/systemd/system/aigis-zero.service` and `osqueryd.service.d/aigis-zero.conf` |
| 5 | `/etc/sysctl.d/60-aigis-zero.conf` (re-applies remaining sysctl config) |
| 6 | `/etc/security/limits.d/99-aigis-zero.conf` |
| 7 | `/etc/osquery/osquery.conf` `/etc/osquery/osquery.flags` `/etc/osquery/extensions.load` |
| 8 | `/var/osquery/` (RocksDB + socket) `/run/osquery/` (+ `/var/log/osquery/` with `--purge-logs`) |
| 9 | osquery package + repo files (`--remove-osquery` only) |

#### What `uninstall.sh` always preserves (by default)

- `/var/log/aigis-zero/` — agent logs (use `--purge-logs` to remove)
- `/var/log/osquery/` — osquery result logs (use `--purge-logs` to remove)
- The osquery package (use `--remove-osquery` to also uninstall)

> **Why preserve logs by default?** Log files are forensic artifacts. Removing
> them during uninstall could destroy evidence of security events. Always
> preserve them unless you explicitly need a full purge.

---

### Method B — Build from Repository: Manual Uninstall

```bash
# 1. Stop and disable services
sudo systemctl stop aigis-zero osqueryd
sudo systemctl disable aigis-zero osqueryd

# 2. Remove agent binary
sudo rm -f /usr/sbin/aigis-zero

# 3. Remove agent files
sudo rm -rf /etc/aigis-zero /var/lib/aigis-zero
# sudo rm -rf /var/log/aigis-zero   # optional — removes logs

# 4. Remove systemd units
sudo rm -f /etc/systemd/system/aigis-zero.service
sudo rm -f /etc/systemd/system/osqueryd.service.d/aigis-zero.conf
sudo rmdir /etc/systemd/system/osqueryd.service.d 2>/dev/null || true

# 5. Remove kernel tunables and ulimits
sudo rm -f /etc/sysctl.d/60-aigis-zero.conf
sudo rm -f /etc/security/limits.d/99-aigis-zero.conf
sudo sysctl --system   # re-apply remaining config

# 6. Remove osquery config files
sudo rm -f /etc/osquery/osquery.conf
sudo rm -f /etc/osquery/osquery.flags
sudo rm -f /etc/osquery/extensions.load
sudo rmdir /etc/osquery 2>/dev/null || true   # only if empty

# 7. Remove osquery runtime directories
sudo rm -rf /var/osquery
sudo rm -rf /run/osquery
# sudo rm -rf /var/log/osquery   # optional — removes logs

# 8. Reload systemd
sudo systemctl daemon-reload

# 9. (Optional) Remove osquery binary and shared assets
sudo rm -f /usr/bin/osqueryd /usr/bin/osqueryi
sudo rm -f /etc/systemd/system/osqueryd.service
sudo rm -rf /usr/share/osquery
```

