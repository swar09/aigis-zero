#!/usr/bin/env bash
set -euo pipefail

# scripts/precommit.sh
# Installs check.sh as the git pre-commit hook.
# Usage: ./scripts/precommit.sh

HOOK_DIR="$(git rev-parse --git-dir)/hooks"
HOOK_FILE="$HOOK_DIR/pre-commit"

mkdir -p "$HOOK_DIR"
cat > "$HOOK_FILE" << 'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Running pre-commit checks..."
./scripts/check.sh
EOF

chmod +x "$HOOK_FILE"
echo "✔ pre-commit hook installed at $HOOK_FILE"
