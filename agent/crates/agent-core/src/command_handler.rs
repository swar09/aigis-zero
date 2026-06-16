use edr_sdk::models::envelope::{ServerMessage, ServerMessageType};
use serde_json::Value;
use tracing::{info, warn};
// Assume these exist per the plan snippet
use osquery_client::OsqueryClient;
use isolation::IsolationManager;

pub struct CommandHandler {
    osquery: OsqueryClient,
    isolation: IsolationManager,
}

impl CommandHandler {
    pub async fn handle(&self, msg: ServerMessage) -> Result<Value, String> {
        let command = msg.payload.get("command")
            .and_then(|v| v.as_str())
            .ok_or("missing command field")?;

        match command {
            "run_query" => {
                let sql = msg.payload.get("sql")
                    .and_then(|v| v.as_str())
                    .ok_or("missing sql field")?;
                let results = self.osquery.query(sql).await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(results).unwrap_or_default())
            }
            "isolate" => {
                self.isolation.enable().await.map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"status": "isolated"}))
            }
            "unisolate" => {
                self.isolation.disable().await.map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"status": "unisolated"}))
            }
            _ => Err(format!("unknown command: {command}")),
        }
    }
}
