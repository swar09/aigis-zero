use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::schema::alerts;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionSource {
    YaraX,
    Sigma,
    Mitre,
    Behavioral,
}

impl DetectionSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::YaraX => "yara_x",
            Self::Sigma => "sigma",
            Self::Mitre => "mitre",
            Self::Behavioral => "behavioral",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub id: String,
    pub node_id: Uuid,
    pub hostname: String,
    pub event_type: String,
    pub timestamp_ns: i64,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub raw_sequence_id: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = alerts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAlertEntity {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_id: Uuid,
    pub node_id: Uuid,
    pub hostname: String,
    pub severity: AlertSeverity,
    pub source: DetectionSource,
    pub mitre_technique_id: Option<String>,
    pub mitre_tactic: Option<String>,
    pub description: String,
    pub triggering_event_id: Option<String>,
    pub threat_score: f32,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl From<&Alert> for NewAlertEntity {
    fn from(alert: &Alert) -> Self {
        Self {
            alert_id: alert.alert_id,
            node_id: alert.node_id,
            hostname: alert.hostname.clone(),
            severity: alert.severity.as_str().to_string(),
            source: alert.source.as_str().to_string(),
            mitre_technique_id: alert.mitre_technique_id.clone(),
            mitre_tactic: alert.mitre_tactic.clone(),
            description: alert.description.clone(),
            triggering_event_id: alert.triggering_event_id.clone(),
            threat_score: alert.threat_score,
            status: alert.status.clone(),
            created_at: alert.created_at,
        }
    }
}
