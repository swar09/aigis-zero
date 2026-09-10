'use client';

import React, { useState } from 'react';
import { Check, ChevronDown, ChevronRight, Copy } from 'lucide-react';
import { EventLogEntity } from '../lib/types';

interface LogTableProps {
  logs: EventLogEntity[];
  loading?: boolean;
}

export function LogTable({ logs, loading = false }: LogTableProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  function copyPayload(log: EventLogEntity, e: React.MouseEvent) {
    e.stopPropagation();
    navigator.clipboard.writeText(JSON.stringify(log.payload, null, 2));
    setCopiedId(log.event_id);
    setTimeout(() => setCopiedId(null), 2000);
  }

  function getPayloadSummary(log: EventLogEntity): string {
    const p = log.payload as Record<string, unknown>;
    if (!p) return '-';
    if (p.path) return String(p.path);
    if (p.cmdline) return String(p.cmdline);
    if (p.remote_address) return `${p.remote_address}:${p.remote_port || ''}`;
    if (p.address) return `${p.address}:${p.port || ''}`;
    if (p.user) return `user: ${p.user}`;
    return JSON.stringify(p).slice(0, 80);
  }

  return (
    <div className="table-container">
      <table className="data-table">
        <thead>
          <tr>
            <th style={{ width: '32px' }}></th>
            <th>TIMESTAMP (UTC)</th>
            <th>HOST</th>
            <th>EVENT TYPE</th>
            <th>PAYLOAD SUMMARY</th>
            <th style={{ textAlign: 'right' }}>ACTION</th>
          </tr>
        </thead>
        <tbody>
          {loading ? (
            <tr>
              <td
                colSpan={6}
                style={{
                  textAlign: 'center',
                  padding: '36px',
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--muted-fg)',
                }}
              >
                Querying telemetry logs...
              </td>
            </tr>
          ) : logs.length === 0 ? (
            <tr>
              <td
                colSpan={6}
                style={{
                  textAlign: 'center',
                  padding: '40px',
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--muted-fg)',
                }}
              >
                No telemetry log entries found matching current filters.
              </td>
            </tr>
          ) : (
            logs.map((log) => {
              const isExpanded = expandedId === log.event_id;
              return (
                <React.Fragment key={log.event_id}>
                  <tr
                    onClick={() => setExpandedId(isExpanded ? null : log.event_id)}
                    style={{ cursor: 'pointer' }}
                  >
                    <td style={{ textAlign: 'center', padding: '12px 8px' }}>
                      {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                    </td>
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: '12px', whiteSpace: 'nowrap' }}>
                      {log.recorded_at ? new Date(log.recorded_at).toISOString().replace('T', ' ').slice(0, 19) : '-'}
                    </td>
                    <td>
                      <span style={{ fontWeight: 600, fontFamily: 'var(--font-mono)', fontSize: '13px' }}>
                        {log.hostname}
                      </span>
                    </td>
                    <td>
                      <span
                        style={{
                          fontFamily: 'var(--font-mono)',
                          fontSize: '11px',
                          textTransform: 'uppercase',
                          padding: '2px 6px',
                          border: '1px solid var(--border-dark)',
                          background: 'var(--muted-bg)',
                          color: 'var(--fg)',
                          whiteSpace: 'nowrap',
                        }}
                      >
                        {log.event_type}
                      </span>
                    </td>
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: '12px', color: 'var(--muted-fg)', maxWidth: '400px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {getPayloadSummary(log)}
                    </td>
                    <td style={{ textAlign: 'right' }}>
                      <button
                        type="button"
                        onClick={(e) => copyPayload(log, e)}
                        className="btn-secondary"
                        style={{ padding: '4px 8px', fontSize: '11px', display: 'inline-flex', alignItems: 'center', gap: '4px' }}
                        title="Copy raw JSON payload"
                      >
                        {copiedId === log.event_id ? (
                          <>
                            <Check size={12} style={{ color: 'var(--color-success)' }} />
                            <span>Copied</span>
                          </>
                        ) : (
                          <>
                            <Copy size={12} />
                            <span>JSON</span>
                          </>
                        )}
                      </button>
                    </td>
                  </tr>

                  {isExpanded && (
                    <tr>
                      <td colSpan={6} style={{ background: 'var(--muted-bg)', padding: '16px 20px', borderBottom: '2px solid var(--border-dark)' }}>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                            <div style={{ display: 'flex', gap: '16px', fontSize: '11px', fontFamily: 'var(--font-mono)', color: 'var(--muted-fg)' }}>
                              <span>EVENT ID: <strong style={{ color: 'var(--fg)' }}>{log.event_id}</strong></span>
                              <span>NODE ID: <strong style={{ color: 'var(--fg)' }}>{log.node_id}</strong></span>
                              {log.raw_sequence_id && <span>SEQ: <strong style={{ color: 'var(--fg)' }}>{log.raw_sequence_id}</strong></span>}
                            </div>
                            <button
                              type="button"
                              onClick={(e) => copyPayload(log, e)}
                              className="btn-secondary"
                              style={{ padding: '4px 10px', fontSize: '11px', display: 'inline-flex', alignItems: 'center', gap: '6px' }}
                            >
                              {copiedId === log.event_id ? <Check size={13} style={{ color: 'var(--color-success)' }} /> : <Copy size={13} />}
                              <span>{copiedId === log.event_id ? 'Copied to clipboard' : 'Copy JSON payload'}</span>
                            </button>
                          </div>

                          <pre
                            style={{
                              background: 'var(--bg)',
                              border: '1px solid var(--border-dark)',
                              padding: '14px',
                              fontFamily: 'var(--font-mono)',
                              fontSize: '12px',
                              lineHeight: '1.5',
                              overflowX: 'auto',
                              maxHeight: '320px',
                              color: 'var(--fg)',
                              margin: 0,
                            }}
                          >
                            {JSON.stringify(log.payload, null, 2)}
                          </pre>
                        </div>
                      </td>
                    </tr>
                  )}
                </React.Fragment>
              );
            })
          )}
        </tbody>
      </table>
    </div>
  );
}
