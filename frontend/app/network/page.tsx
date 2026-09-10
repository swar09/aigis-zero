'use client';

import React, { useState, useEffect } from 'react';
import {
  Activity,
  Check,
  ChevronDown,
  ChevronRight,
  Copy,
  Globe,
  Network,
  RefreshCw,
  Search,
  Server,
  ShieldAlert,
  ShieldCheck,
} from 'lucide-react';
import { getLogs, getNodes, isolateNode, unisolateNode } from '../../lib/api';
import { EventLogEntity, NodeSummaryDto } from '../../lib/types';
import { MetricCard } from '../../components/MetricCard';
import { StatusBadge } from '../../components/StatusBadge';
import { Modal } from '../../components/ui/Modal';

export default function NetworkPage() {
  const [activeTab, setActiveTab] = useState<'telemetry' | 'containment'>('telemetry');
  const [logs, setLogs] = useState<EventLogEntity[]>([]);
  const [nodes, setNodes] = useState<NodeSummaryDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  // Isolation action modal / state
  const [actionNode, setActionNode] = useState<NodeSummaryDto | null>(null);
  const [actionType, setActionType] = useState<'isolate' | 'unisolate'>('isolate');
  const [isolateReason, setIsolateReason] = useState('');
  const [actionLoading, setActionLoading] = useState(false);

  async function loadData() {
    setLoading(true);
    try {
      const [logsData, nodesData] = await Promise.all([
        getLogs({ event_type: 'network', limit: 100 }),
        getNodes({ limit: 100 }),
      ]);
      setLogs(logsData);
      setNodes(nodesData);
    } catch (err) {
      console.error('Failed to load network data', err);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadData();
  }, []);

  const isolatedCount = nodes.filter((n) => n.operator_status === 'isolated').length;
  const uniqueRemoteIps = new Set(
    logs
      .map((l) => (l.payload as Record<string, unknown>)?.remote_address)
      .filter((ip): ip is string => Boolean(ip))
  ).size;

  const filteredLogs = logs.filter((log) => {
    if (!searchQuery) return true;
    const term = searchQuery.toLowerCase();
    const p = log.payload as Record<string, unknown>;
    const remoteAddr = String(p?.remote_address || '').toLowerCase();
    const localAddr = String(p?.local_address || '').toLowerCase();
    return (
      log.hostname.toLowerCase().includes(term) ||
      remoteAddr.includes(term) ||
      localAddr.includes(term) ||
      JSON.stringify(p).toLowerCase().includes(term)
    );
  });

  async function handleIsolationSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!actionNode) return;
    setActionLoading(true);
    try {
      if (actionType === 'isolate') {
        await isolateNode(actionNode.node_id, isolateReason || 'Quarantined via Network console');
      } else {
        await unisolateNode(actionNode.node_id);
      }
      setActionNode(null);
      setIsolateReason('');
      await loadData();
    } catch (err) {
      console.error('Network isolation operation failed', err);
    } finally {
      setActionLoading(false);
    }
  }

  function copyPayload(log: EventLogEntity) {
    navigator.clipboard.writeText(JSON.stringify(log.payload, null, 2));
    setCopiedId(log.event_id);
    setTimeout(() => setCopiedId(null), 2000);
  }

  useEffect(() => {
    document.title = 'Network';
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <div style={{ borderBottom: '1px solid var(--border-dark)', paddingBottom: '16px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <Network size={26} style={{ color: 'var(--fg)' }} />
          <h1
            style={{
              fontFamily: 'var(--font-display)',
              fontSize: '32px',
              fontWeight: 700,
              letterSpacing: '-0.5px',
              color: 'var(--fg)',
            }}
          >
            Network
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
          Socket connection telemetry, remote destination traffic, and endpoint network containment controls.
        </p>
      </div>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))',
          gap: '16px',
        }}
      >
        <MetricCard
          title="Network Events"
          value={logs.length}
          subtitle="Recent socket connections"
          icon={<Activity size={18} />}
          loading={loading}
        />
        <MetricCard
          title="Unique Remote IPs"
          value={uniqueRemoteIps}
          subtitle="Distinct external destinations"
          icon={<Globe size={18} />}
          loading={loading}
        />
        <MetricCard
          title="Isolated Hosts"
          value={isolatedCount}
          subtitle="Network quarantined nodes"
          subtitleColor={isolatedCount > 0 ? 'var(--color-danger)' : undefined}
          icon={<ShieldAlert size={18} />}
          loading={loading}
        />
        <MetricCard
          title="Monitored Endpoints"
          value={nodes.length}
          subtitle="Active fleet interfaces"
          icon={<Server size={18} />}
          loading={loading}
        />
      </div>

      <div style={{ display: 'flex', borderBottom: '2px solid var(--border-dark)', gap: '4px' }}>
        <button
          onClick={() => setActiveTab('telemetry')}
          style={{
            padding: '10px 18px',
            fontFamily: 'var(--font-mono)',
            fontSize: '12px',
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: '0.05em',
            background: activeTab === 'telemetry' ? 'var(--fg)' : 'transparent',
            color: activeTab === 'telemetry' ? 'var(--bg)' : 'var(--muted-fg)',
            border: 'none',
            cursor: 'pointer',
            transition: 'background 0.1s ease, color 0.1s ease',
          }}
        >
          Connection Telemetry ({logs.length})
        </button>
        <button
          onClick={() => setActiveTab('containment')}
          style={{
            padding: '10px 18px',
            fontFamily: 'var(--font-mono)',
            fontSize: '12px',
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: '0.05em',
            background: activeTab === 'containment' ? 'var(--fg)' : 'transparent',
            color: activeTab === 'containment' ? 'var(--bg)' : 'var(--muted-fg)',
            border: 'none',
            cursor: 'pointer',
            transition: 'background 0.1s ease, color 0.1s ease',
          }}
        >
          Host Containment ({nodes.length})
        </button>
      </div>

      {activeTab === 'telemetry' ? (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          <div
            className="card"
            style={{
              padding: '14px 18px',
              display: 'flex',
              gap: '12px',
              alignItems: 'center',
            }}
          >
            <div style={{ flex: 1, position: 'relative' }}>
              <Search
                size={14}
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
                placeholder="Filter by remote IP, local address, or host..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="input-text"
                style={{ paddingLeft: '34px' }}
              />
            </div>
            <button
              onClick={loadData}
              className="btn-secondary"
              disabled={loading}
              style={{ padding: '8px 14px' }}
            >
              <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
              <span>Refresh</span>
            </button>
          </div>

          <div className="table-container">
            <table className="data-table">
              <thead>
                <tr>
                  <th style={{ width: '32px' }}></th>
                  <th>TIMESTAMP (UTC)</th>
                  <th>HOST</th>
                  <th>REMOTE DESTINATION</th>
                  <th>LOCAL BIND</th>
                  <th>PROTOCOL</th>
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
                      }}
                    >
                      Loading network connection logs...
                    </td>
                  </tr>
                ) : filteredLogs.length === 0 ? (
                  <tr>
                    <td
                      colSpan={7}
                      style={{
                        textAlign: 'center',
                        padding: '36px',
                        fontFamily: 'var(--font-mono)',
                        color: 'var(--muted-fg)',
                      }}
                    >
                      No network connection telemetry found.
                    </td>
                  </tr>
                ) : (
                  filteredLogs.map((log) => {
                    const isExpanded = expandedId === log.event_id;
                    const p = (log.payload as Record<string, unknown>) || {};
                    const remoteAddr = p.remote_address ? `${p.remote_address}:${p.remote_port || ''}` : '-';
                    const localAddr = p.local_address ? `${p.local_address}:${p.local_port || ''}` : '-';
                    const protocol = String(p.protocol || 'TCP').toUpperCase();

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
                          <td style={{ fontFamily: 'var(--font-mono)', fontSize: '13px', fontWeight: 600 }}>
                            {remoteAddr}
                          </td>
                          <td style={{ fontFamily: 'var(--font-mono)', fontSize: '12px', color: 'var(--muted-fg)' }}>
                            {localAddr}
                          </td>
                          <td>
                            <span
                              style={{
                                fontFamily: 'var(--font-mono)',
                                fontSize: '11px',
                                padding: '2px 6px',
                                border: '1px solid var(--border-dark)',
                                background: 'var(--muted-bg)',
                              }}
                            >
                              {protocol}
                            </span>
                          </td>
                          <td style={{ textAlign: 'right', whiteSpace: 'nowrap' }}>
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                copyPayload(log);
                              }}
                              className="btn-secondary"
                              style={{ padding: '3px 8px', fontSize: '10px' }}
                            >
                              {copiedId === log.event_id ? (
                                <>
                                  <Check size={11} style={{ color: 'var(--color-success)' }} />
                                  <span>Copied</span>
                                </>
                              ) : (
                                <>
                                  <Copy size={11} />
                                  <span>Copy JSON</span>
                                </>
                              )}
                            </button>
                          </td>
                        </tr>
                        {isExpanded && (
                          <tr>
                            <td colSpan={7} style={{ padding: '0', background: 'var(--muted-bg)' }}>
                              <div style={{ padding: '16px 20px', borderTop: '1px solid var(--border-light)' }}>
                                <pre
                                  style={{
                                    margin: 0,
                                    padding: '12px',
                                    background: 'var(--bg)',
                                    border: '1px solid var(--border-dark)',
                                    fontFamily: 'var(--font-mono)',
                                    fontSize: '12px',
                                    lineHeight: '1.4',
                                    overflowX: 'auto',
                                    maxHeight: '280px',
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
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          <div className="table-container">
            <table className="data-table">
              <thead>
                <tr>
                  <th>HOSTNAME</th>
                  <th>MACHINE ID</th>
                  <th>OS VERSION</th>
                  <th>NETWORK STATUS</th>
                  <th style={{ textAlign: 'right' }}>QUARANTINE ACTION</th>
                </tr>
              </thead>
              <tbody>
                {nodes.map((node) => {
                  const isIsolated = node.operator_status === 'isolated';
                  return (
                    <tr key={node.node_id}>
                      <td>
                        <span style={{ fontWeight: 600, fontFamily: 'var(--font-mono)', fontSize: '13px' }}>
                          {node.hostname}
                        </span>
                        <div style={{ fontSize: '11px', fontFamily: 'var(--font-mono)', color: 'var(--muted-fg)' }}>
                          {node.node_id}
                        </div>
                      </td>
                      <td style={{ fontFamily: 'var(--font-mono)', fontSize: '12px', color: 'var(--muted-fg)' }}>
                        {node.machine_id}
                      </td>
                      <td style={{ fontFamily: 'var(--font-mono)', fontSize: '12px', color: 'var(--muted-fg)' }}>
                        {node.os_version}
                      </td>
                      <td>
                        <StatusBadge type="operator_status" value={node.operator_status} />
                      </td>
                      <td style={{ textAlign: 'right' }}>
                        {isIsolated ? (
                          <button
                            onClick={() => {
                              setActionNode(node);
                              setActionType('unisolate');
                            }}
                            className="btn-secondary"
                            style={{ padding: '4px 10px', fontSize: '11px' }}
                          >
                            <ShieldCheck size={12} style={{ color: 'var(--color-success)' }} />
                            <span>Restore Access</span>
                          </button>
                        ) : (
                          <button
                            onClick={() => {
                              setActionNode(node);
                              setActionType('isolate');
                            }}
                            className="btn-danger"
                            style={{ padding: '4px 10px', fontSize: '11px' }}
                          >
                            <ShieldAlert size={12} />
                            <span>Isolate Host</span>
                          </button>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal
        isOpen={!!actionNode}
        onClose={() => setActionNode(null)}
        title={actionType === 'isolate' ? 'Isolate Host from Network' : 'Restore Host Network Access'}
      >
        {actionNode && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
            <p style={{ fontSize: '13px', fontFamily: 'var(--font-mono)', color: 'var(--muted-fg)' }}>
              {actionType === 'isolate'
                ? `Quarantine ${actionNode.hostname} using nftables rules. All external traffic will be dropped except control plane telemetry.`
                : `Remove network quarantine rules for ${actionNode.hostname}. External connectivity will be restored.`}
            </p>

            <form onSubmit={handleIsolationSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
              {actionType === 'isolate' && (
                <div>
                  <label
                    htmlFor="network-isolate-reason"
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontSize: '11px',
                      fontWeight: 600,
                      textTransform: 'uppercase',
                      display: 'block',
                      marginBottom: '6px',
                    }}
                  >
                    Reason
                  </label>
                  <input
                    id="network-isolate-reason"
                    type="text"
                    required
                    placeholder="Suspicious socket connection / lateral movement..."
                    value={isolateReason}
                    onChange={(e) => setIsolateReason(e.target.value)}
                    className="input-text"
                  />
                </div>
              )}

              <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px', marginTop: '8px' }}>
                <button
                  type="button"
                  onClick={() => setActionNode(null)}
                  className="btn-secondary"
                  disabled={actionLoading}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className={actionType === 'isolate' ? 'btn-danger' : 'btn-primary'}
                  disabled={actionLoading}
                >
                  {actionLoading
                    ? 'Processing...'
                    : actionType === 'isolate'
                    ? 'Confirm Quarantine'
                    : 'Confirm Restore'}
                </button>
              </div>
            </form>
          </div>
        )}
      </Modal>
    </div>
  );
}
