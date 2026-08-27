use std::{env, path::PathBuf};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct RuleEngineConfig {
    pub kafka_brokers: String,
    pub kafka_group_id: String,
    pub kafka_topics: Vec<String>,
    pub rules_directory: PathBuf,
    pub mitre_taxonomy_path: PathBuf,
    pub database_url: String,
    pub db_pool_max_size: usize,
    pub scanner_worker_count: usize,
    pub channel_capacity: usize,
    pub dedup_capacity: usize,
    pub dedup_suppression_window_secs: u64,
    pub batch_max_size: usize,
    pub batch_flush_interval_ms: u64,
    pub health_port: u16,
    pub dlq_topic: String,
    pub alerts_topic: String,
}

impl RuleEngineConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let database_url = env::var("DATABASE_URL").map_err(|_| AppError::Config("DATABASE_URL must be set".into()))?;

        let kafka_brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".into());
        let kafka_group_id = env::var("KAFKA_GROUP_ID").unwrap_or_else(|_| "aigis-rule-engine".into());
        let kafka_topics = env::var("KAFKA_TOPICS")
            .unwrap_or_else(|_| "aigis.events.process,aigis.events.network,aigis.events.file,aigis.events.auth".into())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let rules_directory = PathBuf::from(env::var("RULES_DIR").unwrap_or_else(|_| "./rules".into()));
        let mitre_taxonomy_path = PathBuf::from(
            env::var("MITRE_TAXONOMY_PATH").unwrap_or_else(|_| "./rules/mitre/enterprise-attack-linux.json".into()),
        );

        let db_pool_max_size = env::var("DB_POOL_MAX_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20);

        let scanner_worker_count = env::var("SCANNER_WORKERS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(num_cpus::get);

        let channel_capacity = env::var("CHANNEL_CAPACITY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10_000);

        let dedup_capacity = env::var("DEDUP_CAPACITY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100_000);

        let dedup_suppression_window_secs = env::var("DEDUP_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        let batch_max_size = env::var("BATCH_MAX_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500);

        let batch_flush_interval_ms = env::var("BATCH_FLUSH_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        let health_port = env::var("HEALTH_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8081);

        let dlq_topic = env::var("DLQ_TOPIC").unwrap_or_else(|_| "aigis.events.dlq".into());
        let alerts_topic = env::var("ALERTS_TOPIC").unwrap_or_else(|_| "aigis.alerts".into());

        Ok(Self {
            kafka_brokers,
            kafka_group_id,
            kafka_topics,
            rules_directory,
            mitre_taxonomy_path,
            database_url,
            db_pool_max_size,
            scanner_worker_count,
            channel_capacity,
            dedup_capacity,
            dedup_suppression_window_secs,
            batch_max_size,
            batch_flush_interval_ms,
            health_port,
            dlq_topic,
            alerts_topic,
        })
    }
}
