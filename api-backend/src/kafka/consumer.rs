use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message,
};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::models::ws::LiveEvent;

pub async fn start_kafka_consumer(
    brokers: &str,
    group_id: &str,
    topics: &[&str],
    broadcast_tx: broadcast::Sender<LiveEvent>,
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
                        if let Some(event) = borrowed_msg.payload().and_then(|p| parse_kafka_message(topic, p)) {
                            let _ = broadcast_tx.send(event);
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

fn parse_kafka_message(topic: &str, payload: &[u8]) -> Option<LiveEvent> {
    if let Ok(direct_event) = serde_json::from_slice::<LiveEvent>(payload) {
        return Some(direct_event);
    }

    let json: serde_json::Value = serde_json::from_slice(payload).ok()?;

    match topic {
        t if t.starts_with("aigis.events") => {
            let node_id = json
                .get("node_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let hostname = json
                .get("hostname")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let event_type = json
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let timestamp_ns = json
                .get("timestamp_ns")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(|| chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

            Some(LiveEvent::Log {
                node_id,
                hostname,
                event_type,
                payload: json,
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
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let hostname = json
                .get("hostname")
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
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let agent_status = json
                .get("agent_status")
                .or_else(|| json.get("status"))
                .and_then(|v| v.as_str())
                .unwrap_or("healthy")
                .to_string();
            let events_buffered = json
                .get("events_buffered")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let timestamp_ns = json
                .get("timestamp_ns")
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
