#!/usr/bin/env bash
set -euo pipefail

# scripts/check.sh
# Single entrypoint: fmt, clippy, typos, build, test.
# Usage: ./scripts/check.sh [--fix]
#   --fix   auto-apply fmt and fixable clippy/typo issues instead of just checking

FIX=false
if [[ "${1:-}" == "--fix" ]]; then
  FIX=true
fi

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
ok()   { echo -e "\033[1;32m✔ $1\033[0m"; }
fail() { echo -e "\033[1;31m✘ $1\033[0m"; exit 1; }

# --- 1. rustfmt ---
step "rustfmt (uses nightly for import grouping if available)"
FMT_CMD="cargo fmt"
if cargo +nightly --version >/dev/null 2>&1; then
  FMT_CMD="cargo +nightly fmt"
fi

if $FIX; then
  $FMT_CMD --all || fail "rustfmt failed to apply"
  ok "formatted"
else
  $FMT_CMD --all -- --check || fail "formatting issues found — run with --fix (or cargo +nightly fmt)"
  ok "format clean"
fi

# --- 2. clippy ---
step "clippy (deny warnings)"
if $FIX; then
  cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings \
    || fail "clippy found issues it couldn't auto-fix"
else
  cargo clippy --all-targets --all-features -- -D warnings || fail "clippy found issues"
fi
ok "clippy clean"

# --- 3. typos ---
step "typo check (requires: cargo install typos-cli)"
if command -v typos >/dev/null 2>&1; then
  if $FIX; then
    typos --write-changes || fail "typos failed to auto-fix"
  else
    typos || fail "typos found — run with --fix, or add false positives to typos.toml"
  fi
  ok "no typos"
else
  echo "  (skipped — install with: cargo install typos-cli)"
fi

# --- 4. build ---
step "build (all targets, all features)"
cargo build --all-targets --all-features || fail "build failed"
ok "build clean"

# --- 5. test ---
step "test"
cargo test --all-features || fail "tests failed"
ok "tests passed"

echo -e "\n\033[1;32mAll checks passed.\033[0m"
