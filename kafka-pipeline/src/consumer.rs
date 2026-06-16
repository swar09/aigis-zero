use async_trait::async_trait;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::config::ClientConfig;
use tokio::sync::CancellationToken;
use tracing::{info, warn, error, debug};

/// Trait for implementing Kafka message processors
#[async_trait]
pub trait MessageProcessor: Send + Sync + 'static {
    /// Process a single message. Return Ok(()) to commit, Err to skip.
    async fn process(&self, key: Option<&[u8]>, payload: &[u8], topic: &str, partition: i32, offset: i64) -> Result<(), String>;
}

/// A consumer worker that reads from a topic and calls a processor
pub struct ConsumerWorker {
    consumer: StreamConsumer,
    processor: Box<dyn MessageProcessor>,
    shutdown: CancellationToken,
}

impl ConsumerWorker {
    pub fn new(
        brokers: &str,
        group_id: &str,
        topics: &[&str],
        processor: Box<dyn MessageProcessor>,
        shutdown: CancellationToken,
    ) -> Result<Self, String> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("auto.offset.reset", "earliest")
            .set("enable.auto.commit", "true")
            .set("auto.commit.interval.ms", "1000")
            .set("fetch.min.bytes", "1")
            .set("fetch.max.wait.ms", "100")
            .set("max.poll.interval.ms", "300000")
            .set("session.timeout.ms", "45000")
            .create()
            .map_err(|e| format!("Consumer creation error: {e}"))?;

        consumer.subscribe(topics)
            .map_err(|e| format!("Topic subscription error: {e}"))?;

        Ok(Self { consumer, processor, shutdown })
    }

    pub async fn run(&self) {
        use tokio_stream::StreamExt;

        info!("Consumer worker started");

        let stream = self.consumer.stream();
        tokio::pin!(stream);

        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => {
                    info!("Consumer worker shutting down");
                    break;
                }
                msg = stream.next() => {
                    match msg {
                        Some(Ok(borrowed_msg)) => {
                            let topic = borrowed_msg.topic();
                            let partition = borrowed_msg.partition();
                            let offset = borrowed_msg.offset();
                            let key = borrowed_msg.key();
                            let payload = borrowed_msg.payload().unwrap_or(&[]);

                            if let Err(e) = self.processor.process(key, payload, topic, partition, offset).await {
                                error!(error = %e, topic, partition, offset, "Message processing failed");
                                // TODO: Send to DLQ
                            }
                        }
                        Some(Err(e)) => {
                            error!(error = %e, "Kafka consumer error");
                        }
                        None => break,
                    }
                }
            }
        }
    }
}
