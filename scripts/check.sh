#!/usr/bin/env bash
set -euo pipefail

# scripts/check.sh
# Multi-target quality gate: rustfmt, clippy, typos, build, test, and frontend.
#
# Usage:
#   ./scripts/check.sh [TARGET] [OPTIONS]
#
# Targets:
#   all (default)                 Check all backend crates and frontend
#   frontend, -f, --frontend      Check only frontend (lint, typecheck, tests)
#   backend, -b, --backend        Check only backend (fmt, clippy, build, test for all crates)
#   <service>                     Check a specific service:
#                                   api-backend, fleet-server, agent,
#                                   kafka-pipeline, rule-engine, sdk
#   <crate>                       Check a specific crate (e.g. isolation, fleet-server-bin)
#   -s, --service <service>       Target a service by name
#   -p, --package <crate>         Target a Cargo package/crate by name
#
# Options:
#   --fix                         Auto-apply formatting and auto-fix clippy, lint, or typo issues
#   -h, --help                    Show this help message

print_usage() {
  cat << 'EOF'
Usage: ./scripts/check.sh [TARGET] [OPTIONS]

Targets:
  all (default)                 Check all backend crates and frontend
  frontend, -f, --frontend      Check only frontend (lint, typecheck, tests)
  backend, -b, --backend        Check only backend (fmt, clippy, build, test for all crates)
  <service>                     Check a specific service:
                                  api-backend, fleet-server, agent,
                                  kafka-pipeline, rule-engine, sdk
  <crate>                       Check a specific crate (e.g. isolation, fleet-server-bin)
  -s, --service <service>       Target a service by name
  -p, --package <crate>         Target a Cargo package/crate by name

Options:
  --fix                         Auto-apply formatting and auto-fix clippy, lint, or typo issues
  -h, --help                    Show this help message

Examples:
  ./scripts/check.sh                      # Check everything (default)
  ./scripts/check.sh --fix                # Check and auto-fix everything
  ./scripts/check.sh frontend             # Check only frontend
  ./scripts/check.sh frontend --fix       # Check and auto-fix frontend
  ./scripts/check.sh backend              # Check all backend crates, skip frontend
  ./scripts/check.sh api-backend          # Check api-backend service
  ./scripts/check.sh fleet-server --fix   # Check and auto-fix all fleet-server crates
  ./scripts/check.sh agent                # Check all agent crates
  ./scripts/check.sh -p isolation         # Check specific isolation crate
EOF
}

FIX=false
TARGET=""
EXPLICIT_SERVICE=""
EXPLICIT_PACKAGE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --fix)
      FIX=true
      shift
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    -f|--frontend)
      TARGET="frontend"
      shift
      ;;
    -b|--backend)
      TARGET="backend"
      shift
      ;;
    --all)
      TARGET="all"
      shift
      ;;
    -s|--service)
      if [[ -z "${2:-}" ]]; then
        echo -e "\033[1;31mError: --service requires a service name.\033[0m" >&2
        exit 1
      fi
      EXPLICIT_SERVICE="$2"
      shift 2
      ;;
    -p|--package)
      if [[ -z "${2:-}" ]]; then
        echo -e "\033[1;31mError: --package requires a crate/package name.\033[0m" >&2
        exit 1
      fi
      EXPLICIT_PACKAGE="$2"
      shift 2
      ;;
    -*)
      echo -e "\033[1;31mUnknown option: $1\033[0m" >&2
      print_usage
      exit 1
      ;;
    *)
      if [[ -z "$TARGET" ]]; then
        TARGET="$1"
      else
        echo -e "\033[1;31mMultiple targets specified: '$TARGET' and '$1'\033[0m" >&2
        print_usage
        exit 1
      fi
      shift
      ;;
  esac
done

if [[ -n "$EXPLICIT_SERVICE" ]]; then
  TARGET="$EXPLICIT_SERVICE"
elif [[ -n "$EXPLICIT_PACKAGE" ]]; then
  TARGET="$EXPLICIT_PACKAGE"
fi

if [[ -z "$TARGET" ]]; then
  TARGET="all"
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

TARGET_MODE=""       # "all", "frontend", "backend-all", "service"
CARGO_PACKAGES=()   # Array of crate names
TARGET_DIR=""        # Directory for scoped typos

