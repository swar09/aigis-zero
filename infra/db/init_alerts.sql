-- Schema definition for edr_alerts database

CREATE TABLE IF NOT EXISTS alerts (
    alert_id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id             UUID        NOT NULL,
    hostname            TEXT        NOT NULL,
    severity            TEXT        NOT NULL, -- 'low' | 'medium' | 'high' | 'critical'
    source              TEXT        NOT NULL, -- 'yara_x' | 'sigma' | 'mitre'
    mitre_technique_id  TEXT,
    mitre_tactic        TEXT,
    description         TEXT        NOT NULL,
    triggering_event_id TEXT,
    threat_score        REAL        NOT NULL DEFAULT 0.0,
    status              TEXT        NOT NULL DEFAULT 'open', -- 'open' | 'acknowledged' | 'dismissed'
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_alerts_node_id         ON alerts (node_id);
CREATE INDEX IF NOT EXISTS idx_alerts_severity        ON alerts (severity);
CREATE INDEX IF NOT EXISTS idx_alerts_status          ON alerts (status);
CREATE INDEX IF NOT EXISTS idx_alerts_mitre_technique ON alerts (mitre_technique_id);
CREATE INDEX IF NOT EXISTS idx_alerts_created_at      ON alerts (created_at DESC);
