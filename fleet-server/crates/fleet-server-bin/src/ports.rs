use std::sync::Arc;

use async_trait::async_trait;
use fleet_manager::{EventIngestPort, IncomingEvent, OutgoingCommand};
use health_tracker::HealthTracker;
use kafka_handler::KafkaPublisher;
use node_enrollment::NodeEnroller;
use postgres_interface::{DbPool, PgHealthStore, PgNodeStore};
use tonic::Status;

pub struct KafkaEventIngest {
    publisher: Arc<KafkaPublisher>,
    topic: String,
}

#[async_trait]
impl EventIngestPort for KafkaEventIngest {
    async fn ingest_event(&self, event: IncomingEvent) -> Result<Option<OutgoingCommand>, Status> {
        let payload_val: serde_json::Value = serde_json::from_slice(&event.payload)
            .unwrap_or_else(|_| serde_json::json!({ "raw": String::from_utf8_lossy(&event.payload) }));

        let event_id = if event.sequence_id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            event.sequence_id.clone()
        };

        let telemetry_envelope = serde_json::json!({
            "id": event_id,
            "node_id": event.node_id,
            "hostname": "",
            "event_type": event.event_type,
            "timestamp_ns": event.timestamp_ns,
            "payload": payload_val,
            "raw_sequence_id": Some(event.sequence_id.clone()),
        });

        let payload_bytes = match serde_json::to_vec(&telemetry_envelope) {
            Ok(bytes) => bytes,
            Err(_) => event.payload.clone(),
        };

        match self
            .publisher
            .publish(&self.topic, &event.node_id, &payload_bytes)
            .await
        {
            Ok(_) => Ok(Some(OutgoingCommand::Ack {
                sequence_id: event.sequence_id,
            })),
            Err(e) => {
                tracing::error!(error = %e, "Failed to publish event to Kafka");
                Err(Status::internal("Failed to publish event to message broker"))
            }
        }
    }
}

pub fn build_ports(
    pg_pool: DbPool,
    jwt_secret: &str,
    kafka_brokers: &str,
    kafka_topic: &str,
) -> (Arc<NodeEnroller>, Arc<HealthTracker>, Arc<KafkaEventIngest>) {
    let node_store = Arc::new(PgNodeStore::new(pg_pool.clone()));
    let health_store = Arc::new(PgHealthStore::new(pg_pool));

    let enroller = Arc::new(NodeEnroller::new(node_store, jwt_secret.as_bytes()));
    let tracker = Arc::new(HealthTracker::new(health_store));

    let publisher = KafkaPublisher::new(kafka_brokers).expect("Failed to initialize KafkaPublisher");
    let event_ingest = Arc::new(KafkaEventIngest {
        publisher: Arc::new(publisher),
        topic: kafka_topic.to_string(),
    });

    (enroller, tracker, event_ingest)
}
