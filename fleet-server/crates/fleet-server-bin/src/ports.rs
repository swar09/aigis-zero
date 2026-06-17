use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;

use fleet_manager::{EventIngestPort, IncomingEvent, OutgoingCommand};
use health_tracker::HealthTracker;
use node_enrollment::NodeEnroller;
use postgres_interface::{PgHealthStore, PgNodeStore};

/// Stub event ingest — holds the place of `kafka-handler` until that crate is
/// implemented. Acks every event to unblock agent buffer clearing.
///
/// WARNING: event payloads are discarded. This is intentional while Kafka is
/// out of scope. See the implementation plan for the full data flow once
/// `kafka-handler` is wired.
// TODO: Replace StubEventIngest with a real Kafka-integrated EventIngestPort.
// 1. The implementation should load `kafka_brokers` and `kafka_topic_agents_events` from Settings.
// 2. Instantiate a FutureProducer from rdkafka via the `kafka-handler` crate.
// 3. Publish the serialized incoming event payload to Kafka on the specified topic, using the node_id or sequence_id as key.
// 4. Return OutgoingCommand::Ack only after the event has been successfully written to Kafka.
pub struct StubEventIngest;

#[async_trait]
impl EventIngestPort for StubEventIngest {
    async fn ingest_event(&self, event: IncomingEvent) -> Result<Option<OutgoingCommand>, Status> {
        tracing::debug!(
            node_id     = %event.node_id,
            event_type  = %event.event_type,
            sequence_id = %event.sequence_id,
            payload_len = event.payload.len(),
            "stub: event received (kafka-handler not yet implemented — payload discarded)"
        );
        // Ack so the agent can advance its sequence and clear its local buffer.
        Ok(Some(OutgoingCommand::Ack {
            sequence_id: event.sequence_id,
        }))
    }
}

/// Builds the real port implementations backed by PostgreSQL.
///
/// Call once at startup after the DB pool is ready. The returned `Arc`s are
/// injected into `FleetServiceImpl`.
pub fn build_ports(
    pg_pool: sqlx::PgPool,
    jwt_secret: &str,
) -> (Arc<NodeEnroller>, Arc<HealthTracker>, Arc<StubEventIngest>) {
    let node_store = Arc::new(PgNodeStore::new(pg_pool.clone()));
    let health_store = Arc::new(PgHealthStore::new(pg_pool));

    let enroller = Arc::new(NodeEnroller::new(node_store, jwt_secret.as_bytes()));
    let tracker = Arc::new(HealthTracker::new(health_store));
    let event_ingest = Arc::new(StubEventIngest);

    (enroller, tracker, event_ingest)
}
