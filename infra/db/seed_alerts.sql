-- Seed data for edr_alerts
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM alerts LIMIT 1) THEN

    INSERT INTO alerts (alert_id, node_id, hostname, severity, source, mitre_technique_id, mitre_tactic, description, triggering_event_id, threat_score, status, created_at) VALUES
      (
        'b1b2c3d4-0001-0000-0000-000000000001',
        'a1b2c3d4-0003-0000-0000-000000000003',
        'prod-web-01',
        'critical',
        'yara_x',
        'T1059.004',
        'Execution',
        'Reverse shell spawned from /tmp/bash_rev.sh with interactive connection to remote C2',
        'c1b2c3d4-0001-0000-0000-000000000001',
        94.5,
        'open',
        now() - INTERVAL '3 hours'
      ),
      (
        'b1b2c3d4-0002-0000-0000-000000000002',
        'a1b2c3d4-0001-0000-0000-000000000001',
        'dev-linux-01',
        'high',
        'sigma',
        'T1003.008',
        'Credential Access',
        'Unauthorized attempt to read /etc/shadow by unprivileged user',
        'c1b2c3d4-0002-0000-0000-000000000002',
        78.0,
        'acknowledged',
        now() - INTERVAL '6 hours'
      ),
      (
        'b1b2c3d4-0003-0000-0000-000000000003',
        'a1b2c3d4-0002-0000-0000-000000000002',
        'dev-linux-02',
        'medium',
        'mitre',
        'T1046',
        'Discovery',
        'Rapid port scanning detected across internal 192.168.1.0/24 subnet',
        'c1b2c3d4-0003-0000-0000-000000000003',
        52.0,
        'open',
        now() - INTERVAL '1 day'
      ),
      (
        'b1b2c3d4-0004-0000-0000-000000000004',
        'a1b2c3d4-0004-0000-0000-000000000004',
        'prod-db-01',
        'low',
        'yara_x',
        'T1082',
        'Discovery',
        'System information enumeration commands executed via cron script',
        'c1b2c3d4-0004-0000-0000-000000000004',
        22.5,
        'dismissed',
        now() - INTERVAL '2 days'
      );

  END IF;
END;
$$;
