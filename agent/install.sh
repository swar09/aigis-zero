#!/usr/bin/env bash
set -e

# Aigis-Zero Agent Installation Script
# Must be run as root

if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root."
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

step() {
    printf "[%d/8] %-45s " "$1" "$2"
}
done_step() {
    printf "[DONE]\n"
}

# 1. Stop existing services
step 1 "Stopping existing services"
systemctl stop aigis-zero.service >/dev/null 2>&1 || true
systemctl stop osqueryd.service >/dev/null 2>&1 || true
done_step

# 2. Copy binary
step 2 "Installing agent binary"
mkdir -p /usr/sbin
cp target/x86_64-unknown-linux-musl/release/edr-agent /usr/sbin/aigis-zero
chmod 755 /usr/sbin/aigis-zero
done_step

# 3. Setup paths and permissions
step 3 "Setting up directories"
mkdir -p /etc/aigis-zero
mkdir -p /var/lib/aigis-zero
mkdir -p /var/log/aigis-zero
chmod 700 /etc/aigis-zero
chmod 700 /var/lib/aigis-zero
chmod 700 /var/log/aigis-zero
cp agent.toml /etc/aigis-zero/config.toml
chmod 600 /etc/aigis-zero/config.toml
done_step

# 4. Apply sysctl and verify
step 4 "Applying sysctl tunables"
cp sysctl/60-aigis-zero.conf /etc/sysctl.d/
sysctl --system >/dev/null 2>&1

# Verify
FAILED=0
VAL=$(sysctl -n fs.inotify.max_user_watches)
if [ "$VAL" -lt 524288 ]; then FAILED=1; fi
VAL=$(sysctl -n net.core.bpf_jit_enable)
if [ "$VAL" -ne 1 ]; then FAILED=1; fi

if [ $FAILED -eq 1 ]; then
    printf "[FAIL]\n"
    echo "Warning: Sysctl limits were not applied successfully. eBPF/osquery might have issues."
else
    done_step
fi

# 5. Apply limits
step 5 "Applying ulimits"
cp limits/99-aigis-zero.conf /etc/security/limits.d/
done_step

# 6. Copy osquery config
step 6 "Copying osquery configuration"
mkdir -p /etc/osquery
cp osquery/osquery.conf /etc/osquery/
cp osquery/osquery.flags /etc/osquery/
chmod 600 /etc/osquery/osquery.conf
chmod 600 /etc/osquery/osquery.flags
done_step

# 7. Apply systemd services
step 7 "Installing systemd services"
cp systemd/aigis-zero.service /etc/systemd/system/
mkdir -p /etc/systemd/system/osqueryd.service.d
cp systemd/osqueryd.service.d/aigis-zero.conf /etc/systemd/system/osqueryd.service.d/
systemctl daemon-reload
done_step

# 8. Start and enable services
step 8 "Starting services"
systemctl enable osqueryd.service >/dev/null 2>&1
systemctl enable aigis-zero.service >/dev/null 2>&1
systemctl start osqueryd.service
systemctl start aigis-zero.service
done_step

echo "=========================="
echo "Installation complete!"
echo "Check status: systemctl status aigis-zero"
