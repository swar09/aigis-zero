#!/usr/bin/env bash
# =============================================================================
# Aigis-Zero Agent Uninstaller
# Method: musl tarball (mirrors what install.sh set up)
#
# Usage (run from inside the extracted tarball directory, or from anywhere):
#   sudo bash uninstall.sh
#
# What this removes:
#   - aigis-zero binary and all agent files
#   - osquery config files installed by this agent (NOT the osquery package itself)
#   - Systemd units and drop-ins
#   - Kernel tunables and ulimits
#   - Runtime directories (/run/osquery, /var/osquery)
#
# What this KEEPS (by default):
#   - The osquery package itself (pass --remove-osquery to also uninstall it)
#   - /var/log/osquery (logs kept for forensics; pass --purge-logs to remove)
# =============================================================================
set -euo pipefail

# ── Root check ────────────────────────────────────────────────────────────────
if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (sudo bash uninstall.sh)."
  exit 1
fi

# ── Parse flags ───────────────────────────────────────────────────────────────
REMOVE_OSQUERY=0
PURGE_LOGS=0
for arg in "$@"; do
  case "$arg" in
    --remove-osquery) REMOVE_OSQUERY=1 ;;
    --purge-logs)     PURGE_LOGS=1 ;;
    --help|-h)
      echo "Usage: sudo bash uninstall.sh [--remove-osquery] [--purge-logs]"
      echo ""
      echo "  --remove-osquery  Also uninstall the osquery package from this system"
      echo "  --purge-logs      Also delete /var/log/osquery and /var/log/aigis-zero"
      exit 0
      ;;
    *) echo "Unknown argument: $arg (try --help)"; exit 1 ;;
  esac
done

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

echo "Aigis-Zero Agent Uninstaller"
echo "============================"
[ "$REMOVE_OSQUERY" -eq 1 ] && echo "  (will also remove osquery package)"
[ "$PURGE_LOGS"     -eq 1 ] && echo "  (will also purge log directories)"
echo ""

TOTAL_STEPS=9

step() {
    printf "[%d/%d] %-52s " "$1" "$TOTAL_STEPS" "$2"
}
ok() {
    printf "[DONE]\n"
}

# ── Step 1: Stop and disable both services ────────────────────────────────────
# Each service is independent — stop them both cleanly before removing files.
step 1 "Stopping and disabling services"
systemctl stop     aigis-zero.service >/dev/null 2>&1 || true
systemctl disable  aigis-zero.service >/dev/null 2>&1 || true
systemctl stop     osqueryd.service   >/dev/null 2>&1 || true
systemctl disable  osqueryd.service   >/dev/null 2>&1 || true
ok

# ── Step 2: Remove agent binary ───────────────────────────────────────────────
step 2 "Removing agent binary"
rm -f /usr/sbin/aigis-zero
ok

# ── Step 3: Remove agent config and data ─────────────────────────────────────
step 3 "Removing agent config and data"
rm -rf /etc/aigis-zero
rm -rf /var/lib/aigis-zero
if [ "$PURGE_LOGS" -eq 1 ]; then
    rm -rf /var/log/aigis-zero
else
    echo ""
    echo "         Note: /var/log/aigis-zero preserved (use --purge-logs to remove)"
fi
ok

# ── Step 4: Remove systemd units ──────────────────────────────────────────────
step 4 "Removing systemd units and drop-ins"
rm -f  /etc/systemd/system/aigis-zero.service
# Remove only our drop-in file; if the directory is now empty, clean it up too
rm -f  /etc/systemd/system/osqueryd.service.d/aigis-zero.conf
rmdir  /etc/systemd/system/osqueryd.service.d 2>/dev/null || true
ok

# ── Step 5: Remove kernel tunables ───────────────────────────────────────────
step 5 "Removing sysctl tunables"
rm -f /etc/sysctl.d/60-aigis-zero.conf
# Re-apply remaining sysctl config to restore kernel defaults
sysctl --system >/dev/null 2>&1 || true
ok

