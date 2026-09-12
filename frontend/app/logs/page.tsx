'use client';

import React, { useState, useEffect } from 'react';
import { FileText, RefreshCw, Search } from 'lucide-react';
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

export default function LogsPage() {
  const [eventType, setEventType] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [limit, setLimit] = useState('100');
  const [logs, setLogs] = useState<EventLogEntity[]>([]);
  const [loading, setLoading] = useState(false);

  async function fetchLogs() {
    setLoading(true);
    try {
      const data = await getLogs({
        event_type: eventType !== 'all' ? eventType : undefined,
        limit: parseInt(limit, 10) || 100,
      });
      setLogs(data);
    } catch (err) {
      console.error('Failed to load event logs', err);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchLogs();
  }, [eventType, limit]);

  const filteredLogs = logs.filter((log) => {
    if (!searchQuery) return true;
    const term = searchQuery.toLowerCase();
    return (
      log.hostname.toLowerCase().includes(term) ||
      log.node_id.toLowerCase().includes(term) ||
      log.event_type.toLowerCase().includes(term) ||
      JSON.stringify(log.payload).toLowerCase().includes(term)
    );
  });

  useEffect(() => {
    document.title = 'Logs';
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <div style={{ borderBottom: '1px solid var(--border-dark)', paddingBottom: '16px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <FileText size={26} style={{ color: 'var(--fg)' }} />
          <h1
            style={{
              fontFamily: 'var(--font-display)',
              fontSize: '32px',
              fontWeight: 700,
              letterSpacing: '-0.5px',
              color: 'var(--fg)',
            }}
          >
            Logs
          </h1>
        </div>
        <p
          style={{
            fontSize: '13px',
            fontFamily: 'var(--font-mono)',
            color: 'var(--muted-fg)',
            marginTop: '6px',
          }}
        >
          Audit trail and telemetry events captured from enrolled endpoint agents.
        </p>
      </div>

      <div
        className="card"
        style={{
          padding: '16px 20px',
          display: 'flex',
          gap: '12px',
          alignItems: 'center',
          flexWrap: 'wrap',
        }}
      >
        <div style={{ minWidth: '190px' }}>
          <select
            value={eventType}
            onChange={(e) => setEventType(e.target.value)}
            className="input-text"
            aria-label="Filter by event type"
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
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="input-text"
            style={{ paddingLeft: '34px' }}
            aria-label="Search telemetry logs"
          />
        </div>

        <div style={{ width: '120px' }}>
          <select
            value={limit}
            onChange={(e) => setLimit(e.target.value)}
            className="input-text"
            aria-label="Rows per page"
          >
            <option value="50">50 rows</option>
            <option value="100">100 rows</option>
            <option value="250">250 rows</option>
            <option value="500">500 rows</option>
          </select>
        </div>

        <button
          type="button"
          onClick={fetchLogs}
          className="btn-secondary"
          disabled={loading}
          style={{ padding: '8px 14px' }}
          aria-label="Refresh telemetry logs"
        >
          <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
          <span>Refresh</span>
        </button>
      </div>

      <LogTable logs={filteredLogs} loading={loading} />
    </div>
  );
}
