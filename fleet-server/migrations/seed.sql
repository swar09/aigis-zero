-- Seed data for local development. Runs once on first container boot.
-- Idempotent: skips if nodes table already has data.
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM nodes LIMIT 1) THEN

    INSERT INTO nodes (node_id, machine_id, hostname, os_version, agent_version, agent_status, operator_status, first_seen_at, last_enrolled_at) VALUES
      ('a1b2c3d4-0001-0000-0000-000000000001', 'mid-aaa-001', 'dev-linux-01', 'Ubuntu 24.04', '0.1.0', 'healthy',  'active',   now() - INTERVAL '30 days', now() - INTERVAL '2 hours'),
      ('a1b2c3d4-0002-0000-0000-000000000002', 'mid-aaa-002', 'dev-linux-02', 'Ubuntu 22.04', '0.1.0', 'healthy',  'active',   now() - INTERVAL '25 days', now() - INTERVAL '1 hour'),
      ('a1b2c3d4-0003-0000-0000-000000000003', 'mid-aaa-003', 'prod-web-01',  'Debian 12',   '0.1.0', 'degraded', 'isolated', now() - INTERVAL '10 days', now() - INTERVAL '3 days'),
      ('a1b2c3d4-0004-0000-0000-000000000004', 'mid-aaa-004', 'prod-db-01',   'RHEL 9.3',    '0.1.0', 'healthy',  'active',   now() - INTERVAL '5 days',  now() - INTERVAL '30 minutes'),
      ('a1b2c3d4-0005-0000-0000-000000000005', 'mid-aaa-005', 'prod-db-02',   'RHEL 9.3',    '0.1.0', 'healthy',  'active',   now() - INTERVAL '1 day',   now() - INTERVAL '10 minutes');

    INSERT INTO enrollment_events (node_id, event_type, hostname, os_version, agent_version, enrolled_at) VALUES
      ('a1b2c3d4-0001-0000-0000-000000000001', 'new_enrollment', 'dev-linux-01', 'Ubuntu 24.04', '0.1.0', now() - INTERVAL '30 days'),
      ('a1b2c3d4-0001-0000-0000-000000000001', 're_enrollment',  'dev-linux-01', 'Ubuntu 24.04', '0.1.0', now() - INTERVAL '2 hours'),
      ('a1b2c3d4-0002-0000-0000-000000000002', 'new_enrollment', 'dev-linux-02', 'Ubuntu 22.04', '0.1.0', now() - INTERVAL '25 days'),
      ('a1b2c3d4-0002-0000-0000-000000000002', 're_enrollment',  'dev-linux-02', 'Ubuntu 22.04', '0.1.0', now() - INTERVAL '1 hour'),
      ('a1b2c3d4-0003-0000-0000-000000000003', 'new_enrollment', 'prod-web-01',  'Debian 12',    '0.1.0', now() - INTERVAL '10 days'),
      ('a1b2c3d4-0004-0000-0000-000000000004', 'new_enrollment', 'prod-db-01',   'RHEL 9.3',     '0.1.0', now() - INTERVAL '5 days'),
      ('a1b2c3d4-0005-0000-0000-000000000005', 'new_enrollment', 'prod-db-02',   'RHEL 9.3',     '0.1.0', now() - INTERVAL '1 day');

    INSERT INTO node_health (node_id, agent_status, events_buffered, recorded_at)
    SELECT
      node_id,
      agent_status,
      floor(random() * 100)::BIGINT,
      now() - (INTERVAL '1 minute' * generate_series(1, 60))
    FROM nodes;

  END IF;
END;
$$;
