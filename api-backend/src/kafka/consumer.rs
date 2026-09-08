use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message,
};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    models::{log::NewEventLogEntity, ws::LiveEvent},
    repositories::LogRepository,
};

pub async fn start_kafka_consumer(
    brokers: &str,
    group_id: &str,
    topics: &[&str],
    broadcast_tx: broadcast::Sender<LiveEvent>,
    log_repo: Arc<dyn LogRepository>,
    shutdown: CancellationToken,
) {
    let consumer_res: Result<StreamConsumer, _> = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", group_id)
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "latest")
        .set("session.timeout.ms", "6000")
        .create();

    let consumer = match consumer_res {
        Ok(c) => c,
        Err(e) => {
            warn!(err = %e, "Kafka consumer initialization deferred / failed");
            return;
        }
    };

    if let Err(e) = consumer.subscribe(topics) {
        warn!(err = %e, "Failed to subscribe to Kafka topics");
        return;
    }

    info!(topics = ?topics, "Kafka background consumer worker active");

    // Micro-batch log persistence pipeline
    let (persist_tx, mut persist_rx) = tokio::sync::mpsc::channel::<NewEventLogEntity>(10_000);
    let worker_repo = log_repo.clone();
    let worker_shutdown = shutdown.clone();

    tokio::spawn(async move {
        let mut batch = Vec::with_capacity(50);
        let mut flush_interval = tokio::time::interval(Duration::from_millis(500));

        loop {
            tokio::select! {
                _ = worker_shutdown.cancelled() => {
                    if !batch.is_empty()
                        && let Err(e) = worker_repo.insert_batch(&batch).await {
                            warn!(err = %e, count = batch.len(), "Failed to flush event logs on shutdown");
                        }
                    break;
                }
                _ = flush_interval.tick() => {
                    if !batch.is_empty() {
                        if let Err(e) = worker_repo.insert_batch(&batch).await {
                            warn!(err = %e, count = batch.len(), "Failed to persist micro-batch of event logs");
                        }
                        batch.clear();
                    }
                }
                item = persist_rx.recv() => {
                    match item {
                        Some(log_entity) => {
                            batch.push(log_entity);
                            if batch.len() >= 50 {
                                if let Err(e) = worker_repo.insert_batch(&batch).await {
                                    warn!(err = %e, count = batch.len(), "Failed to persist batch of event logs");
                                }
                                batch.clear();
                            }
                        }
                        None => {
                            if !batch.is_empty() {
                                let _ = worker_repo.insert_batch(&batch).await;
                            }
                            break;
                        }
                    }
                }
            }
        }
    });

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                info!("Kafka consumer loop terminating on shutdown signal");
                break;
            }
            msg_result = consumer.recv() => {
                match msg_result {
                    Ok(borrowed_msg) => {
                        let topic = borrowed_msg.topic();
                        if let Some(payload) = borrowed_msg.payload() {
                            if let Some(event) = parse_kafka_message(topic, payload) {
                                let _ = broadcast_tx.send(event);
                            }
                            if let Some(log_entity) = parse_log_entity(topic, payload) {
                                let _ = persist_tx.try_send(log_entity);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(err = %e, "Kafka stream consumer message receive error");
                    }
                }
            }
        }
    }
}

