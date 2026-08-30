use std::sync::Arc;

use futures_util::StreamExt;
use rdkafka::{
    config::ClientConfig,
    consumer::{CommitMode, Consumer, StreamConsumer},
    message::Message,
};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{error::AppError, kafka::dlq::DlqProducer, models::TelemetryEvent};

pub struct TelemetryConsumer {
    consumer: StreamConsumer,
    dlq: DlqProducer,
    channel_tx: Sender<TelemetryEvent>,
    shutdown: CancellationToken,
}

impl TelemetryConsumer {
    pub fn new(
        brokers: &str,
        group_id: &str,
        topics: &[String],
        channel_tx: Sender<TelemetryEvent>,
        dlq: DlqProducer,
        shutdown: CancellationToken,
    ) -> Result<Self, AppError> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("partition.assignment.strategy", "cooperative-sticky")
            .set("auto.offset.reset", "earliest")
            .set("enable.auto.commit", "false")
            .set("enable.auto.offset.store", "false")
            .set("fetch.min.bytes", "1")
            .set("fetch.wait.max.ms", "100")
            .set("session.timeout.ms", "45000")
            .set("max.poll.interval.ms", "300000")
            .create()
            .map_err(|e| AppError::KafkaConsume(format!("Failed to create Kafka consumer: {e}")))?;

        let topic_slices: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();
        consumer
            .subscribe(&topic_slices)
            .map_err(|e| AppError::KafkaConsume(format!("Failed to subscribe to topics: {e}")))?;

        info!(group_id = %group_id, topics = ?topics, "Initialized TelemetryConsumer with cooperative-sticky assignor");

        Ok(Self {
            consumer,
            dlq,
            channel_tx,
            shutdown,
        })
    }

    pub async fn run(self: Arc<Self>) {
        info!("Starting Kafka telemetry consumer stream loop");
        let stream = self.consumer.stream();
        tokio::pin!(stream);

        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => {
                    info!("Consumer shutdown signal received; halting poll loop");
                    break;
                }
                msg = stream.next() => {
                    match msg {
                        Some(Ok(borrowed_msg)) => {
                            let topic = borrowed_msg.topic();
                            let partition = borrowed_msg.partition();
                            let offset = borrowed_msg.offset();
                            let payload = borrowed_msg.payload().unwrap_or(&[]);

                            if payload.is_empty() {
                                continue;
                            }

                            match serde_json::from_slice::<TelemetryEvent>(payload) {
                                Ok(event) => {
                                    if let Err(e) = self.channel_tx.send(event).await {
                                        error!(error = %e, "Internal scanner channel closed; stopping consumer");
                                        break;
                                    }
                                }
                                Err(err) => {
                                    warn!(
                                        error = %err,
                                        topic,
                                        partition,
                                        offset,
                                        "Malformed event payload; routing to Dead Letter Queue"
                                    );
                                    let err_msg = err.to_string();
                                    if let Err(dlq_err) = self.dlq.send_poison_pill(topic, partition, offset, payload, &err_msg).await {
                                        error!(error = %dlq_err, "Failed to route poisoned record to DLQ");
                                    }
                                }
                            }

                            // Commit offset to Kafka broker
                            if let Err(e) = self.consumer.commit_message(&borrowed_msg, CommitMode::Async) {
                                warn!(error = %e, topic, partition, offset, "Failed to commit message offset");
                            }
                        }
                        Some(Err(e)) => {
                            error!(error = %e, "Kafka consumer stream error");
                        }
                        None => {
                            info!("Kafka stream returned None; consumer terminating");
                            break;
                        }
                    }
                }
            }
        }
    }
}
