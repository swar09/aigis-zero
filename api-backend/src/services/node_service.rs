//! Business logic service for endpoint node inventory and network containment.

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    clients::FleetClient,
    error::AppError,
    models::node::{IsolateNodeResponse, NodeDetailDto, NodeFilterParams, NodeSummaryDto},
    repositories::NodeRepository,
};

/// Service handling endpoint inventory lookups and operator quarantine actions.
#[derive(Clone)]
pub struct NodeService {
    node_repo: Arc<dyn NodeRepository>,
    fleet_client: Arc<FleetClient>,
}

impl NodeService {
    /// Creates a new `NodeService` with the injected repository and fleet control client.
    pub fn new(node_repo: Arc<dyn NodeRepository>, fleet_client: Arc<FleetClient>) -> Self {
        Self {
            node_repo,
            fleet_client,
        }
    }

    /// Lists endpoint nodes matching optional filter parameters with total count.
    pub async fn list_nodes(&self, params: NodeFilterParams) -> Result<(Vec<NodeSummaryDto>, i64), AppError> {
        self.node_repo.find_all(params).await
    }

    /// Retrieves full endpoint details along with recent historical heartbeat telemetry.
    pub async fn get_node_by_id(&self, node_id: Uuid) -> Result<NodeDetailDto, AppError> {
        self.node_repo
            .find_by_id(node_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Node with id '{node_id}' not found")))
    }

    /// Quarantines an endpoint by updating operator status to `isolated` and dispatching isolation commands.
    pub async fn isolate_node(&self, node_id: Uuid, reason: Option<String>) -> Result<IsolateNodeResponse, AppError> {
        let res = self.node_repo.update_operator_status(node_id, "isolated").await?;

        let reason_str = reason.unwrap_or_else(|| "Operator manual isolation".to_string());
        if let Err(e) = self.fleet_client.send_isolate_command(node_id, true, &reason_str).await {
            tracing::warn!(err = %e, %node_id, "Fleet server command dispatch failed");
        }

        Ok(res)
    }

    /// Lifts quarantine from an endpoint by setting operator status to `active` and notifying fleet control.
    pub async fn unisolate_node(&self, node_id: Uuid) -> Result<IsolateNodeResponse, AppError> {
        let res = self.node_repo.update_operator_status(node_id, "active").await?;

        if let Err(e) = self
            .fleet_client
            .send_isolate_command(node_id, false, "Operator un-isolation")
            .await
        {
            tracing::warn!(err = %e, %node_id, "Fleet server command dispatch failed");
        }

        Ok(res)
    }
}
