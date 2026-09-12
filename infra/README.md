# Aigis-Zero infrastructure

The infrastructure layer coordinates containerized databases, the Kafka message broker, topic management scripts, and Kubernetes deployment configurations for the Aigis-Zero EDR platform.

For the complete operations, disaster recovery, and deployment manual, refer to the [Infrastructure guide](guide.md).

## Quick start

The fastest way to boot, verify, and seed the entire infrastructure is using the repository script:

```bash
# Start containers, await health checks, create topics, and apply database seeds
./scripts/infra.sh up
```

Alternatively, use standard Docker Compose:

```bash
cd infra
docker compose up -d
```

## Services and ports

| Service | Container Name | Host Port | Database / Web UI |
|---|---|---|---|
| PostgreSQL (nodes) | `edr-postgres-nodes` | 5433 | `edr_nodes` (Node inventory & health) |
| PostgreSQL (alerts) | `edr-postgres-alerts` | 5434 | `edr_alerts` (Detection alerts) |
| PostgreSQL (logs) | `edr-postgres-logs` | 5435 | `edr_logs` (Raw & normalized telemetry) |
| Apache Kafka Broker | `edr-kafka` | 9092 | Event bus (`kafka:29092` internal) |
| Kafka UI | `edr-kafka-ui` | 8090 | [http://localhost:8090](http://localhost:8090) |
| Fleet Server | `edr-fleet-server` | 50051 | Tonic gRPC control plane |
| API Backend | `edr-api-backend` | 8080 | REST & WebSocket gateway |
| Rule Engine | `edr-rule-engine` | 8081 | YARA-X scanning service |
| Kafka Pipeline | `edr-kafka-pipeline` | 8082 | Event normalization router |

## Operations reference

Use `./scripts/infra.sh` for lifecycle management:

- `./scripts/infra.sh up`: Start all containers and seed databases.
- `./scripts/infra.sh down`: Stop all containers.
- `./scripts/infra.sh status`: Show container health checks and port mappings.
- `./scripts/infra.sh seed`: Re-apply schemas and mock fixtures.
- `./scripts/infra.sh reset`: Wipe persistent volumes and rebuild from scratch.

## Additional resources

- [Infrastructure operations guide](guide.md): In-depth topology, Kafka partitioning, KEDA autoscaling, and backup runbooks.
- [Database initialization scripts](db/): DDL files and seed data for alerts and event logs.
- [Kafka topic provisioning](scripts/create-topics.sh): Partition and retention policy definitions.
- [Kubernetes KEDA scaler](k8s/keda-scaler.yml): Consumer group autoscaling specifications.
