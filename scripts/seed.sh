#!/usr/bin/env bash
set -euo pipefail

# scripts/seed.sh
# Resets and seeds the local dev database.
# Usage: ./scripts/seed.sh [--reset]
#   --reset   drop and recreate the database before seeding

RESET=false
if [[ "${1:-}" == "--reset" ]]; then
  RESET=true
fi

# Load .env if present
if [[ -f .env ]]; then
  set -a
  # shellcheck source=/dev/null
  source .env
  set +a
fi

: "${DATABASE_URL:?DATABASE_URL not set — check .env or pass DATABASE_URL=...}"

if $RESET; then
  echo "▶ resetting database"
  if command -v sqlx >/dev/null 2>&1; then
    sqlx database drop -y || true
    sqlx database create
    sqlx migrate run --source fleet-server/migrations
  else
    echo "  sqlx-cli not found — install with: cargo install sqlx-cli"
    exit 1
  fi
fi

echo "▶ seeding data"
if cargo run --bin seed --quiet 2>/dev/null; then
  echo "✔ seeded via cargo run --bin seed"
elif [[ -f ./fleet-server/migrations/seed.sql ]]; then
  psql "$DATABASE_URL" -f ./fleet-server/migrations/seed.sql
  echo "✔ seeded via fleet-server/migrations/seed.sql"
elif [[ -f ./seed/dev_seed.sql ]]; then
  psql "$DATABASE_URL" -f ./seed/dev_seed.sql
  echo "✔ seeded via seed/dev_seed.sql"
else
  echo "✘ no seed binary (src/bin/seed.rs) or seed SQL file found."
  echo "  Create one of those, then re-run this script."
  exit 1
fi
