# fleet-server

Control plane service managing agent enrollment, node heartbeats, telemetry ingestion, and quarantine command dispatch over gRPC.

## crate layout

- `fleet-server-bin`: binary entry point, settings loader, diesel-async pool init, and Tonic server
- `grpc-listener`: gRPC service implementing `FleetService` contracts (`RegisterAgent`, `EventStream`, `Heartbeat`)
- `node-enrollment`: node registration logic with 24-hour HMAC-SHA256 JWT generation
- `health-tracker`: heartbeat tracker preventing heartbeats from altering quarantine status
- `fleet-manager`: agent state transition domain models
- `kafka-handler`: high-throughput telemetry publisher streaming raw events to Kafka (`aigis.events.raw`)
- `postgres-interface`: database access layer for the `edr_nodes` registry using `diesel-async` and `deadpool`
- `fleet-tracing`: structured logging initialization

## environment configuration

Set the following variables in the root `.env` or system environment:

```bash
# Server binding
HOST=0.0.0.0
PORT=50051

# PostgreSQL registry database
DATABASE_URL=postgres://edr:edr_dev_password@localhost:5433/edr_nodes
DB_POOL_MAX_SIZE=20

# Kafka event stream
KAFKA_BROKERS=localhost:29092
KAFKA_TOPIC_AGENTS_EVENTS=aigis.events.raw

# Security
JWT_SECRET=super_secret_jwt_key_replace_in_production_32_bytes_min
FLEET_ENROLLMENT_SECRET=fleet_enrollment_pre_shared_secret_2026

# Logging
RUST_LOG=info
LOG_FORMAT=json
```

## running locally

```bash
# Ensure infrastructure is running
./scripts/infra.sh up

# Start fleet server
cargo run -p fleet-server-bin
```

## grpc service endpoints

| Method | Request | Response | Description |
|---|---|---|---|
| `RegisterAgent` | `RegisterRequest` | `RegisterResponse` | Validates enrollment secret, registers node, issues JWT |
| `EventStream` | `AgentEvent` (stream) | `ServerCommand` (stream) | Ingests telemetry into Kafka and dispatches quarantine commands |
| `Heartbeat` | `HeartbeatRequest` | `HeartbeatResponse` | Updates node liveness timestamp and agent status |
