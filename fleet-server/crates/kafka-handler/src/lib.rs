use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;

pub struct KafkaPublisher {
    producer: FutureProducer,
}

impl KafkaPublisher {
    /// Initializes a new Kafka publisher connected to the specified brokers.
    ///
    /// # Examples
    ///
    /// ```
    /// let publisher = KafkaPublisher::new("localhost:9092")?;
    /// # Ok::<(), String>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the Kafka producer cannot be created or configured.
    pub fn new(brokers: &str) -> Result<Self, String> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create()
            .map_err(|e| e.to_string())?;

        Ok(Self { producer })
    }

    /// Publishes a message to Kafka.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), String> {
    /// let publisher = KafkaPublisher::new("localhost:9092")?;
    /// publisher.publish("my-topic", "key1", b"hello").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn publish(&self, topic: &str, key: &str, payload: &[u8]) -> Result<(), String> {
        let record = FutureRecord::to(topic).key(key).payload(payload);

        self.producer
            .send(record, Timeout::Never)
            .await
            .map_err(|(e, _)| e.to_string())?;

        Ok(())
    }
}
