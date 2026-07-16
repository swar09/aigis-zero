use std::sync::Arc;

use uuid::Uuid;

use crate::{
    clients::FleetClient,
    error::AppError,
    models::node::{IsolateNodeResponse, NodeDetailDto, NodeFilterParams, NodeSummaryDto},
    repositories::NodeRepository,
};

#[derive(Clone)]
pub struct NodeService {
    node_repo: Arc<dyn NodeRepository>,
    fleet_client: Arc<FleetClient>,
}

impl NodeService {
    pub fn new(node_repo: Arc<dyn NodeRepository>, fleet_client: Arc<FleetClient>) -> Self {
        Self {
            node_repo,
            fleet_client,
        }
    }

    pub async fn list_nodes(
        &self,
        params: NodeFilterParams,
    ) -> Result<(Vec<NodeSummaryDto>, i64), AppError> {
        self.node_repo.find_all(params).await
    }

    pub async fn get_node_by_id(&self, node_id: Uuid) -> Result<NodeDetailDto, AppError> {
        self.node_repo
            .find_by_id(node_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Node with id '{node_id}' not found")))
    }

    pub async fn isolate_node(
        &self,
        node_id: Uuid,
        reason: Option<String>,
    ) -> Result<IsolateNodeResponse, AppError> {
        let res = self
            .node_repo
            .update_operator_status(node_id, "isolated")
            .await?;

        let reason_str = reason.unwrap_or_else(|| "Operator manual isolation".to_string());
        if let Err(e) = self
            .fleet_client
            .send_isolate_command(node_id, true, &reason_str)
            .await
        {
            tracing::warn!(err = %e, %node_id, "Fleet server command dispatch failed");
        }

        Ok(res)
    }

    pub async fn unisolate_node(&self, node_id: Uuid) -> Result<IsolateNodeResponse, AppError> {
        let res = self
            .node_repo
            .update_operator_status(node_id, "active")
            .await?;

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
