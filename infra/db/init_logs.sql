-- Schema definition for edr_logs database

CREATE TABLE IF NOT EXISTS event_logs (
    event_id        UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id         UUID        NOT NULL,
    event_type      TEXT        NOT NULL, -- 'process_events' | 'socket_events' | 'file_events'
    hostname        TEXT        NOT NULL,
    payload         JSONB       NOT NULL,
    raw_sequence_id TEXT,
    recorded_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_event_logs_node_id     ON event_logs (node_id);
CREATE INDEX IF NOT EXISTS idx_event_logs_event_type  ON event_logs (event_type);
CREATE INDEX IF NOT EXISTS idx_event_logs_recorded_at ON event_logs (recorded_at DESC);
