use std::time::Duration;

use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use tracing::{error, info};

use crate::{error::AppError, models::Alert};

#[derive(Clone)]
pub struct AlertKafkaProducer {
    producer: FutureProducer,
    topic: String,
}

impl AlertKafkaProducer {
    pub fn new(brokers: &str, topic: &str) -> Result<Self, AppError> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("queue.buffering.max.messages", "100000")
            .set("compression.type", "lz4")
            .create()
            .map_err(|e| AppError::KafkaProduce(format!("Failed to create alerts producer: {e}")))?;

        info!(topic = %topic, "Initialized Alert Kafka producer");
        Ok(Self {
            producer,
            topic: topic.to_string(),
        })
    }

    pub async fn publish(&self, alert: &Alert) -> Result<(), AppError> {
        let payload = serde_json::to_vec(alert)?;
        let key = alert.node_id.to_string();

        let record = FutureRecord::to(&self.topic).payload(&payload).key(&key);

        match self.producer.send(record, Duration::from_secs(3)).await {
            Ok(_) => Ok(()),
            Err((e, _)) => {
                error!(error = %e, alert_id = %alert.alert_id, "Failed to publish alert to Kafka topic");
                Err(AppError::KafkaProduce(format!("Alert publish error: {e}")))
            }
        }
    }
}
