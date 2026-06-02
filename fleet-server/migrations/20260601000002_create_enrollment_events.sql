-- Enrollment event log: append-only audit trail.
-- Every call to RegisterAgent — new or re-enroll — writes one row.
-- Never update or delete rows from this table.
CREATE TABLE IF NOT EXISTS enrollment_events (
    event_id       UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id        UUID        NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
    event_type     TEXT        NOT NULL, -- 'new_enrollment' | 're_enrollment'
    hostname       TEXT        NOT NULL,
    os_version     TEXT        NOT NULL,
    agent_version  TEXT        NOT NULL,
    enrolled_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_enrollment_events_node_id
    ON enrollment_events (node_id, enrolled_at DESC);

CREATE INDEX IF NOT EXISTS idx_enrollment_events_enrolled_at
    ON enrollment_events (enrolled_at DESC);
