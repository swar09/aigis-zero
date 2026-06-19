#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, warn};

use crate::consumer::MessageProcessor;

/// Routes events from aigis.events.raw to typed topics based on event_type
pub struct EventRouterProcessor {
    producer: FutureProducer,
}

impl EventRouterProcessor {
    /// Constructs a new EventRouterProcessor with the provided Kafka producer.
    ///
    /// # Examples
    ///
    /// ```
    /// let producer = FutureProducer::new(...);
    /// let router = EventRouterProcessor::new(producer);
    /// ```
    pub fn new(producer: FutureProducer) -> Self {
        Self { producer }
    }

    /// Maps an event type to its target Kafka topic.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let topic = processor.route_topic("process_start");
    /// assert_eq!(topic, "aigis.events.process");
    /// let topic = processor.route_topic("file_create");
    /// assert_eq!(topic, "aigis.events.file");
    /// let topic = processor.route_topic("unknown_event");
    /// assert_eq!(topic, "aigis.events.raw");
    /// ```
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
    /// Routes events to typed Kafka topics based on their event type.
    ///
    /// Extracts the `event_type` field from the JSON payload, maps it to a target topic
    /// using `route_topic`, and forwards the original payload bytes to Kafka. If the
    /// `event_type` field is missing or not a string, defaults to `"unknown"`.
    ///
    /// # Errors
    ///
    /// Returns an error message if the payload is not valid JSON or the Kafka send operation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example(processor: &EventRouterProcessor) {
    /// let payload = br#"{"event_type": "process_start", "pid": 1234}"#;
    /// assert!(processor.process(None, payload, "input", 0, 0).await.is_ok());
    /// # }
    /// ```
    async fn process(
        &self,
        key: Option<&[u8]>,
        payload: &[u8],
        _topic: &str,
        _partition: i32,
        _offset: i64,
    ) -> Result<(), String> {
        // Lightweight JSON peek — only extract event_type field
        let event: Value =
            serde_json::from_slice(payload).map_err(|e| format!("Invalid JSON: {e}"))?;

        let event_type = event
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let target_topic = self.route_topic(event_type);

        // Forward to typed topic
        let record = FutureRecord::to(target_topic)
            .payload(payload) // Raw bytes, no re-serialization
            .key(key.unwrap_or(&[]));

        self.producer
            .send(record, Timeout::After(Duration::from_secs(5)))
            .await
            .map_err(|(e, _)| format!("Kafka send error: {e}"))?;

        debug!(event_type, target_topic, "Event routed");
        Ok(())
    }
}
