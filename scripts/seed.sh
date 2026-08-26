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

# Database URLs with defaults matching .env
DATABASE_URL_NODES="${DATABASE_URL_NODES:-postgres://edr:edrpassword@localhost:5433/edr_nodes}"
DATABASE_URL_ALERTS="${DATABASE_URL_ALERTS:-postgres://edr:edrpassword@localhost:5434/edr_alerts}"
DATABASE_URL_LOGS="${DATABASE_URL_LOGS:-postgres://edr:edrpassword@localhost:5435/edr_logs}"

if $RESET; then
  echo "▶ resetting database schema"
  if command -v sqlx >/dev/null 2>&1; then
    DATABASE_URL="$DATABASE_URL_NODES" sqlx database drop -y || true
    DATABASE_URL="$DATABASE_URL_NODES" sqlx database create
    DATABASE_URL="$DATABASE_URL_NODES" sqlx migrate run --source fleet-server/migrations
  fi
fi

echo "▶ seeding databases"
if command -v psql >/dev/null 2>&1; then
  # 1. edr_nodes
  psql "$DATABASE_URL_NODES" -f ./fleet-server/migrations/20260601000001_create_nodes.sql >/dev/null 2>&1 || true
  psql "$DATABASE_URL_NODES" -f ./fleet-server/migrations/20260601000002_create_enrollment_events.sql >/dev/null 2>&1 || true
  psql "$DATABASE_URL_NODES" -f ./fleet-server/migrations/20260601000003_create_node_health.sql >/dev/null 2>&1 || true
  psql "$DATABASE_URL_NODES" -f ./fleet-server/migrations/seed.sql >/dev/null 2>&1 || true
  echo "✔ seeded edr_nodes"

  # 2. edr_alerts
  if [[ -f ./infra/db/init_alerts.sql ]]; then
    psql "$DATABASE_URL_ALERTS" -f ./infra/db/init_alerts.sql >/dev/null 2>&1 || true
    psql "$DATABASE_URL_ALERTS" -f ./infra/db/seed_alerts.sql >/dev/null 2>&1 || true
    echo "✔ seeded edr_alerts"
  fi

  # 3. edr_logs
  if [[ -f ./infra/db/init_logs.sql ]]; then
    psql "$DATABASE_URL_LOGS" -f ./infra/db/init_logs.sql >/dev/null 2>&1 || true
    psql "$DATABASE_URL_LOGS" -f ./infra/db/seed_logs.sql >/dev/null 2>&1 || true
    echo "✔ seeded edr_logs"
  fi
elif cargo run --bin seed --quiet 2>/dev/null; then
  echo "✔ seeded via cargo run --bin seed"
else
  echo "✘ psql not found on host. Tip: use './scripts/infra.sh seed' or './scripts/infra.sh up' to seed directly via Docker."
fi

