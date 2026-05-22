-- migrations/001_create_nodes.sql
CREATE TABLE nodes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hostname    VARCHAR(255) NOT NULL,
    os_version  VARCHAR(255),
    agent_version VARCHAR(50),
    machine_id  VARCHAR(64) UNIQUE NOT NULL,
    enrolled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen   TIMESTAMPTZ,
    status      VARCHAR(20) NOT NULL DEFAULT 'online',
                -- 'online' | 'offline' | 'isolated' | 'degraded'
    ip_address  INET,
    CONSTRAINT status_check CHECK (status IN ('online','offline','isolated','degraded'))
);

CREATE TABLE agent_configs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id     UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    config      JSONB NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE pending_commands (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id     UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    command     JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered   BOOLEAN NOT NULL DEFAULT FALSE,
    delivered_at TIMESTAMPTZ
);

CREATE INDEX idx_nodes_status ON nodes(status);
CREATE INDEX idx_pending_commands_undelivered ON pending_commands(node_id, delivered) WHERE delivered = FALSE;
