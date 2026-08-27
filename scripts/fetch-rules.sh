#!/usr/bin/env bash
set -euo pipefail

# scripts/fetch-rules.sh
# Downloads official MITRE ATT&CK taxonomy STIX 2.1 data and community YARA rules directly from GitHub.
# Usage:
#   ./scripts/fetch-rules.sh          # Download and provision rules
#   ./scripts/fetch-rules.sh --clean  # Remove downloaded rule datasets

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RULES_DIR="$ROOT_DIR/rule-engine/rules"

if [[ "${1:-}" == "--clean" ]]; then
    echo "▶ Cleaning downloaded YARA rules and MITRE datasets"
    rm -rf "$RULES_DIR/mitre/enterprise-attack-linux.json"
    rm -rf "$RULES_DIR/process"/*.yar
    rm -rf "$RULES_DIR/file"/*.yar
    rm -rf "$RULES_DIR/network"/*.yar
    rm -rf "$RULES_DIR/auth"/*.yar
    echo "✔ Clean complete"
    exit 0
fi

echo "▶ Provisioning rule-engine rule directories"
mkdir -p "$RULES_DIR/mitre" "$RULES_DIR/process" "$RULES_DIR/file" "$RULES_DIR/network" "$RULES_DIR/auth" "$RULES_DIR/custom"
touch "$RULES_DIR/mitre/.gitkeep" "$RULES_DIR/process/.gitkeep" "$RULES_DIR/file/.gitkeep" "$RULES_DIR/network/.gitkeep" "$RULES_DIR/auth/.gitkeep" "$RULES_DIR/custom/.gitkeep"

echo "▶ Fetching official MITRE ATT&CK Enterprise STIX data from GitHub..."
python3 - << 'EOF'
import json
import os
import urllib.request

url = "https://raw.githubusercontent.com/mitre-attack/attack-stix-data/master/enterprise-attack/enterprise-attack.json"
print("  Downloading enterprise-attack.json...")
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
try:
    with urllib.request.urlopen(req) as response:
        data = json.loads(response.read().decode('utf-8'))
except Exception as e:
    print(f"  Warning: failed to download STIX data ({e}). Creating fallback Linux taxonomy.")
    data = {"objects": []}

tactics = {}
for obj in data.get('objects', []):
    if obj.get('type') == 'x-mitre-tactic':
        short_name = obj.get('x_mitre_shortname')
        tactics[short_name] = {
            'name': obj.get('name'),
            'id': obj.get('external_references', [{}])[0].get('external_id', '')
        }

linux_techniques = {}
for obj in data.get('objects', []):
    if obj.get('type') == 'attack-pattern':
        platforms = obj.get('x_mitre_platforms', [])
        if 'Linux' in platforms and not obj.get('revoked', False) and not obj.get('x_mitre_deprecated', False):
            ext_refs = obj.get('external_references', [])
            technique_id = None
            for ref in ext_refs:
                if ref.get('source_name') == 'mitre-attack':
                    technique_id = ref.get('external_id')
                    break
            
            if not technique_id:
                continue

            kill_chain = obj.get('kill_chain_phases', [])
            tactic_name = "Execution"
            tactic_id = "TA0002"
            if kill_chain:
                phase_name = kill_chain[0].get('phase_name')
                if phase_name in tactics:
                    tactic_name = tactics[phase_name]['name']
                    tactic_id = tactics[phase_name]['id']

            severity = "medium"
            base_threat_score = 50.0
            if tactic_name in ["Privilege Escalation", "Credential Access", "Impact"]:
                severity = "critical"
                base_threat_score = 85.0
            elif tactic_name in ["Execution", "Persistence", "Defense Evasion", "Command and Control"]:
                severity = "high"
                base_threat_score = 75.0
            elif tactic_name in ["Discovery", "Lateral Movement", "Collection", "Exfiltration"]:
                severity = "medium"
                base_threat_score = 60.0
            elif tactic_name in ["Initial Access", "Reconnaissance", "Resource Development"]:
                severity = "low"
                base_threat_score = 40.0

            desc = obj.get('description', '')
            first_sentence = desc.split('. ')[0] + '.' if desc else obj.get('name', '')

            linux_techniques[technique_id] = {
                "technique_id": technique_id,
                "technique_name": obj.get('name'),
                "tactic": tactic_name,
                "tactic_id": tactic_id,
                "default_severity": severity,
                "base_threat_score": base_threat_score,
                "description": first_sentence.replace('\n', ' ').strip()
            }

output_path = "rule-engine/rules/mitre/enterprise-attack-linux.json"
os.makedirs(os.path.dirname(output_path), exist_ok=True)
with open(output_path, "w") as f:
    json.dump(linux_techniques, f, indent=2)

print(f"  Extracted {len(linux_techniques)} Linux ATT&CK techniques -> {output_path}")
EOF

echo "▶ Fetching curated community YARA rules from GitHub..."
python3 - << 'EOF'
import os
import shutil
import subprocess
import tempfile

temp_dir = tempfile.mkdtemp(prefix="yara_clone_")
try:
    print(f"  Cloning Neo23x0/signature-base into {temp_dir}...")
    subprocess.run(["git", "clone", "--depth", "1", "https://github.com/Neo23x0/signature-base.git", temp_dir], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    src_yara = os.path.join(temp_dir, "yara")
    mappings = {
        "rule-engine/rules/process": [
            "gen_mal_scripts.yar",
            "gen_recon_indicators.yar",
            "gen_cn_hacktool_scripts.yar",
            "gen_susp_hacktool.yar"
        ],
        "rule-engine/rules/file": [
            "gen_webshells.yar",
            "apt_venom_linux_rootkit.yar",
            "apt_winnti_linux.yar"
        ],
        "rule-engine/rules/network": [
            "gen_nighthawk_c2.yar",
            "webshell_regeorg.yar"
        ]
    }

    for dest_dir, files in mappings.items():
        os.makedirs(dest_dir, exist_ok=True)
        for fname in files:
            src = os.path.join(src_yara, fname)
            if os.path.exists(src):
                shutil.copy2(src, os.path.join(dest_dir, fname))
                print(f"  Installed {fname} -> {dest_dir}")
finally:
    shutil.rmtree(temp_dir, ignore_errors=True)
EOF

echo "✔ YARA rules and MITRE taxonomy successfully provisioned."
