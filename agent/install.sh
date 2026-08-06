#!/usr/bin/env bash
# =============================================================================
# Aigis-Zero Agent Installer
# Method: musl tarball (pre-built static binary from GitHub Releases)
#
# Usage:
#   tar -xzf aigis-zero-agent-linux-<arch>.tar.gz
#   cd aigis-zero-agent
#   sudo bash install.sh
#
# Supported distros: Debian, Ubuntu, Fedora, RHEL, CentOS, Rocky, AlmaLinux
# Supported architectures: x86_64, aarch64
# =============================================================================
set -euo pipefail

# ── Root check ────────────────────────────────────────────────────────────────
if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (sudo bash install.sh)."
  exit 1
fi

cat << "EOF"
  ______   ______   ______   ______   ______        ________  ________  _______    ______  
 /      \ /      | /      \ /      | /      \      /        |/        |/       \  /      \ 
/$$$$$$  |$$$$$$/ /$$$$$$  |$$$$$$/ /$$$$$$  |     $$$$$$$$/ $$$$$$$$/ $$$$$$$  |/$$$$$$  |
$$ |__$$ |  $$ |  $$ | _$$/   $$ |  $$ \__$$/  ______  /$$/  $$ |__    $$ |__$$ |$$ |  $$ |
$$    $$ |  $$ |  $$ |/    |  $$ |  $$      \ /      |/$$/   $$    |   $$    $$< $$ |  $$ |
$$$$$$$$ |  $$ |  $$ |$$$$ |  $$ |   $$$$$$  |$$$$$$//$$/    $$$$$/    $$$$$$$  |$$ |  $$ |
$$ |  $$ | _$$ |_ $$ \__$$ | _$$ |_ /  \__$$ |      /$$/____ $$ |_____ $$ |  $$ |$$ \__$$ |
$$ |  $$ |/ $$   |$$    $$/ / $$   |$$    $$/      /$$      |$$       |$$ |  $$ |$$    $$/ 
$$/   $$/ $$$$$$/  $$$$$$/  $$$$$$/  $$$$$$/       $$$$$$$$/ $$$$$$$$/ $$/   $$/  $$$$$$/  

EOF

echo "Aigis-Zero Agent Installer"
echo "=========================="

TOTAL_STEPS=9

step() {
    printf "[%d/%d] %-50s " "$1" "$TOTAL_STEPS" "$2"
}
ok() {
    printf "[DONE]\n"
}
fail() {
    printf "[FAIL]\n"
    echo "ERROR: $1" >&2
    exit 1
}
warn() {
    printf "[WARN]\n"
    echo "WARNING: $1" >&2
}

# ── Step 1: Detect architecture ───────────────────────────────────────────────
step 1 "Detecting architecture"
HOST_ARCH=$(uname -m)
case "$HOST_ARCH" in
  x86_64)  ;;
  aarch64) ;;
  *) fail "Unsupported architecture: $HOST_ARCH. Only x86_64 and aarch64 are supported." ;;
esac
ok
echo "         Architecture: $HOST_ARCH"

# ── Step 2: Detect distro ─────────────────────────────────────────────────────
step 2 "Detecting Linux distribution"
if [ -f /etc/os-release ]; then
  # shellcheck disable=SC1091
  . /etc/os-release
  DISTRO_ID="${ID:-unknown}"
  DISTRO_ID_LIKE="${ID_LIKE:-}"
else
  DISTRO_ID="unknown"
  DISTRO_ID_LIKE=""
fi

# Normalize: treat ID_LIKE derivatives as their base family
is_debian_family() {
  case "$DISTRO_ID" in ubuntu|debian|linuxmint|pop|kali) return 0 ;; esac
  case "$DISTRO_ID_LIKE" in *debian*|*ubuntu*) return 0 ;; esac
  return 1
}
is_rpm_family() {
  case "$DISTRO_ID" in fedora|rhel|centos|rocky|almalinux|ol) return 0 ;; esac
  case "$DISTRO_ID_LIKE" in *rhel*|*fedora*|*centos*) return 0 ;; esac
  return 1
}
ok
echo "         Distribution: ${PRETTY_NAME:-$DISTRO_ID}"

# ── Step 3: Install dependencies and osquery ──────────────────────────────────
step 3 "Installing dependencies and osquery"

