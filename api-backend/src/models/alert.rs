use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::schema::alerts;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = alerts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AlertEntity {
    pub alert_id: Uuid,
    pub node_id: Uuid,
    pub hostname: String,
    pub severity: String,
    pub source: String,
    pub mitre_technique_id: Option<String>,
    pub mitre_tactic: Option<String>,
    pub description: String,
    pub triggering_event_id: Option<String>,
    pub threat_score: f32,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AlertFilterParams {
    pub severity: Option<String>,
    pub status: Option<String>,
    pub node_id: Option<Uuid>,
    pub mitre_technique: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAlertStatusRequest {
    pub status: String,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateAlertStatusResponse {
    pub alert_id: Uuid,
    pub status: String,
    pub updated_at: DateTime<Utc>,
}
