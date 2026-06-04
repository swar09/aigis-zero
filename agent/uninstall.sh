#!/usr/bin/env bash
set -e

# Aigis-Zero Agent Uninstallation Script
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
echo "Aigis-Zero Agent Uninstaller"
echo "============================"

step() {
    printf "[%d/9] %-45s " "$1" "$2"
}
done_step() {
    printf "[DONE]\n"
}

# 1. Stop agent
step 1 "Stopping agent service"
systemctl stop aigis-zero.service >/dev/null 2>&1 || true
systemctl disable aigis-zero.service >/dev/null 2>&1 || true
done_step

# 2. Stop osquery
step 2 "Stopping osquery service"
systemctl stop osqueryd.service >/dev/null 2>&1 || true
done_step

# 3. Remove binary
step 3 "Removing binary"
rm -f /usr/sbin/aigis-zero
done_step

# 4. Remove config/data
step 4 "Removing agent config and data"
rm -rf /etc/aigis-zero
rm -rf /var/lib/aigis-zero
rm -rf /var/log/aigis-zero
done_step

# 5. Remove services
step 5 "Removing systemd services"
rm -f /etc/systemd/system/aigis-zero.service
rm -rf /etc/systemd/system/osqueryd.service.d/aigis-zero.conf
done_step

# 6. Remove tunables
step 6 "Removing sysctl and limits"
rm -f /etc/sysctl.d/60-aigis-zero.conf
rm -f /etc/security/limits.d/99-aigis-zero.conf
sysctl --system >/dev/null 2>&1 || true
done_step

# 7. Remove nftables
step 7 "Removing isolation rules"
nft delete table inet aigis_isolation >/dev/null 2>&1 || true
done_step

# 8. Remove osquery configs
step 8 "Removing osquery configurations"
rm -f /etc/osquery/osquery.conf
rm -f /etc/osquery/osquery.flags
done_step

# 9. Systemd reload
step 9 "Reloading systemd daemon"
systemctl daemon-reload
done_step

echo "============================"
echo "Uninstallation complete!"
