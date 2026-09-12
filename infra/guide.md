# Infrastructure deployment and operations guide

This guide provides the complete operational reference for deploying, configuring, and maintaining the Aigis-Zero infrastructure layer. It covers Docker Compose services, partitioned PostgreSQL databases, Apache Kafka messaging topologies, Kubernetes autoscaling with KEDA, and disaster recovery procedures.

## 1. System architecture and network topology

The Aigis-Zero infrastructure isolates state across dedicated database instances and an Apache Kafka message bus. Telemetry flows asynchronously from endpoints to storage and detection consumers.

```mermaid
graph TD
    subgraph Storage ["Database Tier (PostgreSQL 16)"]
        DB_Nodes["postgres-nodes:5433<br/>Database: edr_nodes<br/>Role: Registry & Enrollment"]
        DB_Alerts["postgres-alerts:5434<br/>Database: edr_alerts<br/>Role: Detections & MITRE"]
        DB_Logs["postgres-logs:5435<br/>Database: edr_logs<br/>Role: Telemetry Archive"]
    end

    subgraph Messaging ["Message Bus (Apache Kafka 7.6)"]
        ZK["zookeeper:2181"]
        KFK["kafka:9092 (host) / 29092 (internal)"]
        KUI["kafka-ui:8090"]
        ZK --- KFK
        KFK --- KUI
    end

    subgraph Services ["Application Runtime"]
        Fleet["fleet-server:50051 (gRPC)"]
        Pipeline["kafka-pipeline:8082 (HTTP)"]
        Engine["rule-engine:8081 (HTTP)"]
        API["api-backend:8080 (HTTP/WS)"]
    end

    Fleet -->|"Writes nodes & health"| DB_Nodes
    Fleet -->|"Publishes aigis.events.raw"| KFK
    KFK -->|"Consumes raw events"| Pipeline
    Pipeline -->|"Fans out typed topics"| KFK
    KFK -->|"Consumes process/network/file/auth"| Engine
    Engine -->|"Persists alerts"| DB_Alerts
    Engine -->|"Publishes aigis.alerts"| KFK
    API -->|"Queries inventory"| DB_Nodes
    API -->|"Queries alerts"| DB_Alerts
    API -->|"Queries telemetry logs"| DB_Logs
    KFK -->|"Live feeds to WebSocket"| API
    API -->|"Dispatches quarantine"| Fleet
```

### Port and network assignments

| Service | Container Name | Host Port | Internal Port | Protocol | Purpose |
|---|---|---|---|---|---|
| PostgreSQL (nodes) | `edr-postgres-nodes` | 5433 | 5432 | TCP | Enrolled nodes, host health, enrollment audit events |
| PostgreSQL (alerts) | `edr-postgres-alerts` | 5434 | 5432 | TCP | Threat alerts, MITRE tactics, triage records |
| PostgreSQL (logs) | `edr-postgres-logs` | 5435 | 5432 | TCP | Historical event logs and telemetry records |
| Apache ZooKeeper | `edr-zookeeper` | - | 2181 | TCP | Kafka coordination metadata |
| Apache Kafka Broker | `edr-kafka` | 9092 | 29092 | TCP | Distributed event bus |
| Kafka UI | `edr-kafka-ui` | 8090 | 8080 | HTTP | Web management console for Kafka topics and consumer groups |
| Fleet Server | `edr-fleet-server` | 50051 | 50051 | gRPC (HTTP/2) | Agent enrollment, heartbeat, and telemetry ingestion |
| API Backend | `edr-api-backend` | 8080 | 8080 | HTTP / WS | REST API and WebSocket live stream gateway |
| Rule Engine | `edr-rule-engine` | 8081 | 8081 | HTTP | YARA-X scanning, MITRE enrichment, Prometheus metrics |
| Kafka Pipeline | `edr-kafka-pipeline` | 8082 | 8082 | HTTP | Event normalization router, Prometheus metrics |
| Frontend Console | `edr-frontend` | 3000 | 3000 | HTTP | Next.js operator user interface |

## 2. Environment configuration

All infrastructure components read settings from `.env` in the repository root. Copy the template before starting containers:

```bash
cp .env.example .env
```

Key environment variables:

```bash
# Database credentials
POSTGRES_USER=edr
POSTGRES_PASSWORD=edrpassword_change_in_production
DB_POOL_MAX_SIZE=20

# Database URLs for host-level tools and microservices
DATABASE_URL_NODES=postgres://edr:edrpassword_change_in_production@localhost:5433/edr_nodes
DATABASE_URL_ALERTS=postgres://edr:edrpassword_change_in_production@localhost:5434/edr_alerts
DATABASE_URL_LOGS=postgres://edr:edrpassword_change_in_production@localhost:5435/edr_logs

# Kafka settings
KAFKA_BROKERS=localhost:9092
KAFKA_CONSUMER_GROUP=edr-api-backend-live
KAFKA_TOPIC_AGENTS_EVENTS=aigis.events.raw

# Service control plane endpoints
FLEET_GRPC_URL=http://localhost:50051
HOST=0.0.0.0
PORT=8080
HEALTH_PORT=8081
```

