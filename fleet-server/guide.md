# Fleet Server deployment and operations guide

This guide covers the architecture, node enrollment lifecycle, bidirectional streaming protocols, database schemas, and production deployment runbooks for the Aigis-Zero Fleet Server (`fleet-server`).

## 1. System role and architecture

The Fleet Server is the central gRPC control plane. It coordinates endpoint enrollment, authenticates agents, ingests telemetry streams into Apache Kafka, records health heartbeats in PostgreSQL, and routes containment commands down to endpoints.

```mermaid
sequenceDiagram
    autonumber
    participant Agent as Endpoint Agent
    participant Fleet as Fleet Server (:50051)
    participant DB as edr_nodes (:5433)
    participant Kafka as Kafka (aigis.events.raw)
    participant API as API Backend

    Note over Agent,Fleet: 1. Registration and Enrollment
    Agent->>Fleet: RegisterRequest (hostname, os, machine_id)<br/>Header: x-enrollment-secret
    Fleet->>Fleet: Validate pre-shared secret
    Fleet->>DB: INSERT INTO nodes & enrollment_events
    DB-->>Fleet: node_id UUID
    Fleet->>Fleet: Mint 24h HMAC-SHA256 JWT
    Fleet-->>Agent: RegisterResponse (node_id, JWT token)

    Note over Agent,Fleet: 2. Bidirectional Event Stream
    Agent->>Fleet: EventStream (stream AgentEvent)<br/>Header: Authorization Bearer JWT
    Fleet->>Fleet: Validate JWT claims
    loop Telemetry Ingestion
        Agent->>Fleet: AgentEvent (type, payload, sequence_id)
        Fleet->>Kafka: Produce record to aigis.events.raw (key: node_id)
        Fleet-->>Agent: ServerCommand::Ack (sequence_id)
        Agent->>Agent: Purge event from local SQLite WAL
    end

    Note over Agent,Fleet: 3. Liveness Heartbeat
    loop Every 60 seconds
        Agent->>Fleet: HeartbeatRequest (node_id, status, events_buffered)
        Fleet->>DB: UPDATE node_health & nodes.last_seen_at
        Fleet-->>Agent: HeartbeatResponse (ok: true)
    end

    Note over API,Agent: 4. Operator Quarantine Command
    API->>Fleet: gRPC send_isolate_command(node_id, true)
    Fleet-->>Agent: ServerCommand::Isolate (isolate: true, reason)
    Agent->>Agent: Apply kernel nftables drop rules
```

## 2. Configuration reference

Set environment variables in `.env` or systemd environment files:

```bash
# gRPC listener binding
HOST=0.0.0.0
PORT=50051

# Registry database pool
DATABASE_URL=postgres://edr:edrpassword@localhost:5433/edr_nodes
DB_POOL_MAX_SIZE=20

# Kafka event producer
KAFKA_BROKERS=localhost:9092
KAFKA_TOPIC_AGENTS_EVENTS=aigis.events.raw

# Security credentials
JWT_SECRET=super_secret_jwt_key_replace_in_production_32_bytes_min
FLEET_ENROLLMENT_SECRET=aigis_node_enrollment_pre_shared_key_change_me

# Optional TLS certificate configuration
TLS_CERT_PATH=/etc/aigis-zero/certs/server.crt
TLS_KEY_PATH=/etc/aigis-zero/certs/server.key

# Observability
RUST_LOG=info,fleet_server=debug
LOG_FORMAT=json
```

## 3. The node enrollment handshake in detail

Enrollment establishes identity and cryptographic trust between an endpoint and the fleet.

### Step 1: Pre-shared secret validation

When an agent starts for the first time, it has no persistent token. It sends a gRPC `RegisterAgent` RPC:

```protobuf
message RegisterRequest {
  string hostname = 1;
  string os_version = 2;
  string agent_version = 3;
  string machine_id = 4;
}
```

The request metadata must include the enrollment secret:
`x-enrollment-secret: <FLEET_ENROLLMENT_SECRET>`

The server checks this against `FLEET_ENROLLMENT_SECRET`. If missing or invalid, the server rejects the request immediately with gRPC status `UNAUTHENTICATED`.

### Step 2: Database registration and identity assignment

Once the secret is validated, the `EnrollmentService` checks `edr_nodes`:
- If `machine_id` already exists, the server updates the hostname, agent version, and last seen timestamp, reusing the existing `node_id`. This prevents duplicate entries when an agent restarts or re-enrolls.
- If `machine_id` is new, the server generates a new UUID v4 `node_id` and inserts records into both `nodes` and `enrollment_events`.

### Step 3: JWT token issuance

The server mints a JSON Web Token containing:
- `sub`: The assigned `node_id`
- `hostname`: The endpoint hostname
- `exp`: Expiration timestamp (24 hours from issuance)

The response delivers the token and configuration:

```protobuf
message RegisterResponse {
  string node_id = 1;
  string token = 2;
  AgentConfig config = 3;
}
```

