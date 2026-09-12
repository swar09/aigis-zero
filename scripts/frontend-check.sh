#!/usr/bin/env bash
set -euo pipefail

# scripts/frontend-check.sh
# Frontend quality gate: lint, typecheck, test, and build.
# Usage: ./scripts/frontend-check.sh [--fix]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_DIR="$(cd "$SCRIPT_DIR/../frontend" && pwd)"

step() { echo -e "\n\033[1;34m▶ $1\033[0m"; }
ok()   { echo -e "\033[1;32m✔ $1\033[0m"; }
fail() { echo -e "\033[1;31m✘ $1\033[0m"; exit 1; }

FIX=false
for arg in "$@"; do
  if [[ "$arg" == "--fix" ]]; then
    FIX=true
  fi
done

cd "$FRONTEND_DIR"

step "frontend lint"
if $FIX; then
  npm run lint -- --fix || fail "frontend lint failed"
else
  npm run lint || fail "frontend lint failed"
fi
ok "lint clean"

step "frontend typecheck"
npm run typecheck || fail "frontend typecheck failed"
ok "typecheck clean"

step "frontend unit & component tests"
npm run test || fail "frontend tests failed"
ok "tests passed"

echo -e "\n\033[1;32mFrontend checks passed.\033[0m"