## 3. Operations with scripts/infra.sh

The `scripts/infra.sh` tool automates container lifecycle, health checking, topic provisioning, and schema migration.

### Starting the infrastructure

```bash
./scripts/infra.sh up
```

This command executes the following workflow:
1. Runs `docker compose -f infra/docker-compose.yml up -d`.
2. Polls health check endpoints for ZooKeeper, Kafka, and the three PostgreSQL containers until healthy.
3. Automatically provisions all standard Kafka topics with configured partition counts.
4. Applies DDL schemas and mock seed data to `edr_nodes`, `edr_alerts`, and `edr_logs`.
5. Displays ready-to-use curl commands and connection URLs.

### Checking health status

```bash
./scripts/infra.sh status
```

Displays container runtime state and port bindings.

### Re-applying seed fixtures

To reseed sample nodes, alerts, and telemetry logs without recreating containers:

```bash
./scripts/infra.sh seed
```

### Resetting environment and data volumes

To wipe all data volumes (Kafka topics and PostgreSQL tables) and boot fresh:

```bash
./scripts/infra.sh reset
```

### Tearing down containers

```bash
./scripts/infra.sh down
```

## 4. Deploying with pre-built Docker images from GitHub Container Registry (ghcr.io)

For production environments where building from source is undesirable, pre-built production images are published automatically to the GitHub Container Registry (`ghcr.io`) on every main branch commit and release tag.

### Published container images

| Service | Image Repository | Default Port | Base OS |
|---|---|---|---|
| API Backend | `ghcr.io/swar09/aigis-api-backend:latest` | 8080 | Debian Slim (Distroless runtime) |
| Fleet Server | `ghcr.io/swar09/aigis-fleet-server:latest` | 50051 | Debian Slim (Distroless runtime) |
| Kafka Pipeline | `ghcr.io/swar09/aigis-kafka-pipeline:latest` | 8082 | Debian Slim (Distroless runtime) |
| Rule Engine | `ghcr.io/swar09/aigis-rule-engine:latest` | 8081 | Debian Slim (Distroless runtime) |
| Frontend Console | `ghcr.io/swar09/aigis-frontend:latest` | 3000 | Node 20 Alpine (Standalone) |
| Endpoint Agent | `ghcr.io/swar09/aigis-agent:latest` | - | Alpine (Musl static) |

### Authenticating and pulling images

```bash
# Optional: Authenticate to GHCR (required if repository is private)
echo "$CR_PAT" | docker login ghcr.io -u <GITHUB_USERNAME> --password-stdin

# Pull all production service images
docker pull ghcr.io/swar09/aigis-api-backend:latest
docker pull ghcr.io/swar09/aigis-fleet-server:latest
docker pull ghcr.io/swar09/aigis-kafka-pipeline:latest
docker pull ghcr.io/swar09/aigis-rule-engine:latest
docker pull ghcr.io/swar09/aigis-frontend:latest
```

### Running pre-built images with Docker Compose

To run pre-built images without compiling code locally, set the image tags in `infra/docker-compose.yml` or run with an environment override file:

```bash
# Export the registry prefix
export REGISTRY=ghcr.io/swar09
export TAG=latest

# Start all core data services and application containers
docker compose -f infra/docker-compose.yml up -d
```

### Running standalone pre-built containers

Each container reads configuration directly from an environment file:

```bash
# Example: Running the Fleet Server container
docker run -d \
  --name edr-fleet-server \
  --restart unless-stopped \
  --network host \
  --env-file .env \
  ghcr.io/swar09/aigis-fleet-server:latest

# Example: Running the API Backend container
docker run -d \
  --name edr-api-backend \
  --restart unless-stopped \
  -p 8080:8080 \
  --env-file .env \
  ghcr.io/swar09/aigis-api-backend:latest

# Example: Running the Frontend Console container
docker run -d \
  --name edr-frontend \
  --restart unless-stopped \
  -p 3000:3000 \
  -e BACKEND_URL=http://localhost:8080 \
  ghcr.io/swar09/aigis-frontend:latest
```

## 5. Kafka topic topology and partition strategy

Topics are partitioned to enable parallel consumption across worker instances. Partition keys preserve ordering per endpoint:

```
aigis.events.raw (12 partitions, key: node_id)
       |
       +--> aigis.events.process (12 partitions, key: node_id) -> rule-engine
       +--> aigis.events.network (12 partitions, key: node_id) -> rule-engine
       +--> aigis.events.file    (8 partitions,  key: node_id) -> rule-engine
       +--> aigis.events.auth    (8 partitions,  key: node_id) -> rule-engine
       +--> aigis.events.norm    (12 partitions, key: node_id) -> long-term storage
       +--> aigis.events.dlq     (4 partitions)                -> error inspection
```

