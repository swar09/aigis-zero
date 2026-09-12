# API backend deployment and operations guide

This guide covers the configuration, deployment, runtime workflows, and operational maintenance of `edr-api-backend`. The service provides REST endpoints for security operations, delivers real-time telemetry over WebSockets, coordinates host quarantine commands with the Fleet Server, and queries partitioned PostgreSQL databases.

## 1. Subsystem architecture

The API backend separates responsibilities into four layers: HTTP handlers, business services, database repositories, and asynchronous background workers.

```mermaid
graph TD
    subgraph Clients ["Operator Layer"]
        UI_REST["Console REST Client"]
        UI_WS["Console WebSocket Client"]
    end

    subgraph Backend ["edr-api-backend (Axum 0.8)"]
        AuthGuard["JWT Auth Guard & Trace Layer"]
        
        subgraph Handlers ["HTTP & WS Handlers"]
            H_Auth["/api/v1/auth"]
            H_Nodes["/api/v1/nodes"]
            H_Alerts["/api/v1/alerts"]
            H_Logs["/api/v1/logs"]
            H_WS["/api/v1/ws"]
            H_Health["/healthz & /readyz"]
        end

        subgraph Services ["Service Layer"]
            S_Auth["AuthService"]
            S_Nodes["NodeService"]
            S_Alerts["AlertService"]
            S_Logs["LogService"]
        end

        subgraph Repositories ["Diesel-Async Repositories"]
            R_Nodes["DieselNodeRepository"]
            R_Alerts["DieselAlertRepository"]
            R_Logs["DieselLogRepository"]
        end

        subgraph Streaming ["Live Event Pipeline"]
            BroadcastHub["tokio::sync::broadcast Hub"]
            KafkaWorker["rdkafka StreamConsumer Task"]
        end

        FleetClient["Fleet gRPC Client (Tonic)"]
    end

    subgraph Storage ["Infrastructure"]
        DB_Nodes[("PostgreSQL edr_nodes :5433")]
        DB_Alerts[("PostgreSQL edr_alerts :5434")]
        DB_Logs[("PostgreSQL edr_logs :5435")]
        KafkaCluster{{"Kafka Cluster :9092"}}
        FleetServer["Fleet Server :50051"]
    end

    UI_REST --> AuthGuard
    AuthGuard --> Handlers
    H_Auth --> S_Auth
    H_Nodes --> S_Nodes
    H_Alerts --> S_Alerts
    H_Logs --> S_Logs

    S_Nodes --> R_Nodes
    S_Nodes --> FleetClient
    S_Alerts --> R_Alerts
    S_Logs --> R_Logs

    R_Nodes --> DB_Nodes
    R_Alerts --> DB_Alerts
    R_Logs --> DB_Logs

    FleetClient -->|"gRPC IsolateCommand"| FleetServer
    KafkaCluster --> KafkaWorker
    KafkaWorker --> BroadcastHub
    BroadcastHub --> H_WS
    UI_WS <--> H_WS
```

### Architectural design principles

1. Direct database isolation: The service connects to three PostgreSQL databases through distinct connection pools. No query crosses database boundaries.
2. Non-blocking live telemetry: Incoming Kafka events bypass database storage and stream directly into in-memory broadcast channels. High-rate endpoint telemetry cannot exhaust database connection pools.
3. Decoupled control plane: The API backend does not communicate with endpoint agents directly. When an operator requests an action (such as host quarantine), the backend delegates command execution to the Fleet Server over gRPC.

## 2. Configuration parameters

Configuration is managed through environment variables. The service loads `.env` from the crate directory or repository root.

```bash
# Server binding
HOST=0.0.0.0
PORT=8080

# Partitioned database pools
DATABASE_URL_NODES=postgres://edr:edrpassword@localhost:5433/edr_nodes
DATABASE_URL_ALERTS=postgres://edr:edrpassword@localhost:5434/edr_alerts
DATABASE_URL_LOGS=postgres://edr:edrpassword@localhost:5435/edr_logs
DB_POOL_MAX_SIZE=16

# Kafka live stream consumption
KAFKA_BROKERS=localhost:9092
KAFKA_CONSUMER_GROUP=edr-api-backend-live

# Fleet Server gRPC control plane
FLEET_GRPC_URL=http://localhost:50051

# Operator authentication
JWT_SECRET=super_secret_jwt_key_replace_in_production_32_bytes_min
JWT_EXPIRATION_SECS=86400
ADMIN_DEFAULT_USER=admin
ADMIN_DEFAULT_PASSWORD=admin_password_change_in_production

# Logging and diagnostics
RUST_LOG=info,edr_api_backend=debug
```

## 3. Operator workflows and endpoint containment

### Authentication

Operators authenticate using Argon2-verified credentials. On success, the API issues an HMAC-SHA256 JWT valid for 24 hours:

```bash
# Login request
curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin_password_change_in_production"}' | jq .

# Example response
# {
#   "success": true,
#   "data": {
#     "token": "eyJhbGciOiJIUzI1NiIsIn...",
#     "expires_at": 1773414000
#   }
# }
```