case "$TARGET" in
  all)
    TARGET_MODE="all"
    ;;
  frontend)
    TARGET_MODE="frontend"
    TARGET_DIR="frontend"
    ;;
  backend)
    TARGET_MODE="backend-all"
    ;;
  api-backend|edr-api-backend)
    TARGET_MODE="service"
    CARGO_PACKAGES=("edr-api-backend")
    TARGET_DIR="api-backend"
    ;;
  fleet-server)
    TARGET_MODE="service"
    CARGO_PACKAGES=(
      "fleet-server-bin"
      "grpc-listener"
      "fleet-manager"
      "node-enrollment"
      "health-tracker"
      "fleet-tracing"
      "postgres-interface"
      "kafka-handler"
    )
    TARGET_DIR="fleet-server"
    ;;
  agent)
    TARGET_MODE="service"
    CARGO_PACKAGES=(
      "agent-bin"
      "agent-core"
      "agent-tracing"
      "event-buffer"
      "fleet-client"
      "isolation"
      "osquery-client"
    )
    TARGET_DIR="agent"
    ;;
  kafka-pipeline|edr-kafka-pipeline)
    TARGET_MODE="service"
    CARGO_PACKAGES=("edr-kafka-pipeline")
    TARGET_DIR="kafka-pipeline"
    ;;
  rule-engine|edr-rule-engine)
    TARGET_MODE="service"
    CARGO_PACKAGES=("edr-rule-engine")
    TARGET_DIR="rule-engine"
    ;;
  sdk|edr-sdk)
    TARGET_MODE="service"
    CARGO_PACKAGES=("edr-sdk")
    TARGET_DIR="sdk"
    ;;
  *)
    # Check if target matches an individual crate in workspace
    ALL_WORKSPACE_PACKAGES=$(cargo metadata --no-deps --format-version 1 2>/dev/null | jq -r '.packages[].name' 2>/dev/null || true)
    if echo "$ALL_WORKSPACE_PACKAGES" | grep -qx "$TARGET"; then
      TARGET_MODE="service"
      CARGO_PACKAGES=("$TARGET")
      MANIFEST_PATH=$(cargo metadata --no-deps --format-version 1 2>/dev/null | jq -r --arg pkg "$TARGET" '.packages[] | select(.name == $pkg) | .manifest_path' 2>/dev/null || true)
      if [[ -n "$MANIFEST_PATH" && -f "$MANIFEST_PATH" ]]; then
        TARGET_DIR="$(dirname "$MANIFEST_PATH")"
      fi
    elif [[ -d "$TARGET" && -f "$TARGET/package.json" ]]; then
      TARGET_MODE="frontend"
      TARGET_DIR="$TARGET"
    elif [[ -d "$TARGET" && -f "$TARGET/Cargo.toml" ]]; then
      TARGET_MODE="service"
      CRATE_NAME=$(cargo metadata --no-deps --format-version 1 2>/dev/null | jq -r --arg path "$(cd "$TARGET" && pwd)/Cargo.toml" '.packages[] | select(.manifest_path == $path) | .name' 2>/dev/null || true)
      if [[ -n "$CRATE_NAME" ]]; then
        CARGO_PACKAGES=("$CRATE_NAME")
        TARGET_DIR="$TARGET"
      else
        echo -e "\033[1;31mError: Directory '$TARGET' has a Cargo.toml but is not a recognized workspace member.\033[0m" >&2
        exit 1
      fi
    else
      echo -e "\033[1;31mError: Unrecognized target, service, or package: '$TARGET'\033[0m" >&2
      print_usage
      exit 1
    fi
    ;;
esac

# -----------------------------------------------------------------------------
# 1. FRONTEND TARGET EXECUTION
# -----------------------------------------------------------------------------
if [[ "$TARGET_MODE" == "frontend" ]]; then
  step "typo check (frontend)"
  if command -v typos >/dev/null 2>&1; then
    if $FIX; then
      typos --write-changes "$TARGET_DIR" || fail "typos failed to auto-fix in $TARGET_DIR"
    else
      typos "$TARGET_DIR" || fail "typos found in $TARGET_DIR: run with --fix or check typos.toml"
    fi
    ok "no typos in $TARGET_DIR"
  else
    echo "  (skipped: install with 'cargo install typos-cli')"
  fi

  if [[ -f "scripts/frontend-check.sh" ]]; then
    step "frontend (lint, typecheck, test)"
    if $FIX; then
      ./scripts/frontend-check.sh --fix || fail "frontend checks failed"
    else
      ./scripts/frontend-check.sh || fail "frontend checks failed"
    fi
    ok "frontend clean"
  else
    fail "scripts/frontend-check.sh not found"
  fi

  echo -e "\n\033[1;32mFrontend checks passed.\033[0m"
  exit 0
fi