Topic specifications:

| Topic | Partitions | Retention Hours | Purpose |
|---|---|---|---|
| `aigis.events.raw` | 12 | 168 (7 days) | Ingested telemetry stream from Fleet Server |
| `aigis.events.process` | 12 | 168 (7 days) | Process execution events (`process_start`, `process_terminate`) |
| `aigis.events.network` | 12 | 168 (7 days) | Socket events (`socket_connect`, `listen`) |
| `aigis.events.file` | 8 | 168 (7 days) | File integrity monitoring events |
| `aigis.events.auth` | 8 | 168 (7 days) | Login and privilege elevation events |
| `aigis.events.norm` | 12 | 168 (7 days) | Normalized schema telemetry for warehouse ingestion |
| `aigis.heartbeats` | 4 | 72 (3 days) | Endpoint heartbeat events |
| `aigis.alerts` | 4 | 720 (30 days) | Threat detection alerts generated by the Rule Engine |
| `aigis.health` | 4 | 72 (3 days) | Health telemetry and agent status signals |
| `aigis.events.dlq` | 4 | 720 (30 days) | Poison pill messages with error headers |

Manual topic creation script:
```bash
bash infra/scripts/create-topics.sh
```

## 6. Kubernetes deployment and autoscaling with KEDA

For Kubernetes production clusters, the manifests in `infra/k8s/` manage automatic scaling based on Kafka lag.

### KEDA ScaledObject configuration

The manifest `infra/k8s/keda-scaler.yml` monitors consumer group lag on the Kafka cluster and scales worker pods up or down:

```yaml
apiVersion: keda.sh/v1alpha1
kind: ScaledObject
metadata:
  name: kafka-rule-engine-scaler
  namespace: aigis-system
spec:
  scaleTargetRef:
    name: edr-rule-engine
  minReplicaCount: 2
  maxReplicaCount: 12
  cooldownPeriod: 30
  pollingInterval: 15
  triggers:
    - type: kafka
      metadata:
        bootstrapServers: kafka.aigis-system.svc.cluster.local:9092
        consumerGroup: aigis-rule-engine
        topic: aigis.events.process
        lagThreshold: "100"
```

Apply the scaler:
```bash
kubectl apply -f infra/k8s/keda-scaler.yml
```

### Verification

Check current scaler status and HPA targets:
```bash
kubectl get scaledobjects -n aigis-system
kubectl get hpa -n aigis-system
```

## 7. Database maintenance, backup, and restore

### Manual database dumps

Export logical backups for all three database instances:

```bash
# Export nodes registry
docker exec -t edr-postgres-nodes pg_dump -U edr -d edr_nodes -F c -f /tmp/nodes_backup.dump
docker cp edr-postgres-nodes:/tmp/nodes_backup.dump ./backups/nodes_$(date +%Y%m%d).dump

# Export alerts database
docker exec -t edr-postgres-alerts pg_dump -U edr -d edr_alerts -F c -f /tmp/alerts_backup.dump
docker cp edr-postgres-alerts:/tmp/alerts_backup.dump ./backups/alerts_$(date +%Y%m%d).dump

# Export logs database
docker exec -t edr-postgres-logs pg_dump -U edr -d edr_logs -F c -f /tmp/logs_backup.dump
docker cp edr-postgres-logs:/tmp/logs_backup.dump ./backups/logs_$(date +%Y%m%d).dump
```

### Restoring a database dump

```bash
# Example restoring edr_nodes
docker cp ./backups/nodes_20260601.dump edr-postgres-nodes:/tmp/restore.dump
docker exec -i edr-postgres-nodes pg_restore -U edr -d edr_nodes --clean --if-exists /tmp/restore.dump
```

## 8. Troubleshooting infrastructure issues

### Kafka broker connection failure
- Symptom: Services log `BrokerTransportFailure` or `Connection refused: localhost:9092`.
- Check: Ensure Kafka container is healthy with `docker inspect --format='{{json .State.Health.Status}}' edr-kafka`.
- Note on ports: If running services on the host machine, connect to `localhost:9092`. If running services inside Docker Compose, connect to `kafka:29092`.

### PostgreSQL port collisions
- Symptom: `bind: address already in use` when starting postgres containers.
- Check: Verify whether local PostgreSQL instances are already running on ports 5433, 5434, or 5435:
  ```bash
  sudo lsof -i :5433 -i :5434 -i :5435
  ```
- Change host ports in `.env` if needed.

### Kafka topic partition imbalance
- Open Kafka UI at [http://localhost:8090](http://localhost:8090).
- Inspect consumer group offsets under the Consumer Groups tab.
- If one partition has high lag, check that producer keys distribute events evenly across `node_id` hashes.
