# EDR Infra — Docker Compose Development Stack
Refer to `docker-compose.yml` for the full local development stack.

## Quick Start
```bash
# Set Postgres password
export POSTGRES_PASSWORD=changeme

# Start all infrastructure
docker-compose up -d

# Verify services
docker-compose ps
```

## Services
| Service | Port | Description |
|---|---|---|
| Kafka | 9092 | Message broker |
| PostgreSQL (logs) | 5432 | Event log storage |
| PostgreSQL (nodes) | 5433 | Node registry |
| PostgreSQL (alerts) | 5434 | Alert storage |
| Kafka UI | 8090 | Web UI for Kafka topics |
