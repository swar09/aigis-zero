-- Node health: time-series heartbeat data. Append-only.
-- One row per heartbeat received. Never updated.
CREATE TABLE IF NOT EXISTS node_health (
    health_id       UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id         UUID        NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
    agent_status    TEXT        NOT NULL, -- 'healthy' | 'degraded'
    events_buffered BIGINT      NOT NULL DEFAULT 0,
    recorded_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Primary query: latest heartbeat for node X
CREATE INDEX IF NOT EXISTS idx_node_health_node_latest
    ON node_health (node_id, recorded_at DESC);

-- Secondary: all nodes not seen in last N minutes
CREATE INDEX IF NOT EXISTS idx_node_health_recorded_at
    ON node_health (recorded_at DESC);
