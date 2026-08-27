// Diesel schema definitions for alerts table in edr_alerts database

diesel::table! {
    alerts (alert_id) {
        alert_id -> Uuid,
        node_id -> Uuid,
        hostname -> Text,
        severity -> Text,
        source -> Text,
        mitre_technique_id -> Nullable<Text>,
        mitre_tactic -> Nullable<Text>,
        description -> Text,
        triggering_event_id -> Nullable<Text>,
        threat_score -> Float4,
        status -> Text,
        created_at -> Timestamptz,
    }
}
