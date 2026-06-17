#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
use async_trait::async_trait;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

/// Trait for implementing Kafka message processors
#[async_trait]
pub trait MessageProcessor: Send + Sync + 'static {
    /// Process a single message. Return Ok(()) to commit, Err to skip.
    async fn process(
        &self,
        key: Option<&[u8]>,
        payload: &[u8],
        topic: &str,
        partition: i32,
        offset: i64,
    ) -> Result<(), String>;
}

/// A consumer worker that reads from a topic and calls a processor
pub struct ConsumerWorker {
    consumer: StreamConsumer,
    processor: Box<dyn MessageProcessor>,
    shutdown: CancellationToken,
}

impl ConsumerWorker {
    /// Creates and configures a Kafka consumer worker.
    ///
    /// Initializes a consumer connected to the specified brokers, subscribes to the provided topics,
    /// and prepares the worker to process messages via the given processor.
    ///
    /// # Arguments
    ///
    /// * `brokers` - Comma-separated Kafka broker addresses
    /// * `group_id` - Consumer group identifier for offset management
    /// * `topics` - Topics to subscribe to for message consumption
    /// * `processor` - Handler for processing received messages
    /// * `shutdown` - Cancellation token to trigger graceful shutdown
    ///
    /// # Errors
    ///
    /// Returns an error string if consumer creation or topic subscription fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let processor = Box::new(MyProcessor);
    /// let shutdown = CancellationToken::new();
    /// let worker = ConsumerWorker::new(
    ///     "localhost:9092",
    ///     "my-group",
    ///     &["topic1"],
    ///     processor,
    ///     shutdown,
    /// )?;
    /// ```
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

        consumer
            .subscribe(topics)
            .map_err(|e| format!("Topic subscription error: {e}"))?;

        Ok(Self {
            consumer,
            processor,
            shutdown,
        })
    }

    /// Continuously processes Kafka messages until shutdown.
    ///
    /// Enters an infinite loop that awaits either a shutdown signal or the next message from the
    /// Kafka stream. For each message, it extracts the key, payload, and metadata (topic, partition,
    /// offset), and delegates processing to the configured `MessageProcessor`. If processing fails,
    /// the error is logged and the loop continues.
    ///
    /// # Examples
    ///
    /// ```
    /// let shutdown = CancellationToken::new();
    /// let processor = Box::new(MyProcessor);
    /// let worker = ConsumerWorker::new("localhost:9092", "my-group", &["my-topic"], processor, shutdown.clone()).unwrap();
    ///
    /// tokio::spawn(worker.run());
    /// // ... handle messages in background ...
    /// shutdown.cancel();
    /// ```
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
