'use client';

import React, { useState, useEffect } from 'react';
import { Crosshair, Search } from 'lucide-react';
import { getLogs } from '../../lib/api';
import { EventLogEntity } from '../../lib/types';
import { LogTable } from '../../components/LogTable';

const EVENT_TYPES = [
  { value: 'all', label: 'All Telemetry Types' },
  { value: 'process', label: 'Process Executions' },
  { value: 'network', label: 'Network Connections' },
  { value: 'file', label: 'File Integrity Events' },
  { value: 'auth', label: 'Authentication Events' },
  { value: 'listening_ports', label: 'Listening Ports' },
];

export default function ThreatHuntPage() {
  const [eventType, setEventType] = useState('all');
  const [searchHost, setSearchHost] = useState('');
  const [limit, setLimit] = useState('100');
  const [logs, setLogs] = useState<EventLogEntity[]>([]);
  const [loading, setLoading] = useState(false);

  async function executeHunt(e?: React.FormEvent) {
    if (e) e.preventDefault();
    setLoading(true);
    try {
      const data = await getLogs({
        event_type: eventType !== 'all' ? eventType : undefined,
        limit: parseInt(limit, 10) || 100,
      });
      setLogs(data);
    } catch (err) {
      console.error('Threat hunt query failed', err);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    document.title = 'Threat Hunt';
    executeHunt();
  }, [eventType, limit]);

  const filteredLogs = logs.filter((log) => {
    if (!searchHost) return true;
    const term = searchHost.toLowerCase();
    return (
      log.hostname.toLowerCase().includes(term) ||
      log.node_id.toLowerCase().includes(term) ||
      log.event_type.toLowerCase().includes(term) ||
      JSON.stringify(log.payload).toLowerCase().includes(term)
    );
  });

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '28px' }}>
      <div style={{ borderBottom: '1px solid var(--border-dark)', paddingBottom: '20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <Crosshair size={26} style={{ color: 'var(--fg)' }} />
          <h1
            style={{
              fontFamily: 'var(--font-display)',
              fontSize: '32px',
              fontWeight: 700,
              letterSpacing: '-0.5px',
              color: 'var(--fg)',
            }}
          >
            Threat Hunt
          </h1>
        </div>
        <p
          style={{
            fontSize: '14px',
            fontFamily: 'var(--font-mono)',
            color: 'var(--muted-fg)',
            marginTop: '6px',
          }}
        >
          Query structured endpoint telemetry events across process executions, network sockets, and system auth.
        </p>
      </div>

      <form
        onSubmit={executeHunt}
        className="card"
        style={{
          padding: '20px',
          display: 'flex',
          gap: '16px',
          alignItems: 'center',
          flexWrap: 'wrap',
        }}
      >
        <div style={{ minWidth: '180px' }}>
          <select
            value={eventType}
            onChange={(e) => setEventType(e.target.value)}
            className="input-text"
            aria-label="Event type"
          >
            {EVENT_TYPES.map((t) => (
              <option key={t.value} value={t.value}>
                {t.label}
              </option>
            ))}
          </select>
        </div>

        <div style={{ flex: 1, minWidth: '240px', position: 'relative' }}>
          <Search
            size={14}
            aria-hidden="true"
            style={{
              position: 'absolute',
              left: '12px',
              top: '50%',
              transform: 'translateY(-50%)',
              color: 'var(--muted-fg)',
            }}
          />
          <input
            type="text"
            placeholder="Search hostname, event type, or payload attributes..."
            value={searchHost}
            onChange={(e) => setSearchHost(e.target.value)}
            className="input-text"
            style={{ paddingLeft: '34px' }}
            aria-label="Search term"
          />
        </div>

        <div style={{ width: '130px' }}>
          <select
            value={limit}
            onChange={(e) => setLimit(e.target.value)}
            className="input-text"
            aria-label="Result limit"
          >
            <option value="50">50 events</option>
            <option value="100">100 events</option>
            <option value="250">250 events</option>
            <option value="500">500 events</option>
          </select>
        </div>

        <button
          type="submit"
          className="btn-primary"
          disabled={loading}
          style={{ padding: '8px 20px' }}
        >
          {loading ? 'Searching...' : 'Run Query'}
        </button>
      </form>

      <LogTable logs={filteredLogs} loading={loading} />
    </div>
  );
}
