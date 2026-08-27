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
