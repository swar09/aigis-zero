use std::time::Duration;

use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord, Producer},
    util::Timeout,
};

/// High-throughput, low-latency Kafka publisher for streaming telemetry events.
pub struct KafkaPublisher {
    producer: FutureProducer,
}

impl KafkaPublisher {
    /// Creates a new `KafkaPublisher` with tuned buffering, LZ4 compression, and micro-batching.
    pub fn new(brokers: &str) -> Result<Self, String> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("compression.type", "lz4")
            .set("linger.ms", "5")
            .set("batch.num.messages", "10000")
            .set("queue.buffering.max.messages", "100000")
            .set("queue.buffering.max.kbytes", "65536")
            .set("acks", "1")
            .create()
            .map_err(|e| e.to_string())?;

        Ok(Self { producer })
    }

    /// Publishes a record with partition key to the target Kafka topic.
    pub async fn publish(&self, topic: &str, key: &str, payload: &[u8]) -> Result<(), String> {
        let record = FutureRecord::to(topic).key(key).payload(payload);

        self.producer
            .send(record, Timeout::After(Duration::from_secs(10)))
            .await
            .map_err(|(e, _)| e.to_string())?;

        Ok(())
    }

    /// Flushes any pending messages in the producer queue.
    pub fn flush(&self, timeout: Duration) -> Result<(), String> {
        self.producer.flush(Timeout::After(timeout)).map_err(|e| e.to_string())
    }
}
