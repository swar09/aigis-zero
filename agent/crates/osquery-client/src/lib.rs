pub mod client;
pub mod diff;
pub mod types;

pub use crate::client::OsqueryClient;
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

    pub async fn start(&self, _agent_uuid: &str) -> mpsc::Receiver<OsqueryResult> {
        let (_tx, rx) = mpsc::channel(100);
        // Custom scheduler logic removed; agent will rely on osquery.conf directly.
        rx
    }

    pub async fn live_query(&self, sql: &str) -> Result<QueryResponse> {
        let mut client = OsqueryClient::connect(&self.config.socket_path).await?;
        client.live_query(sql).await
    }

    pub async fn update_schedule(&self, _queries: Vec<ScheduledQuery>) -> Result<()> {
        // Custom scheduler logic removed
        Ok(())
    }
}
