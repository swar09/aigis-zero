use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde_json::Value;
use tracing::{debug, warn};
use std::time::Duration;

use crate::consumer::MessageProcessor;

/// Routes events from aigis.events.raw to typed topics based on event_type
pub struct EventRouterProcessor {
    producer: FutureProducer,
}

impl EventRouterProcessor {
    pub fn new(producer: FutureProducer) -> Self {
        Self { producer }
    }

    fn route_topic(&self, event_type: &str) -> &str {
        match event_type {
            "process_start" | "process_end" => "aigis.events.process",
            "network_connect" | "network_listen" => "aigis.events.network",
            "file_create" | "file_modify" | "file_delete" => "aigis.events.file",
            "user_login" | "user_logout" => "aigis.events.auth",
            "osquery_result" | "osquery_snapshot" => "aigis.events.process", // default bucket
            _ => "aigis.events.raw", // unknown types stay in raw
        }
    }
}

#[async_trait::async_trait]
impl MessageProcessor for EventRouterProcessor {
    async fn process(&self, key: Option<&[u8]>, payload: &[u8], _topic: &str, _partition: i32, _offset: i64) -> Result<(), String> {
        // Lightweight JSON peek — only extract event_type field
        let event: Value = serde_json::from_slice(payload)
            .map_err(|e| format!("Invalid JSON: {e}"))?;

        let event_type = event.get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let target_topic = self.route_topic(event_type);

        // Forward to typed topic
        let record = FutureRecord::to(target_topic)
            .payload(payload)  // Raw bytes, no re-serialization
            .key(key.unwrap_or(&[]));

        self.producer
            .send(record, Timeout::After(Duration::from_secs(5)))
            .await
            .map_err(|(e, _)| format!("Kafka send error: {e}"))?;

        debug!(event_type, target_topic, "Event routed");
        Ok(())
    }
}
