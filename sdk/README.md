# edr-sdk

Shared domain models, Protocol Buffer contracts, code generation pipelines, and serialization codecs for the Aigis-Zero EDR platform.

For schema maintenance, extension runbooks, and code generation details, see the [SDK guide](guide.md).

## Subsystem overview

The `edr-sdk` crate maintains canonical contracts across the workspace:
- Strictly domain definitions and data models with zero business logic.
- Compiled Protocol Buffer interfaces for gRPC transports (`fleet-server`, `agent`, `api-backend`).
- Event serialization codecs for Apache Kafka telemetry routing.

## Protocol Buffer definitions

Schemas are located in `proto/`:
- `proto/fleet.proto`: Control plane interface (`FleetService`), agent enrollment handshake, bidirectional telemetry stream, and quarantine commands.
- `proto/events.proto`: Typed security event schemas (process, network, file, auth).
- `proto/agent.proto`: Host-level agent command and status models.

## Usage in workspace crates

Add `edr-sdk` to `Cargo.toml`:

```toml
[dependencies]
edr-sdk = { path = "../sdk" }
```

Import generated types:

```rust
use edr_sdk::proto::fleet::{
    AgentEvent, HeartbeatRequest, RegisterRequest, ServerCommand,
    fleet_service_client::FleetServiceClient,
};
```

## Building and testing

```bash
# Compile protobuf schemas and run unit tests
cargo test -p edr-sdk
```

## Additional resources

- [SDK integration guide](guide.md): In-depth guide covering schema evolution, gRPC contracts, and domain models.
- [Protocol Buffer source files](proto/): Canonical `.proto` definitions.