The agent stores this token in memory and uses it to authenticate all subsequent gRPC streams and unary calls.

## 4. Telemetry streaming and command dispatch

### Bidirectional EventStream

The agent opens `rpc EventStream(stream AgentEvent) returns (stream ServerCommand)`:

1. Request header must include `Authorization: Bearer <JWT_TOKEN>`.
2. The server decodes and validates the token. If valid, the stream starts.
3. Inbound stream: Agent transmits telemetry batches:
   - `node_id`: Endpoint UUID
   - `event_type`: Event category (`process_start`, `socket_connect`, `file_modify`, `user_login`)
   - `payload`: Raw JSON telemetry payload
   - `timestamp_ns`: Nanosecond timestamp recorded by the kernel
   - `sequence_id`: Local SQLite monotonic sequence identifier
4. Telemetry fan-out: The Fleet Server writes incoming events to Kafka topic `aigis.events.raw` using `node_id` as the message key.
5. Inbound acknowledgement: When the Kafka producer confirms message persistence, the Fleet Server sends a `ServerCommand::Ack(sequence_id)` back over the response stream.
6. Local buffer release: Upon receiving the acknowledgment, the agent deletes the event from its local SQLite write-ahead log.

### Command routing and quarantine

When an operator issues a quarantine request via the API backend, the backend calls the Fleet Server:

1. Fleet Server looks up the active bidirectional stream for the specified `node_id`.
2. Fleet Server writes a `ServerCommand::Isolate` into the stream:
   ```protobuf
   message IsolateCommand {
     bool isolate = 1;
     string reason = 2;
   }
   ```
3. The agent receives this frame and instructs its kernel `isolation` module to activate `nftables` blocking rules.

## 5. Heartbeats and health tracking

Agents send `Heartbeat` requests every 60 seconds (configurable via `heartbeat_interval_secs`):

```protobuf
message HeartbeatRequest {
  string node_id = 1;
  string status = 2;             // "healthy" | "degraded" | "error"
  int64 events_buffered = 3;     // count of unacknowledged events in SQLite WAL
}
```

The Fleet Server records the heartbeat in `node_health` and updates `nodes.last_seen_at`.

### Safety invariant: Quarantine preservation

The Fleet Server strictly enforces that heartbeats update operational telemetry but never overwrite an operator-assigned containment status:
- If a node is set to `operator_status = 'isolated'`, receiving a heartbeat with `status = 'healthy'` updates liveness metrics, but the node remains in `isolated` status until an operator explicitly unisolates it.

## 6. Production deployment runbook

### Docker deployment

The service runs as `edr-fleet-server` within the Docker Compose cluster:

```bash
cd infra
docker compose up -d fleet-server
```

Check logs:
```bash
docker logs -f edr-fleet-server
```

### Pulling and running pre-built Docker image

Pull the published image from GitHub Container Registry:

```bash
docker pull ghcr.io/swar09/aigis-fleet-server:latest
```

Run standalone container with host networking for direct gRPC port binding:

```bash
docker run -d \
  --name edr-fleet-server \
  --restart unless-stopped \
  -p 50051:50051 \
  --env-file .env \
  ghcr.io/swar09/aigis-fleet-server:latest
```

### Systemd deployment

For standalone deployment on Linux host instances:

1. Install binary to `/usr/local/bin/fleet-server-bin`.
2. Create environment file `/etc/aigis-zero/fleet-server.env`.
3. Create systemd unit file `/etc/systemd/system/fleet-server.service`:

```ini
[Unit]
Description=Aigis-Zero EDR Fleet Server
After=network.target

[Service]
Type=simple
User=edr
Group=edr
EnvironmentFile=/etc/aigis-zero/fleet-server.env
ExecStart=/usr/local/bin/fleet-server-bin
Restart=always
RestartSec=5
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
```

4. Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now fleet-server
sudo systemctl status fleet-server
```

## 7. Operational troubleshooting

### Agent enrollment rejected
- Log message: `Agent enrollment rejected: invalid or missing enrollment secret`.
- Cause: The `x-enrollment-secret` header passed by the agent does not match `FLEET_ENROLLMENT_SECRET`.
- Fix: Verify secret in `/etc/aigis-zero/config.toml` matches `.env` on the fleet server.

### Stream unauthenticated error
- Log message: `unauthenticated: token expired or invalid`.
- Cause: The agent JWT has passed its 24-hour lifetime.
- Fix: The agent client automatically catches expired token errors and re-triggers enrollment. Verify that the agent can contact the server for renewal.

### Kafka write failures
- Log message: `failed to publish event to aigis.events.raw`.
- Cause: Kafka broker is partitioned, out of disk space, or topic is missing.
- Fix: Check Kafka health with `docker exec -it edr-kafka kafka-topics --bootstrap-server localhost:9092 --list`. Verify disk availability on the Kafka data volume.
