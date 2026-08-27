use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("rule compilation failed: {source_file}:{line}: {message}")]
    RuleCompilation {
        source_file: String,
        line: u32,
        message: String,
    },

    #[error("scan failed for event {event_id}: {message}")]
    ScanFailure { event_id: String, message: String },

    #[error("MITRE taxonomy load failed: {0}")]
    MitreTaxonomyLoad(String),

    #[error("event deserialization failed: {0}")]
    EventDeserialization(String),

    #[error("database pool error: {0}")]
    DatabasePool(String),

    #[error("database query error: {0}")]
    DatabaseQuery(String),

    #[error("kafka produce error: {0}")]
    KafkaProduce(String),

    #[error("kafka consume error: {0}")]
    KafkaConsume(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
