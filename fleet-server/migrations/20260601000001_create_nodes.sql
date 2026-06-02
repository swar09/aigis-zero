-- Nodes table: one row per enrolled endpoint.
-- machine_id is the content of /etc/machine-id — hardware-stable identifier.
-- On re-enrollment (agent reinstall) the row is upserted, not duplicated.
CREATE TABLE IF NOT EXISTS nodes (
    node_id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    machine_id       TEXT        NOT NULL UNIQUE,
    hostname         TEXT        NOT NULL,
    os_version       TEXT        NOT NULL,
    agent_version    TEXT        NOT NULL,
    -- agent_status: written by heartbeats — what the agent reports about itself.
    -- Never set by operators. Values: 'healthy' | 'degraded'.
    agent_status     TEXT        NOT NULL DEFAULT 'healthy',
    -- operator_status: written only by operator commands.
    -- Values: 'active' | 'isolated'. Heartbeats NEVER touch this column.
    operator_status  TEXT        NOT NULL DEFAULT 'active',
    first_seen_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_enrolled_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_nodes_machine_id      ON nodes (machine_id);
CREATE INDEX IF NOT EXISTS idx_nodes_agent_status    ON nodes (agent_status);
CREATE INDEX IF NOT EXISTS idx_nodes_operator_status ON nodes (operator_status);
