use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;

use fleet_manager::{EventIngestPort, IncomingEvent, OutgoingCommand};
use health_tracker::HealthTracker;
use node_enrollment::NodeEnroller;
use postgres_interface::{PgHealthStore, PgNodeStore};
use kafka_handler::KafkaPublisher;

pub struct KafkaEventIngest {
    publisher: Arc<KafkaPublisher>,
    topic: String,
}

#[async_trait]
impl EventIngestPort for KafkaEventIngest {
    async fn ingest_event(&self, event: IncomingEvent) -> Result<Option<OutgoingCommand>, Status> {
        let payload = if event.payload.is_empty() {
            b"{}"
        } else {
            event.payload.as_slice()
        };

        match self.publisher.publish(&self.topic, &event.node_id, payload).await {
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
    pg_pool: sqlx::PgPool,
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
