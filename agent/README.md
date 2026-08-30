# aigis-zero agent

Endpoint telemetry daemon for Linux systems. Collects process, network, file, and authentication events using osquery and eBPF, buffers events in local SQLite storage during network partitions, and streams telemetry to the fleet server over gRPC.

## crate structure

The agent workspace contains seven focused crates:

- `agent-bin`: service entry point, config loader, and signal handling
- `agent-core`: main async event loop with exponential backoff retry (50ms base, 12.8s max)
- `osquery-client`: Thrift IPC client communicating with osqueryd over Unix sockets
- `event-buffer`: SQLite write-ahead log (WAL) for offline storage and ordered event replay
- `fleet-client`: Tonic gRPC client managing agent enrollment, heartbeats, and event streams
- `isolation`: network containment module using Linux nftables packet-filtering rules
- `agent-tracing`: structured JSON logging with configurable log levels

## prerequisites

| Requirement | Minimum | Recommended | Notes |
|---|---|---|---|
| OS | Linux | Linux | x86_64 or aarch64 |
| Kernel | 4.18 | 5.10+ | Requires `CONFIG_BPF_SYSCALL=y` |
| Memory | 256 MB | 512 MB | Under 50MB RSS typical |
| Disk | 100 MB | 1 GB | Depends on SQLite buffer retention |
| osquery | 5.23.0 | 5.23.0 | Installed automatically by installer |

Verify kernel eBPF support:

```bash
uname -r
grep -E "CONFIG_BPF=y|CONFIG_BPF_SYSCALL=y" /boot/config-$(uname -r) 2>/dev/null || \
  zcat /proc/config.gz 2>/dev/null | grep -E "CONFIG_BPF=y|CONFIG_BPF_SYSCALL=y"
```

Mask auditd to prevent netlink socket conflicts:

```bash
sudo systemctl stop auditd 2>/dev/null || true
sudo systemctl mask auditd 2>/dev/null || true
sudo systemctl mask --now systemd-journald-audit.socket
```

## installation

### method a: prebuilt musl binary

```bash
VERSION=agent-v0.1.0
ARCH=$(uname -m)

curl -fsSL \
  "https://github.com/swar09/aigis-zero/releases/download/${VERSION}/aigis-zero-agent-linux-${ARCH}.tar.gz" \
  -o aigis-zero-agent.tar.gz

tar -xzf aigis-zero-agent.tar.gz
cd aigis-zero-agent
sudo bash install.sh
```

### method b: build from source

Install build dependencies:

```bash
# Ubuntu / Debian
sudo apt-get update && sudo apt-get install -y \
  build-essential pkg-config libssl-dev libsystemd-dev libaudit-dev libcap-dev musl-tools

# RHEL / Fedora / Rocky
sudo dnf install -y \
  gcc pkg-config openssl-devel audit-libs-devel systemd-devel libcap-devel
```

Build the binary:

```bash
# Native release build
cargo build --release --bin edr-agent

# Musl static binary
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl --bin edr-agent
```

Install binary, systemd units, and configuration:

```bash
sudo install -o root -g root -m 0755 \
  target/x86_64-unknown-linux-musl/release/edr-agent /usr/sbin/aigis-zero

sudo mkdir -p /etc/aigis-zero /var/lib/aigis-zero /var/log/aigis-zero
sudo chmod 700 /etc/aigis-zero /var/lib/aigis-zero
sudo chmod 755 /var/log/aigis-zero

sudo cp agent/agent.toml /etc/aigis-zero/config.toml
sudo chmod 640 /etc/aigis-zero/config.toml

sudo cp agent/systemd/aigis-zero.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now aigis-zero
```

## configuration

Default configuration file location: `/etc/aigis-zero/config.toml`

```toml
[agent]
log_level = "info"                      # trace | debug | info | warn | error
log_format = "json"                     # json | human
log_dir = "/var/log/aigis-zero"
data_dir = "/var/lib/aigis-zero"
event_buffer_db = "/var/lib/aigis-zero/events.db"
event_buffer_max = 500000               # max records before dropping oldest
event_drain_batch = 100
event_drain_interval_secs = 5

[osquery]
socket_path = "/var/osquery/osquery.em"
conf_path = "/etc/osquery/osquery.conf"
flags_path = "/etc/osquery/osquery.flags"
connect_timeout_secs = 30
query_timeout_secs = 60

[fleet]
host = "127.0.0.1"
port = 50051
heartbeat_interval_secs = 60
reconnect_interval_secs = 10
max_reconnect_attempts = 0             # 0 = retry forever

[isolation]
enabled = false                        # set dynamically by fleet isolation commands
```

## service management

```bash
# Check service status
systemctl status aigis-zero
systemctl status osqueryd

# View live JSON logs
journalctl -u aigis-zero -f

# Restart services
sudo systemctl restart aigis-zero
```

## uninstallation

```bash
sudo systemctl stop aigis-zero osqueryd
sudo systemctl disable aigis-zero osqueryd
sudo rm -f /usr/sbin/aigis-zero
sudo rm -rf /etc/aigis-zero /var/lib/aigis-zero /var/log/aigis-zero
sudo rm -f /etc/systemd/system/aigis-zero.service
sudo rm -f /etc/systemd/system/osqueryd.service.d/aigis-zero.conf
sudo systemctl daemon-reload
```

## troubleshooting

| Symptom | Cause | Solution |
|---|---|---|
| `perf_event_open failed` | Missing kernel BPF config | Ensure kernel is >= 4.18 and `CONFIG_BPF_SYSCALL=y` is set |
| `file_events table empty` | Low inotify watch ceiling | Run `sudo sysctl -w fs.inotify.max_user_watches=524288` |
| `connection refused` on osquery socket | osqueryd still initializing | Check `journalctl -u osqueryd -f` for extension manager start |
| `permission denied on /var/osquery` | Incorrect directory permissions | Run `sudo chmod 750 /var/osquery && sudo chown root:root /var/osquery` |
| `enrollment rejected` | Secret mismatch | Verify `x-enrollment-secret` matches `FLEET_ENROLLMENT_SECRET` on fleet server |
