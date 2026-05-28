pub mod client;
pub mod diff;
pub mod scheduler;
pub mod types;

use crate::client::OsqueryClient;
use crate::scheduler::QueryScheduler;
use crate::types::{OsqueryResult, QueryResponse, ScheduledQuery};
use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub struct OsqueryConfig {
    pub socket_path: PathBuf,
    pub db_path: PathBuf,
}

pub struct OsqueryCollector {
    config: OsqueryConfig,
}

impl OsqueryCollector {
    pub async fn new(config: OsqueryConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub async fn start(&self, agent_uuid: &str) -> mpsc::Receiver<OsqueryResult> {
        let (tx, rx) = mpsc::channel(100);

        let scheduler_db_path = self.config.db_path.clone();
        let socket_path = self.config.socket_path.clone();
        let agent_uuid = agent_uuid.to_string();

        tokio::spawn(async move {
            match QueryScheduler::new(&scheduler_db_path) {
                Ok(scheduler) => scheduler.run(tx, socket_path, agent_uuid).await,
                Err(e) => tracing::error!(
                    "Failed to open scheduler SQLite DB at {:?}: {}",
                    scheduler_db_path,
                    e
                ),
            }
        });

        rx
    }

    pub async fn live_query(&self, sql: &str) -> Result<QueryResponse> {
        let mut client = OsqueryClient::connect(&self.config.socket_path).await?;
        client.live_query(sql).await
    }

    pub async fn update_schedule(&self, queries: Vec<ScheduledQuery>) -> Result<()> {
        let mut scheduler = QueryScheduler::new(&self.config.db_path)?;
        scheduler.upsert_queries(&queries)?;
        Ok(())
    }
}