if is_debian_family; then
    # Runtime deps for osquery
    apt-get update -qq 2>/dev/null || true
    for pkg in wget curl gnupg2 ca-certificates libcap2 libudev1 libblkid1 libaudit1 nftables; do
        apt-get install -y -qq "$pkg" 2>/dev/null || true
    done

    # osquery official repository
    if ! command -v osqueryd &>/dev/null; then
        mkdir -p /usr/share/keyrings
        curl -fsSL https://pkg.osquery.io/deb/pubkey.gpg \
          | gpg --dearmor -o /usr/share/keyrings/osquery.gpg 2>/dev/null || true
        echo "deb [signed-by=/usr/share/keyrings/osquery.gpg] https://pkg.osquery.io/deb deb main" \
          | tee /etc/apt/sources.list.d/osquery.list >/dev/null
        apt-get update -qq 2>/dev/null || true
        apt-get install -y -qq osquery || fail "Failed to install osquery package via apt."
    fi

elif is_rpm_family; then
    # Package manager (prefer dnf, fall back to yum)
    if command -v dnf &>/dev/null; then PKG_MGR=dnf; else PKG_MGR=yum; fi

    # Runtime deps for osquery and firewall
    for pkg in wget curl ca-certificates libcap audit-libs systemd-libs util-linux-libs nftables; do
        $PKG_MGR install -y -q "$pkg" 2>/dev/null || true
    done

    # osquery official repository
    if ! command -v osqueryd &>/dev/null; then
        mkdir -p /etc/pki/rpm-gpg
        curl -fsSL https://pkg.osquery.io/rpm/GPG \
          | tee /etc/pki/rpm-gpg/RPM-GPG-KEY-osquery >/dev/null || true
        mkdir -p /etc/yum.repos.d
        cat > /etc/yum.repos.d/osquery.repo << 'REPO'
[osquery-s3-rpm-release]
name=osquery-s3-rpm-release
baseurl=https://pkg.osquery.io/rpm
enabled=1
repo_gpgcheck=1
gpgcheck=0
gpgkey=https://pkg.osquery.io/rpm/GPG
REPO
        $PKG_MGR install -y -q osquery || fail "Failed to install osquery package via $PKG_MGR."
    fi

else
    warn "Unrecognized distro '$DISTRO_ID'. Skipping package installation. Install osquery manually."
fi

# Mask auditd conflicts — osquery in eBPF mode + disable_audit=true;
# journald audit socket can still conflict, mask it proactively.
systemctl mask --now systemd-journald-audit.socket 2>/dev/null || true

ok

# ── Step 4: Stop existing services ───────────────────────────────────────────
step 4 "Stopping existing services"
systemctl stop aigis-zero.service 2>/dev/null || true
systemctl stop osqueryd.service   2>/dev/null || true
ok

# ── Step 5: Install agent binary ──────────────────────────────────────────────
step 5 "Installing agent binary"

# install.sh is inside the extracted tarball.
# Tarball layout (from agent-release.yml packaging step):
#   aigis-zero-agent/
#     aigis-zero        ← the binary
#     install.sh        ← this script
#     uninstall.sh
#     agent.toml
#     osquery/
#     sysctl/
#     limits/
#     systemd/
#
# So the binary is at ./aigis-zero relative to this script.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="${SCRIPT_DIR}/aigis-zero"

if [ ! -f "$BINARY" ]; then
    fail "Binary not found at '${BINARY}'. " \
         "Run install.sh from inside the extracted tarball directory."
fi

# Verify the binary matches the host arch
BINARY_ARCH=$(file -b "$BINARY" 2>/dev/null || true)
case "$HOST_ARCH" in
  x86_64)
    if ! echo "$BINARY_ARCH" | grep -qi "x86-64\|x86_64"; then
      warn "Binary arch may not match host ($HOST_ARCH). Proceeding anyway."
    fi
    ;;
  aarch64)
    if ! echo "$BINARY_ARCH" | grep -qi "aarch64\|arm64"; then
      warn "Binary arch may not match host ($HOST_ARCH). Proceeding anyway."
    fi
    ;;
esac

mkdir -p /usr/sbin
install -o root -g root -m 0755 "$BINARY" /usr/sbin/aigis-zero
ok

# ── Step 6: Set up agent directories and config ───────────────────────────────
step 6 "Setting up agent directories and config"
mkdir -p /etc/aigis-zero
mkdir -p /var/lib/aigis-zero
mkdir -p /var/log/aigis-zero

# Strict permissions: agent config is root-only
chown root:root /etc/aigis-zero /var/lib/aigis-zero /var/log/aigis-zero
chmod 700 /etc/aigis-zero
chmod 700 /var/lib/aigis-zero
chmod 755 /var/log/aigis-zero     # allow log forwarders to read

install -o root -g root -m 640 "${SCRIPT_DIR}/agent.toml" /etc/aigis-zero/config.toml
ok

# ── Step 7: Apply sysctl tunables ─────────────────────────────────────────────
step 7 "Applying kernel tunables (sysctl)"
install -o root -g root -m 644 \
    "${SCRIPT_DIR}/sysctl/60-aigis-zero.conf" /etc/sysctl.d/
