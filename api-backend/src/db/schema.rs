use diesel::table;

table! {
    nodes (node_id) {
        node_id -> Uuid,
        machine_id -> Text,
        hostname -> Text,
        os_version -> Text,
        agent_version -> Text,
        agent_status -> Text,
        operator_status -> Text,
        first_seen_at -> Timestamptz,
        last_enrolled_at -> Timestamptz,
    }
}

table! {
    node_health (health_id) {
        health_id -> Uuid,
        node_id -> Uuid,
        agent_status -> Text,
        events_buffered -> BigInt,
        recorded_at -> Timestamptz,
    }
}

table! {
    enrollment_events (event_id) {
        event_id -> Uuid,
        node_id -> Uuid,
        event_type -> Text,
        hostname -> Text,
        os_version -> Text,
        agent_version -> Text,
        enrolled_at -> Timestamptz,
    }
}

table! {
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

table! {
    event_logs (event_id) {
        event_id -> Uuid,
        node_id -> Uuid,
        event_type -> Text,
        hostname -> Text,
        payload -> Jsonb,
        raw_sequence_id -> Nullable<Text>,
        recorded_at -> Timestamptz,
    }
}
