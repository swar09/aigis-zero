use std::time::Duration;

use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
};
use serde_json::Value;
use tracing::debug;

use crate::consumer::MessageProcessor;

/// Routes events from aigis.events.raw to typed topics based on event_type
pub struct EventRouterProcessor {
    producer: FutureProducer,
}

impl EventRouterProcessor {
    pub fn new(producer: FutureProducer) -> Self {
        Self { producer }
    }

    fn route_topic(&self, event_type: &str) -> Option<&'static str> {
        match event_type {
            "process_start" | "process_end" | "process" | "process_events" | "bpf_process_events" | "processes"
            | "osquery_result" | "osquery_snapshot" => Some("aigis.events.process"),
            "network_connect" | "network_listen" | "socket_events" | "bpf_socket_events" | "network" => {
                Some("aigis.events.network")
            }
            "file_create" | "file_modify" | "file_delete" | "file_events" | "file" => Some("aigis.events.file"),
            "user_login" | "user_logout" | "logged_in_users" | "auth" => Some("aigis.events.auth"),
            _ => None,
        }
    }
}

#[async_trait::async_trait]
impl MessageProcessor for EventRouterProcessor {
    async fn process(
        &self,
        key: Option<&[u8]>,
        payload: &[u8],
        _topic: &str,
        _partition: i32,
        _offset: i64,
    ) -> Result<(), String> {
        // Lightweight JSON peek — extract event_type or fallback to query_name
        let event: Value = serde_json::from_slice(payload).map_err(|e| format!("Invalid JSON: {e}"))?;

        let event_type = event
            .get("event_type")
            .and_then(|v| v.as_str())
            .or_else(|| {
                event
                    .get("payload")
                    .and_then(|p| p.get("query_name"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("unknown");

        let target_topic = self.route_topic(event_type).unwrap_or("aigis.events.dlq");

        // Forward to typed topic or DLQ (never back to raw to prevent unbounded reprocessing loops)
        let record = FutureRecord::to(target_topic).payload(payload).key(key.unwrap_or(&[]));

        self.producer
            .send(record, Timeout::After(Duration::from_secs(5)))
            .await
            .map_err(|(e, _)| format!("Kafka send error: {e}"))?;

        debug!(event_type, target_topic, "Event routed");
        Ok(())
    }
}
