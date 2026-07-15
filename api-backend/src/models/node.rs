use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::schema::{node_health, nodes};

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = nodes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NodeEntity {
    pub node_id: Uuid,
    pub machine_id: String,
    pub hostname: String,
    pub os_version: String,
    pub agent_version: String,
    pub agent_status: String,
    pub operator_status: String,
    pub first_seen_at: DateTime<Utc>,
    pub last_enrolled_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = node_health)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NodeHealthEntity {
    pub health_id: Uuid,
    pub node_id: Uuid,
    pub agent_status: String,
    pub events_buffered: i64,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct NodeFilterParams {
    pub agent_status: Option<String>,
    pub operator_status: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct NodeSummaryDto {
    pub node_id: Uuid,
    pub machine_id: String,
    pub hostname: String,
    pub os_version: String,
    pub agent_version: String,
    pub agent_status: String,
    pub operator_status: String,
    pub first_seen_at: DateTime<Utc>,
    pub last_enrolled_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct NodeDetailDto {
    pub node: NodeEntity,
    pub recent_health: Vec<NodeHealthEntity>,
}

#[derive(Debug, Deserialize)]
pub struct IsolateNodeRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IsolateNodeResponse {
    pub node_id: Uuid,
    pub operator_status: String,
    pub command_dispatched: bool,
    pub updated_at: DateTime<Utc>,
}
