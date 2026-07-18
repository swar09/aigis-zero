# Aigis-Zero EDR: API backend

The API backend is an asynchronous operator gateway built on Axum 0.8, Diesel-Async, and Tokio. It exposes REST endpoints for security operations, streams live telemetry over WebSockets, interfaces with the Fleet Server gRPC control plane, and queries PostgreSQL databases.

## Capabilities

* **REST security management**:
  * Node inventory with hardware stability tracking (`machine_id`) and status filtering.
  * Host quarantine (`isolate` and `unisolate`) triggering kernel-level `nftables` rules via the Fleet Server.
  * Detection alert triage with MITRE ATT&CK technique tags and severity scoring.
  * Range-based telemetry search across historical event logs.
* **Direct event streaming**:
  * Background `rdkafka` worker consumes from `aigis.events.*`, `aigis.alerts`, and `aigis.health`.
  * Real-time events bypass database writes and stream directly into in-memory `tokio::sync::broadcast` channels for WebSocket distribution.
* **Type-safe data access**:
  * Built on `diesel-async` with `deadpool` connection pooling.
  * Dedicated connection pools for `edr_nodes` (port 5433), `edr_alerts` (port 5434), and `edr_logs` (port 5432).
  * Connection checkout timeouts to prevent cascading latency stalls under heavy load.
* **Operator authentication**:
  * Argon2 password hashing.
  * Stateless HMAC-SHA256 JWT tokens with 24-hour expiration.
  * Separation between agent-reported health and operator-assigned containment status.

## Architecture

```mermaid
graph TB
    subgraph ClientLayer["Operator Layer"]
        UI_REST["SOC UI (REST Client)"]
        UI_WS["SOC Dashboard (WebSocket)"]
    end

    subgraph Backend["edr-api-backend (Axum 0.8)"]
        AuthMiddleware["JWT Auth Guard & Tracing"]
        
        subgraph Handlers["HTTP Handlers"]
            H_Auth["/api/v1/auth"]
            H_Nodes["/api/v1/nodes"]
            H_Alerts["/api/v1/alerts"]
            H_Logs["/api/v1/logs"]
        end

        subgraph Services["Service Layer"]
            S_Auth["AuthService"]
            S_Nodes["NodeService"]
            S_Alerts["AlertService"]
            S_Logs["LogService"]
        end

        subgraph Repositories["Diesel-Async Repositories"]
            R_Nodes["DieselNodeRepository"]
            R_Alerts["DieselAlertRepository"]
            R_Logs["DieselLogRepository"]
        end

        subgraph StreamingHub["Live Streaming Hub"]
            WSHub["/api/v1/ws Handler"]
            BroadcastChannel["tokio::sync::broadcast Hub"]
            KafkaWorker["rdkafka StreamConsumer Task"]
        end

        TonicClient["Fleet gRPC Client"]
    end

    subgraph Infrastructure["Storage & Message Bus"]
        DB_Nodes[("PostgreSQL: edr_nodes :5433")]
        DB_Alerts[("PostgreSQL: edr_alerts :5434")]
        DB_Logs[("PostgreSQL: edr_logs :5432")]
        KafkaCluster{{"Kafka Cluster :9092"}}
        FleetServer["Fleet Server :50051"]
    end

    UI_REST --> AuthMiddleware
    AuthMiddleware --> Handlers
    H_Auth --> S_Auth
    H_Nodes --> S_Nodes
    H_Alerts --> S_Alerts
    H_Logs --> S_Logs

    S_Nodes --> R_Nodes
    S_Nodes --> TonicClient
    S_Alerts --> R_Alerts
    S_Logs --> R_Logs

    R_Nodes --> DB_Nodes
    R_Alerts --> DB_Alerts
    R_Logs --> DB_Logs

    TonicClient --> FleetServer
    KafkaCluster --> KafkaWorker
    KafkaWorker --> BroadcastChannel
    BroadcastChannel --> WSHub
    UI_WS <--> WSHub
```

## API reference

### Authentication (`/api/v1/auth`)
* `POST /api/v1/auth/login`: Authenticates credentials and returns a Bearer JWT.
* `GET  /api/v1/auth/me`: Returns current operator session and permissions.

### Endpoint nodes (`/api/v1/nodes`)
* `GET  /api/v1/nodes`: List enrolled nodes with optional filters (`agent_status`, `operator_status`, `search`, `limit`, `offset`).
* `GET  /api/v1/nodes/:id`: Returns endpoint metadata and recent heartbeat history.
* `POST /api/v1/nodes/:id/isolate`: Quarantines host and dispatches `IsolateCommand(true)` to Fleet Server.
* `POST /api/v1/nodes/:id/unisolate`: Removes quarantine and dispatches `IsolateCommand(false)`.

### Threat detection alerts (`/api/v1/alerts`)
* `GET   /api/v1/alerts`: Query detection alerts with filters (`severity`, `status`, `mitre_technique`, `node_id`).
* `GET   /api/v1/alerts/:id`: Detailed alert breakdown.
* `PATCH /api/v1/alerts/:id/status`: Triage alert (`open`, `acknowledged`, `dismissed`).

### Telemetry logs (`/api/v1/logs`)
* `GET /api/v1/logs`: Search historical telemetry with filters (`node_id`, `event_type`, `from_timestamp`, `to_timestamp`).
* `GET /api/v1/logs/:id`: Single event payload view.

### Real-time live feed (`/api/v1/ws`)
* `GET /api/v1/ws`: WebSocket upgrade endpoint.
  * Query parameters: `?topics=alerts,logs,heartbeats&node_id=<UUID>`
  * Supports inbound subscription filtering frames and ping keepalives.

## Environment configuration

Configuration is loaded from `.env` in the crate root:

```env
HOST=0.0.0.0
PORT=8080

DATABASE_URL_NODES=postgres://edr:edrpassword@localhost:5433/edr_nodes
DATABASE_URL_ALERTS=postgres://edr:edrpassword@localhost:5434/edr_alerts
DATABASE_URL_LOGS=postgres://edr:edrpassword@localhost:5432/edr_logs
DB_POOL_MAX_SIZE=16

KAFKA_BROKERS=localhost:9092
KAFKA_CONSUMER_GROUP=edr-api-backend-live

FLEET_GRPC_URL=http://localhost:50051

JWT_SECRET=super_secret_jwt_key_replace_in_production_32_bytes_min
JWT_EXPIRATION_SECS=86400
ADMIN_DEFAULT_USER=admin
ADMIN_DEFAULT_PASSWORD=admin
```

## Running and testing

### Build and run
```bash
cargo run --bin edr-api-backend
```

### Run quality checks
```bash
./scripts/check.sh
```

### Run examples
```bash
cargo run --example generate_jwt
cargo run --example ws_listener
```