sysctl --system >/dev/null 2>&1

# Verify critical tunables were applied
SYSCTL_FAIL=0
VAL=$(sysctl -n fs.inotify.max_user_watches 2>/dev/null || echo 0)
[ "$VAL" -lt 524288 ] && SYSCTL_FAIL=1
VAL=$(sysctl -n net.core.bpf_jit_enable 2>/dev/null || echo 0)
[ "$VAL" -ne 1 ] && SYSCTL_FAIL=1

if [ "$SYSCTL_FAIL" -eq 1 ]; then
    warn "Some sysctl tunables were not applied. eBPF/file_events may have issues."
else
    ok
fi

# ── Step 8: Apply ulimits ─────────────────────────────────────────────────────
step 8 "Applying ulimits (pam limits)"
install -o root -g root -m 644 \
    "${SCRIPT_DIR}/limits/99-aigis-zero.conf" /etc/security/limits.d/
ok

# ── Step 9: Set up osquery config and directories ─────────────────────────────
step 9 "Setting up osquery config and directories"

# Create osquery directories with correct permissions
# /etc/osquery: world-traversable, root-owned (osquery reads its own config as root)
# /var/osquery: root-only (RocksDB, pidfile, extension socket)
# /var/log/osquery: readable by log agents
# /run/osquery: runtime directory for pidfile (recreated on each boot)
mkdir -p /etc/osquery
mkdir -p /var/osquery
mkdir -p /var/log/osquery
mkdir -p /run/osquery

chown root:root /etc/osquery /var/osquery /var/log/osquery /run/osquery
chmod 755 /etc/osquery       # world-traversable; files are root-owned
chmod 750 /var/osquery       # root-only for RocksDB and socket
chmod 755 /var/log/osquery   # allow log forwarders to read
chmod 755 /run/osquery       # runtime dir; writable only by root

# Install config files
install -o root -g root -m 644 \
    "${SCRIPT_DIR}/osquery/osquery.conf"  /etc/osquery/osquery.conf
install -o root -g root -m 644 \
    "${SCRIPT_DIR}/osquery/osquery.flags" /etc/osquery/osquery.flags

# extensions.load must exist even if empty — osquery will fail to start without it
touch /etc/osquery/extensions.load
chown root:root /etc/osquery/extensions.load
chmod 644 /etc/osquery/extensions.load

# Set up default environment file for osqueryd service (Debian /etc/default vs RPM /etc/sysconfig paths)
if [ -d /etc/default ]; then
    cat > /etc/default/osqueryd << 'ENV'
FLAG_FILE="/etc/osquery/osquery.flags"
CONFIG_FILE="/etc/osquery/osquery.conf"
LOCAL_PIDFILE="/var/osquery/osqueryd.pidfile"
PIDFILE="/var/run/osqueryd.pid"
ENV
    chown root:root /etc/default/osqueryd
    chmod 644 /etc/default/osqueryd
fi
if [ -d /etc/sysconfig ]; then
    cat > /etc/sysconfig/osqueryd << 'ENV'
FLAG_FILE="/etc/osquery/osquery.flags"
CONFIG_FILE="/etc/osquery/osquery.conf"
LOCAL_PIDFILE="/var/osquery/osqueryd.pidfile"
PIDFILE="/var/run/osqueryd.pid"
ENV
    chown root:root /etc/sysconfig/osqueryd
    chmod 644 /etc/sysconfig/osqueryd
fi

ok

# ── Install systemd units ──────────────────────────────────────────────────────
echo ""
echo "[+] Installing systemd service units"

install -o root -g root -m 644 \
    "${SCRIPT_DIR}/systemd/aigis-zero.service" /etc/systemd/system/

mkdir -p /etc/systemd/system/osqueryd.service.d
install -o root -g root -m 644 \
    "${SCRIPT_DIR}/systemd/osqueryd.service.d/aigis-zero.conf" \
    /etc/systemd/system/osqueryd.service.d/aigis-zero.conf

systemctl daemon-reload

# ── Enable and start services (independently) ─────────────────────────────────
echo "[+] Enabling services"
systemctl enable osqueryd.service  >/dev/null 2>&1
systemctl enable aigis-zero.service >/dev/null 2>&1

echo "[+] Starting osqueryd"
systemctl start osqueryd.service

echo "[+] Starting aigis-zero"
systemctl start aigis-zero.service

echo ""
echo "=========================="
echo "Installation complete!"
echo ""
echo "Both services run independently:"
echo "  systemctl status osqueryd    # osquery daemon"
echo "  systemctl status aigis-zero  # EDR agent"
echo ""
echo "View logs:"
echo "  journalctl -u osqueryd -f"
echo "  journalctl -u aigis-zero -f"
echo "=========================="
