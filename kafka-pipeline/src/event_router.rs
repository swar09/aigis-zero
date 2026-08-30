use std::{sync::Arc, time::Duration};

use rdkafka::{
    message::{Header, OwnedHeaders},
    producer::{FutureProducer, FutureRecord, Producer},
    util::Timeout,
};
use serde_json::Value;
use tracing::debug;

use crate::{consumer::MessageProcessor, metrics::PipelineMetrics};

/// Routes events from `aigis.events.raw` to typed topics based on `event_type`.
pub struct EventRouterProcessor {
    producer: FutureProducer,
    metrics: Arc<PipelineMetrics>,
}

impl EventRouterProcessor {
    /// Creates a new `EventRouterProcessor` wrapping a Kafka producer and metrics counter.
    pub fn new(producer: FutureProducer, metrics: Arc<PipelineMetrics>) -> Self {
        Self { producer, metrics }
    }

    /// Flushes any buffered records in the producer.
    pub fn flush(&self, timeout: Duration) -> Result<(), String> {
        self.producer.flush(Timeout::After(timeout)).map_err(|e| e.to_string())
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
        topic: &str,
        partition: i32,
        offset: i64,
    ) -> Result<(), String> {
        self.metrics.inc_consumed();

        // Lightweight JSON peek — extract event_type or fallback to query_name
        let event: Value = match serde_json::from_slice(payload) {
            Ok(v) => v,
            Err(e) => {
                self.metrics.inc_errors();
                self.metrics.inc_routed("aigis.events.dlq");

                let headers = OwnedHeaders::new()
                    .insert(Header {
                        key: "x-source-topic",
                        value: Some(topic.as_bytes()),
                    })
                    .insert(Header {
                        key: "x-original-partition",
                        value: Some(partition.to_string().as_bytes()),
                    })
                    .insert(Header {
                        key: "x-original-offset",
                        value: Some(offset.to_string().as_bytes()),
                    })
                    .insert(Header {
                        key: "x-error-reason",
                        value: Some(b"Invalid JSON payload"),
                    });

                let record = FutureRecord::to("aigis.events.dlq")
                    .payload(payload)
                    .key(key.unwrap_or(&[]))
                    .headers(headers);

                self.producer
                    .send(record, Timeout::After(Duration::from_secs(5)))
                    .await
                    .map_err(|(err, _)| format!("Kafka send error: {err}"))?;

                return Err(format!("Invalid JSON: {e}"));
            }
        };

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
        self.metrics.inc_routed(target_topic);

        // Forward to typed topic or DLQ
        let record = if target_topic == "aigis.events.dlq" {
            let headers = OwnedHeaders::new()
                .insert(Header {
                    key: "x-source-topic",
                    value: Some(topic.as_bytes()),
                })
                .insert(Header {
                    key: "x-original-partition",
                    value: Some(partition.to_string().as_bytes()),
                })
                .insert(Header {
                    key: "x-original-offset",
                    value: Some(offset.to_string().as_bytes()),
                })
                .insert(Header {
                    key: "x-error-reason",
                    value: Some(b"Unclassified event type"),
                });

            FutureRecord::to(target_topic)
                .payload(payload)
                .key(key.unwrap_or(&[]))
                .headers(headers)
        } else {
            FutureRecord::to(target_topic).payload(payload).key(key.unwrap_or(&[]))
        };

        self.producer
            .send(record, Timeout::After(Duration::from_secs(5)))
            .await
            .map_err(|(e, _)| format!("Kafka send error: {e}"))?;

        debug!(event_type, target_topic, "Event routed");
        Ok(())
    }
}
