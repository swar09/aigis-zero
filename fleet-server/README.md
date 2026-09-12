# fleet-server

Control plane service managing agent enrollment, node heartbeats, telemetry ingestion, and quarantine command dispatch over gRPC.

For the comprehensive protocol specification, enrollment lifecycle, and deployment runbooks, see the [Fleet Server guide](guide.md).

## Crate layout

- `fleet-server-bin`: Binary entry point, settings loader, diesel-async pool init, and Tonic server
- `grpc-listener`: gRPC service implementing `FleetService` contracts (`RegisterAgent`, `EventStream`, `Heartbeat`)
- `node-enrollment`: Node registration logic with 24-hour HMAC-SHA256 JWT generation
- `health-tracker`: Heartbeat tracker preventing heartbeats from altering quarantine status
- `fleet-manager`: Agent state transition domain models
- `kafka-handler`: High-throughput telemetry publisher streaming raw events to Kafka (`aigis.events.raw`)
- `postgres-interface`: Database access layer for the `edr_nodes` registry using `diesel-async` and `deadpool`
- `fleet-tracing`: Structured logging initialization

## Environment configuration

Set the following variables in the root `.env` or system environment:

```bash
# Server binding
HOST=0.0.0.0
PORT=50051

# PostgreSQL registry database
DATABASE_URL=postgres://edr:edrpassword_change_in_production@localhost:5433/edr_nodes
DB_POOL_MAX_SIZE=20

# Kafka event stream
KAFKA_BROKERS=localhost:9092
KAFKA_TOPIC_AGENTS_EVENTS=aigis.events.raw

# Security
JWT_SECRET=super_secret_jwt_key_replace_in_production_32_bytes_min
FLEET_ENROLLMENT_SECRET=aigis_node_enrollment_pre_shared_key_change_me

# Logging
RUST_LOG=info
LOG_FORMAT=json
```

## Running locally

```bash
# Ensure infrastructure is running
./scripts/infra.sh up

# Start fleet server
cargo run -p fleet-server-bin
```

## gRPC service endpoints

| Method | Request | Response | Description |
|---|---|---|---|
| `RegisterAgent` | `RegisterRequest` | `RegisterResponse` | Validates enrollment secret, registers node, issues JWT |
| `EventStream` | `AgentEvent` (stream) | `ServerCommand` (stream) | Ingests telemetry into Kafka and dispatches quarantine commands |
| `Heartbeat` | `HeartbeatRequest` | `HeartbeatResponse` | Updates node liveness timestamp and agent status |

## Additional resources

- [Fleet Server operations guide](guide.md): Complete enrollment handshake, event streaming protocol, and deployment.
- [Shared Protobuf contracts](../sdk/proto/fleet.proto): Service interface definition.
- [Database migrations](migrations/): Schema definitions for `nodes`, `enrollment_events`, and `node_health`.
