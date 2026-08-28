#!/usr/bin/env bash
set -euo pipefail

# scripts/ci.sh
# Exact commands CI runs. No auto-fixing — everything must already be clean.
# Usage: ./scripts/ci.sh

# Ensure SQLX_OFFLINE=true is defaulted if DATABASE_URL is not set so checks don't fail when DB is down
if [[ -z "${DATABASE_URL:-}" && -z "${SQLX_OFFLINE:-}" ]]; then
  export SQLX_OFFLINE=true
fi

# Support Homebrew keg-only libpq on macOS
if [[ "$(uname -s)" == "Darwin" ]]; then
  if [[ -d "/opt/homebrew/opt/libpq" ]]; then
    export LIBRARY_PATH="/opt/homebrew/opt/libpq/lib:${LIBRARY_PATH:-}"
    export PKG_CONFIG_PATH="/opt/homebrew/opt/libpq/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
    export PATH="/opt/homebrew/opt/libpq/bin:$PATH"
  elif [[ -d "/usr/local/opt/libpq" ]]; then
    export LIBRARY_PATH="/usr/local/opt/libpq/lib:${LIBRARY_PATH:-}"
    export PKG_CONFIG_PATH="/usr/local/opt/libpq/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
    export PATH="/usr/local/opt/libpq/bin:$PATH"
  fi
fi

step() { echo -e "\n\033[1;34m▶ $1\033[0m"; }

step "fmt check (using nightly for import grouping)"
FMT_CMD="cargo fmt"
if cargo +nightly --version >/dev/null 2>&1; then
  FMT_CMD="cargo +nightly fmt"
fi
$FMT_CMD --all -- --check

step "clippy (deny warnings)"
cargo clippy --all-targets --all-features -- -D warnings

step "typos"
command -v typos >/dev/null 2>&1 && typos || echo "  (typos-cli not installed, skipping)"

step "build"
cargo build --all-targets --all-features

step "test"
cargo test --all-features

step "doc build (catches broken doc links/examples)"
cargo doc --all-features --no-deps

step "security audit"
command -v cargo-audit >/dev/null 2>&1 && cargo audit || echo "  (cargo-audit not installed: cargo install cargo-audit)"

echo -e "\n\033[1;32mCI checks passed locally.\033[0m"
