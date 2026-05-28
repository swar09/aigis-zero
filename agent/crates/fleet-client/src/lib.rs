#[expect(dead_code)]
pub mod connection;
pub mod enrollment;
pub mod heartbeat;
pub mod stream;
pub mod types;

use crate::connection::FleetConnection;
use crate::enrollment::AgentEnrollment;
use crate::heartbeat::HeartbeatManager;
use crate::stream::EventStreamManager;
use crate::types::{AgentEvent, ConnectionState, EnrollmentResult, RegisterRequest, ServerCommand};
use anyhow::{Result, anyhow};
use tokio::sync::{mpsc, watch};

pub struct FleetConfig {
    pub endpoint: String,
}

pub struct FleetClient {
    connection: FleetConnection,
    state_rx: watch::Receiver<ConnectionState>,
    enrollment: Option<EnrollmentResult>,
}

impl FleetClient {
    pub async fn new(config: FleetConfig) -> Result<Self> {
        let (connection, state_rx) = FleetConnection::new(&config.endpoint);
        Ok(Self {
            connection,
            state_rx,
            enrollment: None,
        })
    }

    pub async fn connect(&mut self) -> Result<()> {
        self.connection.connect().await?;
        Ok(())
    }

    pub async fn enroll(&mut self, request: RegisterRequest) -> Result<EnrollmentResult> {
        // We wait for the channel to be ready.
        let channel = self.connection.connect().await?;
        let result = AgentEnrollment::enroll(channel, request).await?;
        self.enrollment = Some(result.clone());
        Ok(result)
    }

    pub async fn start_stream(
        &mut self,
        events_rx: mpsc::Receiver<AgentEvent>,
    ) -> Result<mpsc::Receiver<ServerCommand>> {
        let channel = self.connection.connect().await?;
        let token = self
            .enrollment
            .as_ref()
            .ok_or_else(|| anyhow!("Not enrolled"))?
            .token
            .clone();
        EventStreamManager::start(channel, token, events_rx).await
    }

    pub async fn start_heartbeat(&mut self, interval_secs: u64) -> Result<()> {
        let channel = self.connection.connect().await?;
        let enrollment = self
            .enrollment
            .as_ref()
            .ok_or_else(|| anyhow!("Not enrolled"))?;
        let token = enrollment.token.clone();
        let node_id = enrollment.node_id.clone();
        HeartbeatManager::start(channel, token, node_id, interval_secs).await
    }
}
