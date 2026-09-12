#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="${REPO_ROOT}/infra/docker-compose.yml"
COMPOSE_PROD_FILE="${REPO_ROOT}/docker-compose.prod.yml"

BLUE='\033[1;34m'
GREEN='\033[1;32m'
YELLOW='\033[1;33m'
RED='\033[1;31m'
NC='\033[0m'

log_step() { echo -e "\n${BLUE}▶ $1${NC}"; }
log_ok()   { echo -e "${GREEN}✔ $1${NC}"; }
log_warn() { echo -e "${YELLOW}⚠ $1${NC}"; }
log_fail() { echo -e "${RED}✘ $1${NC}"; exit 1; }

if [[ -f "${REPO_ROOT}/.env" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "${REPO_ROOT}/.env"
  set +a
fi

POSTGRES_USER="${POSTGRES_USER:-edr}"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-edrpassword}"

check_docker() {
  if ! command -v docker > /dev/null 2>&1; then
    log_fail "Docker is not installed or not in PATH."
  fi
  if ! docker info > /dev/null 2>&1; then
    log_fail "Docker daemon is not running or current user lacks permissions."
  fi
}

wait_for_service() {
  local container="$1"
  local max_retries="${2:-30}"
  local count=0

  echo -n "  Waiting for container '${container}' to become healthy..."
  while [[ $count -lt $max_retries ]]; do
    local health_status
    health_status=$(docker inspect --format='{{json .State.Health.Status}}' "$container" 2>/dev/null || echo '"unknown"')
    if [[ "$health_status" == '"healthy"' ]]; then
      echo -e " ${GREEN}healthy${NC}"
      return 0
    elif [[ "$health_status" == '"unhealthy"' ]]; then
      echo -e " ${RED}unhealthy${NC}"
      return 1
    fi
    sleep 1
    count=$((count + 1))
    echo -n "."
  done
  echo -e " ${YELLOW}timed out (proceeding)${NC}"
  return 0
}

apply_seeds() {
  log_step "Applying DDL schemas and seeding mock data into all 3 PostgreSQL databases..."

  echo "  --> Initializing [edr_nodes] (nodes, node_health, enrollment_events)..."
  docker exec -i edr-postgres-nodes psql -U "$POSTGRES_USER" -d edr_nodes -f - < "${REPO_ROOT}/fleet-server/migrations/20260601000001_create_nodes.sql" > /dev/null
  docker exec -i edr-postgres-nodes psql -U "$POSTGRES_USER" -d edr_nodes -f - < "${REPO_ROOT}/fleet-server/migrations/20260601000002_create_enrollment_events.sql" > /dev/null
  docker exec -i edr-postgres-nodes psql -U "$POSTGRES_USER" -d edr_nodes -f - < "${REPO_ROOT}/fleet-server/migrations/20260601000003_create_node_health.sql" > /dev/null
  docker exec -i edr-postgres-nodes psql -U "$POSTGRES_USER" -d edr_nodes -f - < "${REPO_ROOT}/fleet-server/migrations/seed.sql" > /dev/null
  log_ok "edr_nodes schema & seed applied."

  echo "  --> Initializing [edr_alerts] (detection alerts)..."
  docker exec -i edr-postgres-alerts psql -U "$POSTGRES_USER" -d edr_alerts -f - < "${REPO_ROOT}/infra/db/init_alerts.sql" > /dev/null
  docker exec -i edr-postgres-alerts psql -U "$POSTGRES_USER" -d edr_alerts -f - < "${REPO_ROOT}/infra/db/seed_alerts.sql" > /dev/null
  log_ok "edr_alerts schema & seed applied."

  echo "  --> Initializing [edr_logs] (event_logs telemetry)..."
  docker exec -i edr-postgres-logs psql -U "$POSTGRES_USER" -d edr_logs -f - < "${REPO_ROOT}/infra/db/init_logs.sql" > /dev/null
  docker exec -i edr-postgres-logs psql -U "$POSTGRES_USER" -d edr_logs -f - < "${REPO_ROOT}/infra/db/seed_logs.sql" > /dev/null
  log_ok "edr_logs schema & seed applied."
}

create_kafka_topics() {
  log_step "Verifying Kafka Topics..."
  if docker ps --format '{{.Names}}' | grep -q "^edr-kafka$"; then
    for topic in aigis.events.raw aigis.events.process aigis.events.network aigis.events.file aigis.events.auth aigis.events.norm aigis.heartbeats aigis.alerts aigis.health aigis.events.dlq; do
      docker exec -i edr-kafka kafka-topics --bootstrap-server localhost:9092 --create --if-not-exists --topic "$topic" --partitions 4 --replication-factor 1 > /dev/null 2>&1 || true
    done
    log_ok "Kafka topics initialized."
  fi
}

print_ready_banner() {
  local api_port="${PORT:-8080}"
  echo ""
  echo -e "${GREEN}========================================================================${NC}"
  echo -e "${GREEN}       Aigis-Zero Infrastructure & Seed Data is Ready!                  ${NC}"
  echo -e "${GREEN}========================================================================${NC}"
  echo -e "  • ${BLUE}edr_nodes Database:${NC}  localhost:5433 (user: ${POSTGRES_USER})"
  echo -e "  • ${BLUE}edr_alerts Database:${NC} localhost:5434 (user: ${POSTGRES_USER})"
  echo -e "  • ${BLUE}edr_logs Database:${NC}   localhost:5435 (user: ${POSTGRES_USER})"
  echo -e "  • ${BLUE}Kafka Broker:${NC}        localhost:9092"
  echo -e "  • ${BLUE}Kafka UI:${NC}            http://localhost:8090"
  echo -e "  • ${BLUE}API Backend:${NC}         http://localhost:${api_port}"
  echo -e "  • ${BLUE}Fleet Server gRPC:${NC}   http://localhost:50051"
  echo -e "${GREEN}========================================================================${NC}"
  echo -e "  ${YELLOW}Quick API Health Check:${NC}"
  echo -e "    curl -s http://localhost:${api_port}/healthz | jq ."
  echo -e "    curl -s http://localhost:${api_port}/readyz | jq ."
  echo ""
  echo -e "  ${YELLOW}Quick Operator Login & List Nodes:${NC}"
  echo -e "    TOKEN=\$(curl -s -X POST http://localhost:${api_port}/api/v1/auth/login \\"
  echo -e "      -H 'Content-Type: application/json' \\"
  echo -e "      -d '{\"username\":\"admin\",\"password\":\"admin\"}' | jq -r '.data.token')"
  echo -e "    curl -s http://localhost:${api_port}/api/v1/nodes -H \"Authorization: Bearer \$TOKEN\" | jq ."
  echo -e "    curl -s http://localhost:${api_port}/api/v1/alerts -H \"Authorization: Bearer \$TOKEN\" | jq ."
  echo -e "    curl -s http://localhost:${api_port}/api/v1/logs -H \"Authorization: Bearer \$TOKEN\" | jq ."
  echo -e "${GREEN}========================================================================${NC}\n"
}

cmd_up() {
  local use_pull=false
  for arg in "$@"; do
    [[ "$arg" == "--pull" ]] && use_pull=true
  done

  check_docker

  if [[ "$use_pull" == true ]]; then
    log_step "Pull mode: using production manifest (docker-compose.prod.yml) and pulling images from GHCR..."
    docker compose -f "$COMPOSE_PROD_FILE" up -d --pull always
  else
    if [[ ! -f "${REPO_ROOT}/rule-engine/rules/mitre/enterprise-attack-linux.json" ]]; then
      log_step "Fetching MITRE ATT&CK taxonomy and community YARA rules..."
      "${REPO_ROOT}/scripts/fetch-rules.sh"
    fi
    log_step "Build mode: building all app images from local Dockerfiles..."
    docker compose -f "$COMPOSE_FILE" up -d --build
  fi

  log_step "Waiting for container health checks..."
  wait_for_service "edr-zookeeper"
  wait_for_service "edr-kafka"
  wait_for_service "edr-postgres-nodes"
  wait_for_service "edr-postgres-alerts"
  wait_for_service "edr-postgres-logs"

  create_kafka_topics

  if [[ "$use_pull" == false ]]; then
    apply_seeds
  fi

  print_ready_banner
}

cmd_down() {
  local use_pull=false
  for arg in "$@"; do
    [[ "$arg" == "--pull" ]] && use_pull=true
  done

  check_docker
  log_step "Stopping infrastructure containers..."
  if [[ "$use_pull" == true ]]; then
    docker compose -f "$COMPOSE_PROD_FILE" down
  else
    docker compose -f "$COMPOSE_FILE" down
  fi
  log_ok "Containers stopped."
}

cmd_reset() {
  check_docker
  log_step "Resetting all infrastructure & database volumes..."
  docker compose -f "$COMPOSE_FILE" down -v
  log_ok "Volumes removed."
  cmd_up "$@"
}

cmd_status() {
  check_docker
  log_step "Infrastructure Container Status:"
  docker compose -f "$COMPOSE_FILE" ps
}

ACTION="${1:-up}"
shift || true

case "$ACTION" in
  up)
    cmd_up "$@"
    ;;
  down)
    cmd_down "$@"
    ;;
  reset)
    cmd_reset "$@"
    ;;
  seed)
    check_docker
    apply_seeds
    ;;
  status)
    cmd_status
    ;;
  help|--help|-h)
    echo "Usage: ./scripts/infra.sh [up|down|reset|seed|status|help] [--pull]"
    echo ""
    echo "Commands:"
    echo "  up [--pull]   Start containers. Default: build from local Dockerfiles."
    echo "                Pass --pull to pull production images from GHCR instead."
    echo "  down [--pull] Stop containers (use --pull if started with --pull)."
    echo "  reset         Wipe volumes and restart with local builds."
    echo "  seed          Re-apply DDL and seed data to all PostgreSQL databases."
    echo "  status        Show container health and port map."
    ;;
  *)
    log_fail "Unknown action '$ACTION'. Run './scripts/infra.sh help' for usage."
    ;;
esac
