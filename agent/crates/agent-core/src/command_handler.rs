#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
use edr_sdk::models::envelope::{ServerMessage, ServerMessageType};
use serde_json::Value;
use tracing::{info, warn};
use isolation::IsolationManager;
use osquery_client::OsqueryCollector;
use std::sync::Arc;

pub struct CommandHandler {
    pub osquery: Arc<OsqueryCollector>,
    pub isolation: IsolationManager,
}

impl CommandHandler {
    pub async fn handle(&self, msg: ServerMessage) -> Result<Value, String> {
        let command = msg
            .payload
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or("missing command field")?;

        match command {
            "run_query" => {
                let sql = msg
                    .payload
                    .get("sql")
                    .and_then(|v| v.as_str())
                    .ok_or("missing sql field")?;
                let results = self
                    .osquery
                    .live_query(sql)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(results).unwrap_or_default())
            }
            "isolate" => {
                self.isolation.isolate().await.map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"status": "isolated"}))
            }
            "unisolate" => {
                self.isolation
                    .de_isolate()
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"status": "unisolated"}))
            }
            _ => Err(format!("unknown command: {command}")),
        }
    }
}
