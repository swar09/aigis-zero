use async_trait::async_trait;
use diesel_async::RunQueryDsl;
use tracing::{error, info};

use crate::{
    db::{DbPool, schema::alerts},
    error::AppError,
    kafka::AlertKafkaProducer,
    models::{Alert, NewAlertEntity},
};

#[async_trait]
pub trait AlertSink: Send + Sync {
    async fn send_alert(&self, alert: &Alert) -> Result<(), AppError>;
    async fn send_batch(&self, alerts: &[Alert]) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct DualAlertSink {
    kafka_producer: AlertKafkaProducer,
    pool: DbPool,
}

impl DualAlertSink {
    pub fn new(kafka_producer: AlertKafkaProducer, pool: DbPool) -> Self {
        Self { kafka_producer, pool }
    }
}

#[async_trait]
impl AlertSink for DualAlertSink {
    async fn send_alert(&self, alert: &Alert) -> Result<(), AppError> {
        // 1. Publish to Kafka alert stream
        self.kafka_producer.publish(alert).await?;

        // 2. Persist to PostgreSQL edr_alerts
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::DatabasePool(format!("Failed to acquire connection: {e}")))?;

        let entity = NewAlertEntity::from(alert);

        diesel::insert_into(alerts::table)
            .values(&entity)
            .on_conflict(alerts::alert_id)
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| AppError::DatabaseQuery(format!("Failed to insert alert: {e}")))?;

        Ok(())
    }

    async fn send_batch(&self, alerts_batch: &[Alert]) -> Result<(), AppError> {
        if alerts_batch.is_empty() {
            return Ok(());
        }

        // 1. Broadcast alerts to Kafka topic
        for alert in alerts_batch {
            if let Err(e) = self.kafka_producer.publish(alert).await {
                error!(error = %e, alert_id = %alert.alert_id, "Failed to publish alert to Kafka in batch");
            }
        }

        // 2. Batch insert into PostgreSQL with Diesel-Async
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::DatabasePool(format!("Failed to acquire connection: {e}")))?;

        let entities: Vec<NewAlertEntity> = alerts_batch.iter().map(NewAlertEntity::from).collect();

        diesel::insert_into(alerts::table)
            .values(&entities)
            .on_conflict(alerts::alert_id)
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| AppError::DatabaseQuery(format!("Failed to batch insert alerts: {e}")))?;

        info!(count = entities.len(), "Persisted alert batch to PostgreSQL");
        Ok(())
    }
}