fn parse_log_entity(topic: &str, payload: &[u8]) -> Option<NewEventLogEntity> {
    if !topic.starts_with("aigis.events") {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(payload).ok()?;

    let node_id_str = json
        .get("node_id")
        .or_else(|| json.get("agent_uuid"))
        .or_else(|| {
            json.get("payload")
                .and_then(|p| p.get("agent_uuid").or_else(|| p.get("node_id")))
        })
        .and_then(|v| v.as_str())?;

    let node_id = Uuid::parse_str(node_id_str).ok()?;

    let event_type = json
        .get("event_type")
        .or_else(|| json.get("query_name"))
        .or_else(|| {
            json.get("payload")
                .and_then(|p| p.get("query_name").or_else(|| p.get("event_type")))
        })
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let hostname = json
        .get("hostname")
        .or_else(|| json.get("payload").and_then(|p| p.get("hostname")))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown")
        .to_string();

    let raw_sequence_id = json
        .get("raw_sequence_id")
        .or_else(|| json.get("sequence_id"))
        .or_else(|| json.get("payload").and_then(|p| p.get("sequence_id")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let recorded_at = json
        .get("timestamp_ns")
        .or_else(|| json.get("payload").and_then(|p| p.get("timestamp_ns")))
        .and_then(|v| v.as_i64())
        .map(DateTime::from_timestamp_nanos)
        .unwrap_or_else(Utc::now);

    let payload_val = json.get("payload").cloned().unwrap_or(json.clone());

    let event_id = json
        .get("id")
        .or_else(|| json.get("event_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::new_v4);

    Some(NewEventLogEntity {
        event_id,
        node_id,
        event_type,
        hostname,
        payload: payload_val,
        raw_sequence_id,
        recorded_at,
    })
}

fn parse_kafka_message(topic: &str, payload: &[u8]) -> Option<LiveEvent> {
    if let Ok(direct_event) = serde_json::from_slice::<LiveEvent>(payload) {
        return Some(direct_event);
    }

    let json: serde_json::Value = serde_json::from_slice(payload).ok()?;

    match topic {
        t if t.starts_with("aigis.events") => {
            let node_id = json
                .get("node_id")
                .or_else(|| json.get("agent_uuid"))
                .or_else(|| {
                    json.get("payload")
                        .and_then(|p| p.get("agent_uuid").or_else(|| p.get("node_id")))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let hostname = json
                .get("hostname")
                .or_else(|| json.get("payload").and_then(|p| p.get("hostname")))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("unknown")
                .to_string();
            let event_type = json
                .get("event_type")
                .or_else(|| json.get("query_name"))
                .or_else(|| {
                    json.get("payload")
                        .and_then(|p| p.get("query_name").or_else(|| p.get("event_type")))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let timestamp_ns = json
                .get("timestamp_ns")
                .or_else(|| json.get("payload").and_then(|p| p.get("timestamp_ns")))
                .and_then(|v| v.as_i64())
                .unwrap_or_else(|| chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

            let payload_val = json.get("payload").cloned().unwrap_or(json);

            Some(LiveEvent::Log {
                node_id,
                hostname,
                event_type,
                payload: payload_val,
                timestamp_ns,
            })
        }
        "aigis.alerts" => {
            let id = json
                .get("alert_id")
                .or_else(|| json.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let node_id = json
                .get("node_id")
                .or_else(|| json.get("agent_uuid"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let hostname = json
                .get("hostname")
                .or_else(|| json.get("payload").and_then(|p| p.get("hostname")))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let severity = json
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("medium")
                .to_string();
            let mitre_technique_id = json
                .get("mitre_technique_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let description = json
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let threat_score = json
                .get("threat_score")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32)
                .unwrap_or(5.0);
            let timestamp_ns = json
                .get("timestamp_ns")
                .or_else(|| json.get("payload").and_then(|p| p.get("timestamp_ns")))
                .and_then(|v| v.as_i64())
                .unwrap_or_else(|| chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

            Some(LiveEvent::Alert {
                id,
                node_id,
                hostname,
                severity,
                mitre_technique_id,
                description,
                threat_score,
                timestamp_ns,
            })
        }
        "aigis.health" => {
            let node_id = json
                .get("node_id")
                .or_else(|| json.get("agent_uuid"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let agent_status = json
                .get("agent_status")
                .or_else(|| json.get("status"))
                .and_then(|v| v.as_str())
                .unwrap_or("healthy")
                .to_string();
            let events_buffered = json.get("events_buffered").and_then(|v| v.as_i64()).unwrap_or(0);
            let timestamp_ns = json
                .get("timestamp_ns")
                .or_else(|| json.get("payload").and_then(|p| p.get("timestamp_ns")))
                .and_then(|v| v.as_i64())
                .unwrap_or_else(|| chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

            Some(LiveEvent::Heartbeat {
                node_id,
                agent_status,
                events_buffered,
                timestamp_ns,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_entity_from_raw_agent_event() {
        let payload = serde_json::json!({
            "action": "ADDED",
            "agent_uuid": "b6b8bed7-decf-4a56-80de-1600b762f750",
            "query_name": "running_processes",
            "rows": [
                {
                    "columns": [
                        {"name": "pid", "value": "1234"},
                        {"name": "name", "value": "curl"}
                    ]
                }
            ],
            "timestamp_ns": 1788880789164568263_i64
        });
        let bytes = serde_json::to_vec(&payload).unwrap();

        let entity = parse_log_entity("aigis.events.raw", &bytes).expect("should parse log entity");
        assert_eq!(entity.node_id.to_string(), "b6b8bed7-decf-4a56-80de-1600b762f750");
        assert_eq!(entity.event_type, "running_processes");
        assert_eq!(entity.hostname, "unknown");
        assert!(entity.payload.get("rows").is_some());
    }

    #[test]
    fn test_parse_log_entity_from_envelope() {
        let payload = serde_json::json!({
            "id": "c1b2c3d4-0001-0000-0000-000000000001",
            "node_id": "b6b8bed7-decf-4a56-80de-1600b762f750",
            "hostname": "hawkins",
            "event_type": "process_events",
            "timestamp_ns": 1788880789164568263_i64,
            "raw_sequence_id": "seq-999",
            "payload": {
                "cmdline": "ls -la"
            }
        });
        let bytes = serde_json::to_vec(&payload).unwrap();

        let entity = parse_log_entity("aigis.events.process", &bytes).expect("should parse log entity");
        assert_eq!(entity.event_id.to_string(), "c1b2c3d4-0001-0000-0000-000000000001");
        assert_eq!(entity.node_id.to_string(), "b6b8bed7-decf-4a56-80de-1600b762f750");
        assert_eq!(entity.hostname, "hawkins");
        assert_eq!(entity.event_type, "process_events");
        assert_eq!(entity.raw_sequence_id.as_deref(), Some("seq-999"));
    }

    #[test]
    fn test_parse_log_entity_ignores_non_event_topics() {
        let payload = serde_json::json!({"foo": "bar"});
        let bytes = serde_json::to_vec(&payload).unwrap();
        assert!(parse_log_entity("aigis.alerts", &bytes).is_none());
        assert!(parse_log_entity("aigis.health", &bytes).is_none());
    }

    #[test]
    fn test_parse_kafka_message_extracts_agent_uuid() {
        let payload = serde_json::json!({
            "action": "ADDED",
            "agent_uuid": "b6b8bed7-decf-4a56-80de-1600b762f750",
            "query_name": "running_processes",
            "rows": []
        });
        let bytes = serde_json::to_vec(&payload).unwrap();

        let event = parse_kafka_message("aigis.events.raw", &bytes).expect("should parse live event");
        match event {
            LiveEvent::Log {
                node_id, event_type, ..
            } => {
                assert_eq!(node_id, "b6b8bed7-decf-4a56-80de-1600b762f750");
                assert_eq!(event_type, "running_processes");
            }
            _ => panic!("expected LiveEvent::Log"),
        }
    }
}