# -----------------------------------------------------------------------------
# 2. SPECIFIC SERVICE / CRATE TARGET EXECUTION
# -----------------------------------------------------------------------------
if [[ "$TARGET_MODE" == "service" ]]; then
  CARGO_PKG_FLAGS=()
  for pkg in "${CARGO_PACKAGES[@]}"; do
    CARGO_PKG_FLAGS+=("-p" "$pkg")
  done

  step "rustfmt (${CARGO_PACKAGES[*]})"
  FMT_CMD="cargo fmt"
  if cargo +nightly --version >/dev/null 2>&1; then
    FMT_CMD="cargo +nightly fmt"
  fi
  if $FIX; then
    $FMT_CMD "${CARGO_PKG_FLAGS[@]}" || fail "rustfmt failed to apply"
    ok "formatted"
  else
    $FMT_CMD "${CARGO_PKG_FLAGS[@]}" -- --check || fail "formatting issues found: run with --fix (or cargo +nightly fmt)"
    ok "format clean"
  fi

  step "clippy (deny warnings for ${CARGO_PACKAGES[*]})"
  if $FIX; then
    cargo clippy "${CARGO_PKG_FLAGS[@]}" --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings \
      || fail "clippy found issues it could not auto-fix"
  else
    cargo clippy "${CARGO_PKG_FLAGS[@]}" --all-targets --all-features -- -D warnings || fail "clippy found issues"
  fi
  ok "clippy clean"

  step "typo check"
  if command -v typos >/dev/null 2>&1; then
    TYPOS_TARGET=()
    if [[ -n "$TARGET_DIR" && -d "$TARGET_DIR" ]]; then
      TYPOS_TARGET=("$TARGET_DIR")
    fi
    if $FIX; then
      typos --write-changes "${TYPOS_TARGET[@]}" || fail "typos failed to auto-fix"
    else
      typos "${TYPOS_TARGET[@]}" || fail "typos found: run with --fix or add false positives to typos.toml"
    fi
    ok "no typos"
  else
    echo "  (skipped: install with 'cargo install typos-cli')"
  fi

  step "build (${CARGO_PACKAGES[*]})"
  cargo build "${CARGO_PKG_FLAGS[@]}" --all-targets --all-features || fail "build failed"
  ok "build clean"

  step "test (${CARGO_PACKAGES[*]})"
  cargo test "${CARGO_PKG_FLAGS[@]}" --all-features || fail "tests failed"
  ok "tests passed"

  echo -e "\n\033[1;32mChecks passed for ${CARGO_PACKAGES[*]}.\033[0m"
  exit 0
fi

# -----------------------------------------------------------------------------
# 3. BACKEND-ALL & FULL SUITE (ALL) EXECUTION
# -----------------------------------------------------------------------------
step "rustfmt (uses nightly for import grouping if available)"
FMT_CMD="cargo fmt"
if cargo +nightly --version >/dev/null 2>&1; then
  FMT_CMD="cargo +nightly fmt"
fi
if $FIX; then
  $FMT_CMD --all || fail "rustfmt failed to apply"
  ok "formatted"
else
  $FMT_CMD --all -- --check || fail "formatting issues found: run with --fix (or cargo +nightly fmt)"
  ok "format clean"
fi

step "clippy (deny warnings)"
if $FIX; then
  cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings \
    || fail "clippy found issues it could not auto-fix"
else
  cargo clippy --all-targets --all-features -- -D warnings || fail "clippy found issues"
fi
ok "clippy clean"

step "typo check (requires: cargo install typos-cli)"
if command -v typos >/dev/null 2>&1; then
  if $FIX; then
    typos --write-changes || fail "typos failed to auto-fix"
  else
    typos || fail "typos found: run with --fix, or add false positives to typos.toml"
  fi
  ok "no typos"
else
  echo "  (skipped: install with: cargo install typos-cli)"
fi

step "build (all targets, all features)"
cargo build --all-targets --all-features || fail "build failed"
ok "build clean"

step "test"
cargo test --all-features || fail "tests failed"
ok "tests passed"

if [[ "$TARGET_MODE" == "all" && -f "scripts/frontend-check.sh" && -d "frontend" ]]; then
  step "frontend (lint, typecheck, test)"
  if $FIX; then
    ./scripts/frontend-check.sh --fix || fail "frontend checks failed"
  else
    ./scripts/frontend-check.sh || fail "frontend checks failed"
  fi
  ok "frontend clean"
fi

if [[ "$TARGET_MODE" == "backend-all" ]]; then
  echo -e "\n\033[1;32mBackend checks passed.\033[0m"
else
  echo -e "\n\033[1;32mAll checks passed.\033[0m"
fi
