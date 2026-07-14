#!/usr/bin/env bash
set -euo pipefail

# scripts/setup.sh
# Installs the cargo subcommands the other scripts depend on.
# Usage: ./scripts/setup.sh

echo "▶ installing cargo tool dependencies"
cargo install typos-cli --locked || true
cargo install cargo-audit --locked || true
cargo install cargo-cache --locked || true
cargo install sqlx-cli --locked || true   # only if project uses sqlx

chmod +x scripts/*.sh
echo "✔ setup complete — run ./scripts/check.sh to verify"