# ── Step 6: Remove ulimits ────────────────────────────────────────────────────
step 6 "Removing ulimits"
rm -f /etc/security/limits.d/99-aigis-zero.conf
ok

# ── Step 7: Remove osquery config files ──────────────────────────────────────
# Removes only the config files we installed; does NOT remove the osquery
# package itself unless --remove-osquery was passed.
step 7 "Removing osquery config files"
rm -f /etc/osquery/osquery.conf
rm -f /etc/osquery/osquery.flags
rm -f /etc/osquery/extensions.load
# Remove /etc/osquery directory only if it is now empty
rmdir /etc/osquery 2>/dev/null || true
ok

# ── Step 8: Remove osquery runtime and data directories ──────────────────────
step 8 "Removing osquery runtime and data directories"
# /var/osquery holds RocksDB (event store) and the extension socket.
# Safe to remove after osqueryd is stopped.
rm -rf /var/osquery
# /run/osquery is a tmpfs path; systemd-tmpfiles recreates it on next boot
# if a tmpfiles.d rule exists. Removing it now is safe.
rm -rf /run/osquery
if [ "$PURGE_LOGS" -eq 1 ]; then
    rm -rf /var/log/osquery
else
    echo ""
    echo "         Note: /var/log/osquery preserved (use --purge-logs to remove)"
fi
ok

# ── Step 9: Optionally remove osquery package ─────────────────────────────────
step 9 "Cleaning up osquery package and repos"
if [ "$REMOVE_OSQUERY" -eq 1 ]; then
    # Detect distro family
    if [ -f /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        DISTRO_ID="${ID:-unknown}"
        DISTRO_ID_LIKE="${ID_LIKE:-}"
    else
        DISTRO_ID="unknown"
        DISTRO_ID_LIKE=""
    fi

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

    if is_debian_family; then
        apt-get remove -y -qq osquery 2>/dev/null || true
        rm -f /etc/apt/sources.list.d/osquery.list
        rm -f /usr/share/keyrings/osquery.gpg
        apt-get update -qq 2>/dev/null || true
    elif is_rpm_family; then
        if command -v dnf &>/dev/null; then PKG_MGR=dnf; else PKG_MGR=yum; fi
        $PKG_MGR remove -y -q osquery 2>/dev/null || true
        rm -f /etc/yum.repos.d/osquery.repo
        rm -f /etc/pki/rpm-gpg/RPM-GPG-KEY-osquery
    else
        echo ""
        printf "         Warning: unrecognized distro. Remove osquery manually.\n"
    fi
    ok
else
    printf "[SKIP] (pass --remove-osquery to also uninstall osquery)\n"
fi

# ── Remove nftables isolation rules (if any were applied) ─────────────────────
nft delete table inet aigis_isolation >/dev/null 2>&1 || true

# ── Reload systemd ────────────────────────────────────────────────────────────
systemctl daemon-reload

# ── Unmask journald audit socket if we had masked it ─────────────────────────
# Only unmask if auditd is also fully gone; leave it masked otherwise.
if ! systemctl is-enabled auditd >/dev/null 2>&1; then
    : # auditd not present — leave audit socket masked
else
    systemctl unmask systemd-journald-audit.socket 2>/dev/null || true
fi

echo ""
echo "============================"
echo "Uninstallation complete!"
echo ""
echo "The following were preserved:"
[ "$PURGE_LOGS" -eq 0 ] && echo "  /var/log/aigis-zero  (agent logs)"
[ "$PURGE_LOGS" -eq 0 ] && echo "  /var/log/osquery     (osquery logs)"
[ "$REMOVE_OSQUERY" -eq 0 ] && echo "  osquery package      (use --remove-osquery to uninstall)"
echo ""
echo "To fully purge everything:"
echo "  sudo bash uninstall.sh --remove-osquery --purge-logs"
echo "============================"
