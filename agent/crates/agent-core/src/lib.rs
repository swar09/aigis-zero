pub mod config;
pub mod orchestrator;
pub mod command_handler;

use anyhow::Result;
use tokio_util::sync::CancellationToken;
use std::sync::Arc;
// Assume these exist
use osquery_client::OsqueryClient;
use event_buffer::EventBuffer;
use fleet_client::FleetClient;
use command_handler::CommandHandler;

pub struct AgentCore {
    pub shutdown: CancellationToken,
    pub osquery: Arc<OsqueryClient>,
    pub buffer: Arc<EventBuffer>,
    pub command_handler: Arc<CommandHandler>,
    pub fleet_client: Arc<FleetClient>,
}

impl AgentCore {
    pub async fn run(&self) -> Result<()> {
        let shutdown = self.shutdown.clone();

        // Task 1: Osquery polling loop
        let osquery = self.osquery.clone();
        let buffer1 = self.buffer.clone();
        let osquery_task = tokio::spawn(async move {
            // ... existing osquery loop ...
        });

        // Task 2: Command listener
        let cmd_handler = self.command_handler.clone();
        let fleet = self.fleet_client.clone();
        let command_task = tokio::spawn(async move {
            // Listen for incoming ServerMessages
            // Process commands
            // Send responses
        });

        // Wait for shutdown
        shutdown.cancelled().await;
        osquery_task.abort();
        command_task.abort();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_osquery_loop_produces_events() {
        // 1. Osquery loop produces events
    }

    #[tokio::test]
    async fn test_command_handling() {
        // 2. Command handling (run_query, isolate, unisolate)
    }

    #[tokio::test]
    async fn test_shutdown_signal() {
        // 3. Shutdown signal stops all tasks
    }

    #[tokio::test]
    async fn test_event_buffer_integration() {
        // 4. Event buffer integration
    }
}
