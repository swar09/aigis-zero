use std::time::Duration;

use rdkafka::{
    config::ClientConfig,
    message::{Header, OwnedHeaders},
    producer::{FutureProducer, FutureRecord},
};
use tracing::{error, info};

use crate::error::AppError;

#[derive(Clone)]
pub struct DlqProducer {
    producer: FutureProducer,
    topic: String,
}

impl DlqProducer {
    pub fn new(brokers: &str, topic: &str) -> Result<Self, AppError> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("queue.buffering.max.messages", "50000")
            .create()
            .map_err(|e| AppError::KafkaProduce(format!("Failed to create DLQ producer: {e}")))?;

        info!(topic = %topic, "Initialized Dead Letter Queue producer");
        Ok(Self {
            producer,
            topic: topic.to_string(),
        })
    }

    pub async fn send_poison_pill(
        &self,
        original_topic: &str,
        original_partition: i32,
        original_offset: i64,
        raw_payload: &[u8],
        error_message: &str,
    ) -> Result<(), AppError> {
        let headers = OwnedHeaders::new()
            .insert(Header {
                key: "x-original-topic",
                value: Some(original_topic.as_bytes()),
            })
            .insert(Header {
                key: "x-original-partition",
                value: Some(original_partition.to_string().as_bytes()),
            })
            .insert(Header {
                key: "x-original-offset",
                value: Some(original_offset.to_string().as_bytes()),
            })
            .insert(Header {
                key: "x-error-message",
                value: Some(error_message.as_bytes()),
            });

        let record = FutureRecord::to(&self.topic)
            .payload(raw_payload)
            .key(original_topic)
            .headers(headers);

        match self.producer.send(record, Duration::from_secs(3)).await {
            Ok(_) => Ok(()),
            Err((e, _)) => {
                error!(error = %e, topic = %self.topic, "Failed to deliver payload to Dead Letter Queue");
                Err(AppError::KafkaProduce(format!("DLQ delivery error: {e}")))
            }
        }
    }
}
