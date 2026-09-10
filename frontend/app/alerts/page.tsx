'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { AlertTriangle, RefreshCw, Search, X } from 'lucide-react';
import { getAlerts, updateAlertStatus } from '../../lib/api';
import { AlertEntity } from '../../lib/types';
import { StatusBadge } from '../../components/StatusBadge';
import { Drawer } from '../../components/ui/Drawer';

export default function AlertsPage() {
  const [alerts, setAlerts] = useState<AlertEntity[]>([]);
  const [selectedSeverity, setSelectedSeverity] = useState<string>('all');
  const [selectedStatus, setSelectedStatus] = useState<string>('all');
  const [searchTerm, setSearchTerm] = useState('');
  const [loading, setLoading] = useState(true);

  // Selected alert for detail drawer
  const [selectedAlert, setSelectedAlert] = useState<AlertEntity | null>(null);
  const [analystComment, setAnalystComment] = useState('');
  const [statusUpdating, setStatusUpdating] = useState(false);

  const fetchAlerts = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getAlerts({
        severity: selectedSeverity !== 'all' ? selectedSeverity : undefined,
        status: selectedStatus !== 'all' ? selectedStatus : undefined,
        limit: 100,
      });
      setAlerts(data);
    } catch (err) {
      console.error('Failed to load alerts', err);
    } finally {
      setLoading(false);
    }
  }, [selectedSeverity, selectedStatus]);

  useEffect(() => {
    fetchAlerts();
  }, [fetchAlerts]);

  async function handleStatusChange(alertId: string, newStatus: string, comment?: string) {
    setStatusUpdating(true);
    try {
      await updateAlertStatus(alertId, newStatus, comment);
      setAlerts((prev) =>
        prev.map((a) => (a.alert_id === alertId ? { ...a, status: newStatus } : a))
      );
      if (selectedAlert && selectedAlert.alert_id === alertId) {
        setSelectedAlert({ ...selectedAlert, status: newStatus });
      }
    } catch (err) {
      console.error('Failed to update alert status', err);
    } finally {
      setStatusUpdating(false);
    }
  }

  const filteredAlerts = alerts.filter((a) => {
    if (!searchTerm) return true;
    const term = searchTerm.toLowerCase();
    return (
      a.description.toLowerCase().includes(term) ||
      a.hostname.toLowerCase().includes(term) ||
      (a.mitre_technique_id && a.mitre_technique_id.toLowerCase().includes(term))
    );
  });

  useEffect(() => {
    document.title = 'Alerts';
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '28px' }}>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'flex-start',
          borderBottom: '1px solid var(--border-dark)',
          paddingBottom: '20px',
        }}
      >
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <AlertTriangle size={26} style={{ color: 'var(--fg)' }} />
            <h1
              style={{
                fontFamily: 'var(--font-display)',
                fontSize: '32px',
                fontWeight: 700,
                letterSpacing: '-0.5px',
                color: 'var(--fg)',
              }}
            >
              Alerts
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
            Threat detections identified by YARA-X rule inspection and MITRE ATT&amp;CK behavior analysis.
          </p>
        </div>
        <button
          onClick={fetchAlerts}
          className="btn-secondary"
          style={{ padding: '8px 14px' }}
        >
          <RefreshCw size={13} />
          <span>Refresh</span>
        </button>
      </div>

      {/* Filter and search bar */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
        <div style={{ display: 'flex', gap: '16px', alignItems: 'center', flexWrap: 'wrap' }}>
          <div style={{ position: 'relative', width: '320px' }}>
            <Search
              size={15}
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
              placeholder="Search description, host, technique..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="input-text"
              style={{ paddingLeft: '36px' }}
            />
          </div>

          <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: '11px',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--muted-fg)',
                marginRight: '4px',
              }}
            >
              Severity:
            </span>
            {['all', 'critical', 'high', 'medium', 'low'].map((sev) => {
              const isSelected = selectedSeverity === sev;
              return (
                <button
                  key={sev}
                  onClick={() => setSelectedSeverity(sev)}
                  style={{
                    padding: '4px 10px',
                    fontFamily: 'var(--font-mono)',
                    fontSize: '11px',
                    textTransform: 'uppercase',
                    letterSpacing: '0.04em',
                    border: '1px solid var(--border-dark)',
                    background: isSelected ? 'var(--fg)' : 'var(--bg)',
                    color: isSelected ? 'var(--bg)' : 'var(--fg)',
                    fontWeight: isSelected ? 600 : 400,
                    cursor: 'pointer',
                  }}
                >
                  {sev}
                </button>
              );
            })}
          </div>

          <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: '11px',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--muted-fg)',
                marginRight: '4px',
              }}
            >
              Status:
            </span>
            {['all', 'open', 'acknowledged', 'dismissed'].map((st) => {
              const isSelected = selectedStatus === st;
              return (
                <button
                  key={st}
                  onClick={() => setSelectedStatus(st)}
                  style={{
                    padding: '4px 10px',
                    fontFamily: 'var(--font-mono)',
                    fontSize: '11px',
                    textTransform: 'uppercase',
                    letterSpacing: '0.04em',
                    border: '1px solid var(--border-dark)',
                    background: isSelected ? 'var(--fg)' : 'var(--bg)',
                    color: isSelected ? 'var(--bg)' : 'var(--fg)',
                    fontWeight: isSelected ? 600 : 400,
                    cursor: 'pointer',
                  }}
                >
                  {st}
                </button>
              );
            })}
          </div>
        </div>
      </div>

      {/* Detections table */}
      <div className="table-container">
        <table className="data-table">
          <thead>
            <tr>
              <th>SEVERITY</th>
              <th>DESCRIPTION</th>
              <th>HOSTNAME</th>
              <th>TECHNIQUE</th>
              <th>THREAT SCORE</th>
              <th>STATUS</th>
              <th style={{ textAlign: 'right' }}>ACTION</th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              <tr>
                <td
                  colSpan={7}
                  style={{
                    textAlign: 'center',
                    padding: '36px',
                    fontFamily: 'var(--font-mono)',
                    color: 'var(--muted-fg)',
                    fontSize: '13px',
                  }}
                >
                  Loading security detections...
                </td>
              </tr>
            ) : filteredAlerts.length === 0 ? (
              <tr>
                <td
                  colSpan={7}
                  style={{
                    textAlign: 'center',
                    padding: '48px',
                    fontFamily: 'var(--font-mono)',
                    color: 'var(--muted-fg)',
                    fontSize: '13px',
                  }}
                >
                  No detections matching current filter set.
                </td>
              </tr>
            ) : (
              filteredAlerts.map((alert) => (
                <tr key={alert.alert_id}>
                  <td>
                    <StatusBadge type="severity" value={alert.severity} />
                  </td>
                  <td style={{ maxWidth: '340px' }}>
                    <div
                      style={{
                        fontWeight: 600,
                        color: 'var(--fg)',
                        cursor: 'pointer',
                      }}
                      onClick={() => setSelectedAlert(alert)}
                    >
                      {alert.description}
                    </div>
                    <div
                      style={{
                        fontSize: '11px',
                        fontFamily: 'var(--font-mono)',
                        color: 'var(--muted-fg)',
                        marginTop: '3px',
                      }}
                    >
                      Source: {alert.source} | {alert.created_at}
                    </div>
                  </td>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: '13px', color: 'var(--muted-fg)' }}>
                    {alert.hostname}
                  </td>
                  <td>
                    {alert.mitre_technique_id ? (
                      <span
                        style={{
                          display: 'inline-block',
                          padding: '2px 6px',
                          background: 'var(--muted-bg)',
                          border: '1px solid var(--border-dark)',
                          fontFamily: 'var(--font-mono)',
                          fontSize: '11px',
                        }}
                      >
                        {alert.mitre_technique_id}
                      </span>
                    ) : (
                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>N/A</span>
                    )}
                  </td>
                  <td style={{ fontFamily: 'var(--font-mono)', fontWeight: 700 }}>
                    {alert.threat_score.toFixed(1)}
                  </td>
                  <td>
                    <select
                      value={alert.status}
                      onChange={(e) => handleStatusChange(alert.alert_id, e.target.value)}
                      disabled={statusUpdating}
                      style={{
                        padding: '4px 8px',
                        fontSize: '11px',
                        fontFamily: 'var(--font-mono)',
                        border: '1px solid var(--border-dark)',
                        background: 'var(--bg)',
                        color: 'var(--fg)',
                        textTransform: 'uppercase',
                      }}
                    >
                      <option value="open">Open</option>
                      <option value="acknowledged">Acknowledged</option>
                      <option value="dismissed">Dismissed</option>
                    </select>
                  </td>
                  <td style={{ textAlign: 'right' }}>
                    <button
                      onClick={() => setSelectedAlert(alert)}
                      className="btn-secondary"
                      style={{ padding: '4px 10px', fontSize: '11px' }}
                    >
                      Inspect
                    </button>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {/* Alert Detail Drawer */}
      <Drawer
        isOpen={!!selectedAlert}
        onClose={() => setSelectedAlert(null)}
        title="Detection Analysis"
        subtitle={selectedAlert ? `Alert ID: ${selectedAlert.alert_id}` : undefined}
      >
        {selectedAlert && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <div className="card" style={{ padding: '18px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px' }}>
                  <StatusBadge type="severity" value={selectedAlert.severity} />
                  <span style={{ fontSize: '12px', fontFamily: 'var(--font-mono)', color: 'var(--muted-fg)' }}>
                    Threat Score: <strong>{selectedAlert.threat_score.toFixed(1)}</strong>
                  </span>
                </div>
                <h3 style={{ fontSize: '16px', fontWeight: 600, color: 'var(--fg)', lineHeight: 1.4 }}>
                  {selectedAlert.description}
                </h3>
              </div>

              <div className="card" style={{ padding: '18px' }}>
                <h4
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: '11px',
                    fontWeight: 700,
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                    marginBottom: '12px',
                    borderBottom: '1px solid var(--border-light)',
                    paddingBottom: '6px',
                  }}
                >
                  Classification &amp; Metadata
                </h4>
                <div style={{ display: 'grid', gridTemplateColumns: '130px 1fr', gap: '10px', fontSize: '13px' }}>
                  <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Hostname:</span>
                  <span style={{ fontWeight: 600, fontFamily: 'var(--font-mono)' }}>{selectedAlert.hostname}</span>

                  <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Node ID:</span>
                  <span style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>{selectedAlert.node_id}</span>

                  <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Technique ID:</span>
                  <span style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
                    {selectedAlert.mitre_technique_id || 'N/A'}
                  </span>

                  <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>MITRE Tactic:</span>
                  <span style={{ fontFamily: 'var(--font-mono)' }}>{selectedAlert.mitre_tactic || 'N/A'}</span>

                  <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Trigger Event:</span>
                  <span style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
                    {selectedAlert.triggering_event_id || 'N/A'}
                  </span>

                  <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Created (UTC):</span>
                  <span style={{ fontFamily: 'var(--font-mono)' }}>{selectedAlert.created_at}</span>
                </div>
              </div>

              <div className="card" style={{ padding: '18px' }}>
                <h4
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: '11px',
                    fontWeight: 700,
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                    marginBottom: '14px',
                    borderBottom: '1px solid var(--border-light)',
                    paddingBottom: '6px',
                  }}
                >
                  Analyst Triage &amp; Review
                </h4>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                  <div>
                    <label
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: '11px',
                        color: 'var(--muted-fg)',
                        textTransform: 'uppercase',
                        display: 'block',
                        marginBottom: '6px',
                      }}
                    >
                      Update Triage State:
                    </label>
                    <div style={{ display: 'flex', gap: '8px' }}>
                      {['open', 'acknowledged', 'dismissed'].map((st) => (
                        <button
                          key={st}
                          onClick={() => handleStatusChange(selectedAlert.alert_id, st, analystComment)}
                          className={selectedAlert.status === st ? 'btn-primary' : 'btn-secondary'}
                          style={{ padding: '6px 12px', fontSize: '11px' }}
                        >
                          {st}
                        </button>
                      ))}
                    </div>
                  </div>

                  <div>
                    <label
                      htmlFor="resolution-remarks"
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: '11px',
                        color: 'var(--muted-fg)',
                        textTransform: 'uppercase',
                        display: 'block',
                        marginBottom: '6px',
                      }}
                    >
                      Resolution Remarks:
                    </label>
                    <textarea
                      id="resolution-remarks"
                      value={analystComment}
                      onChange={(e) => setAnalystComment(e.target.value)}
                      placeholder="Add investigation remarks, false positive rationale, or ticket links..."
                      rows={3}
                      className="input-text"
                    />
                  </div>
                </div>
              </div>
            </div>
          )}
        </Drawer>
      </div>
  );
}
