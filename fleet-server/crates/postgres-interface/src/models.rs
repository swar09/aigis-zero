use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::{enrollment_events, node_health, nodes};

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = nodes)]
pub struct NewNodeEntity<'a> {
    pub node_id: Uuid,
    pub machine_id: &'a str,
    pub hostname: &'a str,
    pub os_version: &'a str,
    pub agent_version: &'a str,
    pub agent_status: &'a str,
    pub operator_status: &'a str,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = node_health)]
pub struct NewNodeHealthEntity<'a> {
    pub health_id: Uuid,
    pub node_id: Uuid,
    pub agent_status: &'a str,
    pub events_buffered: i64,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = enrollment_events)]
pub struct NewEnrollmentEventEntity<'a> {
    pub event_id: Uuid,
    pub node_id: Uuid,
    pub event_type: &'a str,
    pub hostname: &'a str,
    pub os_version: &'a str,
    pub agent_version: &'a str,
    pub enrolled_at: DateTime<Utc>,
}