Pass this token in the `Authorization: Bearer <TOKEN>` header for all authenticated requests.

### Endpoint quarantine workflow

When an analyst identifies a compromised machine, they trigger network isolation:

1. Analyst issues quarantine command:
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/nodes/<NODE_UUID>/isolate \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"reason": "Suspicious reverse shell detected"}' | jq .
   ```

2. `NodeService` executes two steps:
   - Updates `operator_status` to `isolated` in `edr_nodes`.
   - Invokes `FleetClient::send_isolate_command(node_id, true, reason)` via gRPC to `fleet-server`.

3. `fleet-server` sends an `IsolateCommand(true)` over the bidirectional gRPC stream to the agent on the target machine.

4. The endpoint agent applies `nftables` packet filtering rules, severing all ingress and egress network traffic except the gRPC link back to the Fleet Server.

### Releasing quarantine

To restore normal network access:

```bash
curl -s -X POST http://localhost:8080/api/v1/nodes/<NODE_UUID>/unisolate \
  -H "Authorization: Bearer $TOKEN" | jq .
```

`NodeService` updates `operator_status` to `active` and dispatches `IsolateCommand(false)` to the agent, which flushes its isolation rules.

### Alert triage workflow

Analysts query, inspect, and update threat detections:

```bash
# List open critical alerts
curl -s "http://localhost:8080/api/v1/alerts?severity=critical&status=open" \
  -H "Authorization: Bearer $TOKEN" | jq .

# Update alert triage status to acknowledged
curl -s -X PATCH http://localhost:8080/api/v1/alerts/<ALERT_ID>/status \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"status": "acknowledged"}' | jq .
```

## 4. Real-time WebSocket streaming

The endpoint `GET /api/v1/ws` provides bidirectional streaming using WebSockets.

### Connection parameters

Connect with query parameters:
`ws://localhost:8080/api/v1/ws?topics=alerts,logs,heartbeats&node_id=<OPTIONAL_UUID>`

### Subscription filters

Clients can update topic subscriptions over the open connection by sending a JSON frame:

```json
{
  "action": "subscribe",
  "topics": ["alerts", "logs"]
}
```

The server broadcasts incoming Kafka events matching the subscription directly to the client.

## 5. Deployment procedures

### Running in Docker

Build and start using Docker Compose:

```bash
cd infra
docker compose up -d api-backend
```

Inspect container logs:
```bash
docker logs -f edr-api-backend
```

### Pulling and running pre-built Docker image

Pull the published image from GitHub Container Registry:

```bash
docker pull ghcr.io/swar09/aigis-api-backend:latest
```

Run standalone container with environment file:

```bash
docker run -d \
  --name edr-api-backend \
  --restart unless-stopped \
  -p 8080:8080 \
  --env-file .env \
  ghcr.io/swar09/aigis-api-backend:latest
```

### Running as a systemd service

For bare-metal or virtual machine deployments, create `/etc/systemd/system/edr-api-backend.service`:

```ini
[Unit]
Description=Aigis-Zero EDR API Backend
After=network.target

[Service]
Type=simple
User=edr
Group=edr
WorkingDirectory=/opt/aigis-zero/api-backend
EnvironmentFile=/etc/aigis-zero/api-backend.env
ExecStart=/opt/aigis-zero/bin/edr-api-backend
Restart=always
RestartSec=5
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now edr-api-backend
sudo systemctl status edr-api-backend
```

## 6. Health and readiness probes

The backend exposes standard probes for load balancers and orchestrators:

- `GET /healthz`: Basic process liveness probe. Returns HTTP 200 `{"status": "ok"}`.
- `GET /readyz`: Deep readiness probe. Verifies active connections to `edr_nodes`, `edr_alerts`, and `edr_logs`. Returns HTTP 200 when all pools respond, or HTTP 503 if any pool is unavailable.

Verify readiness:
```bash
curl -s -i http://localhost:8080/readyz
```

## 7. Operational troubleshooting

### Database pool exhaustion
- Symptom: Requests time out with HTTP 500 and log message `PoolCheckoutTimeout`.
- Cause: Queries taking too long or insufficient pool size under high concurrency.
- Fix: Increase `DB_POOL_MAX_SIZE` in `.env` (default is 16) and verify database CPU and I/O utilization.

### Fleet Server command failures
- Symptom: Isolation requests return success for database status but log `Fleet server command dispatch failed`.
- Cause: `fleet-server` is unreachable or the target agent is disconnected.
- Fix: Verify `FLEET_GRPC_URL` is set to `http://localhost:50051` (host) or `http://fleet-server:50051` (Docker). Check fleet server logs for gRPC connection errors.

### WebSocket drops under load
- Symptom: WebSocket clients disconnect with `Lagged` error.
- Cause: Client is consuming slower than the broadcast channel capacity.
- Fix: Ensure the client reads and processes frames promptly without blocking the network reader.
