#!/usr/bin/env bash
set -euo pipefail

# scripts/clean.sh
# Cleans all build artifacts and caches.
# Usage: ./scripts/clean.sh [--deep]
#   --deep   also clears ~/.cargo/registry cache for this project's deps (rarely needed)

DEEP=false
if [[ "${1:-}" == "--deep" ]]; then
  DEEP=true
fi

echo "▶ cargo clean"
cargo clean

echo "▶ removing stray artifacts"
find . -name "*.rs.bk" -delete
find . -name "*.orig" -delete
find . -type d -name "target" -prune -exec rm -rf {} + 2>/dev/null || true

if $DEEP; then
  echo "▶ deep clean: pruning cargo registry cache"
  cargo cache --autoclean 2>/dev/null || echo "  (install cargo-cache for deep prune: cargo install cargo-cache)"
fi

echo "✔ clean complete"
