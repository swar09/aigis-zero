use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::schema::event_logs;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = event_logs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EventLogEntity {
    pub event_id: Uuid,
    pub node_id: Uuid,
    pub event_type: String,
    pub hostname: String,
    pub payload: serde_json::Value,
    pub raw_sequence_id: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LogFilterParams {
    pub node_id: Option<Uuid>,
    pub event_type: Option<String>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
