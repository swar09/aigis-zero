'use client';

import React, { useState, useEffect, useCallback } from 'react';
import {
  AlertTriangle,
  RefreshCw,
  Search,
  Server,
  ShieldAlert,
  ShieldCheck,
  X,
} from 'lucide-react';
import { getNodeById, getNodes, isolateNode, unisolateNode } from '../../lib/api';
import { NodeDetailDto, NodeSummaryDto } from '../../lib/types';
import { StatusBadge } from '../../components/StatusBadge';
import { Drawer } from '../../components/ui/Drawer';
import { Modal } from '../../components/ui/Modal';

export default function EndpointsPage() {
  const [nodes, setNodes] = useState<NodeSummaryDto[]>([]);
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState('all');
  const [loading, setLoading] = useState(true);

  // Detail drawer state
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [detailData, setDetailData] = useState<NodeDetailDto | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  // Isolate dialog state
  const [isolatingNode, setIsolatingNode] = useState<NodeSummaryDto | null>(null);
  const [isolateReason, setIsolateReason] = useState('');
  const [isolateSubmitting, setIsolateSubmitting] = useState(false);

  const fetchNodes = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getNodes({
        search: searchTerm || undefined,
        agent_status: statusFilter !== 'all' && statusFilter !== 'isolated' ? statusFilter : undefined,
        operator_status: statusFilter === 'isolated' ? 'isolated' : undefined,
        limit: 100,
      });
      setNodes(data);
    } catch (err) {
      console.error('Failed to fetch endpoints', err);
    } finally {
      setLoading(false);
    }
  }, [searchTerm, statusFilter]);

  useEffect(() => {
    fetchNodes();
  }, [fetchNodes]);

  async function openDetailDrawer(nodeId: string) {
    setSelectedNodeId(nodeId);
    setDetailLoading(true);
    try {
      const detail = await getNodeById(nodeId);
      setDetailData(detail);
    } catch (err) {
      console.error('Failed to fetch node detail', err);
    } finally {
      setDetailLoading(false);
    }
  }

  function closeDetailDrawer() {
    setSelectedNodeId(null);
    setDetailData(null);
  }

  async function handleConfirmIsolate() {
    if (!isolatingNode) return;
    setIsolateSubmitting(true);
    try {
      await isolateNode(isolatingNode.node_id, isolateReason || 'Quarantined by security operator');
      setIsolatingNode(null);
      setIsolateReason('');
      await fetchNodes();
      if (selectedNodeId === isolatingNode.node_id) {
        await openDetailDrawer(isolatingNode.node_id);
      }
    } catch (err) {
      console.error('Failed to isolate endpoint', err);
    } finally {
      setIsolateSubmitting(false);
    }
  }

  async function handleUnisolate(node: NodeSummaryDto) {
    try {
      await unisolateNode(node.node_id);
      await fetchNodes();
      if (selectedNodeId === node.node_id) {
        await openDetailDrawer(node.node_id);
      }
    } catch (err) {
      console.error('Failed to restore endpoint', err);
    }
  }

  useEffect(() => {
    document.title = 'Endpoints';
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
            <Server size={26} style={{ color: 'var(--fg)' }} />
            <h1
              style={{
                fontFamily: 'var(--font-display)',
                fontSize: '32px',
                fontWeight: 700,
                letterSpacing: '-0.5px',
                color: 'var(--fg)',
              }}
            >
              Endpoints
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
            Host fleet telemetry with agent heartbeats, operating systems, and host quarantine controls.
          </p>
        </div>
        <button
          onClick={fetchNodes}
          className="btn-secondary"
          style={{ padding: '8px 14px' }}
        >
          <RefreshCw size={13} />
          <span>Refresh</span>
        </button>
      </div>

      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          gap: '16px',
          flexWrap: 'wrap',
        }}
      >
        <div style={{ position: 'relative', width: '360px' }}>
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
            placeholder="Search by hostname or machine ID..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="input-text"
            style={{ paddingLeft: '36px' }}
          />
        </div>

        <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
          {['all', 'healthy', 'degraded', 'offline', 'isolated'].map((st) => {
            const isSelected = statusFilter === st;
            return (
              <button
                key={st}
                onClick={() => setStatusFilter(st)}
                style={{
                  padding: '6px 12px',
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

      <div className="table-container">
        <table className="data-table">
          <thead>
            <tr>
              <th>HOSTNAME</th>
              <th>MACHINE ID</th>
              <th>OS VERSION</th>
              <th>AGENT VERSION</th>
              <th>AGENT STATUS</th>
              <th>QUARANTINE</th>
              <th style={{ textAlign: 'right' }}>ACTIONS</th>
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
                  Loading fleet endpoints...
                </td>
              </tr>
            ) : nodes.length === 0 ? (
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
                  No endpoints matching current filter. Check enrollment status.
                </td>
              </tr>
            ) : (
              nodes.map((node) => {
                const isIsolated = node.operator_status === 'isolated';

                return (
                  <tr key={node.node_id}>
                    <td>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <Server size={15} style={{ color: 'var(--muted-fg)' }} />
                        <span
                          style={{
                            fontWeight: 600,
                            color: 'var(--fg)',
                            cursor: 'pointer',
                            textDecoration: 'underline',
                          }}
                          onClick={() => openDetailDrawer(node.node_id)}
                        >
                          {node.hostname}
                        </span>
                      </div>
                    </td>
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: '12px', color: 'var(--muted-fg)' }}>
                      {node.machine_id}
                    </td>
                    <td style={{ fontSize: '13px' }}>{node.os_version}</td>
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
                      {node.agent_version}
                    </td>
                    <td>
                      <StatusBadge type="agent_status" value={node.agent_status} />
                    </td>
                    <td>
                      <StatusBadge type="operator_status" value={node.operator_status} />
                    </td>
                    <td style={{ textAlign: 'right' }}>
                      <div style={{ display: 'inline-flex', gap: '8px' }}>
                        <button
                          onClick={() => openDetailDrawer(node.node_id)}
                          className="btn-secondary"
                          style={{ padding: '4px 10px', fontSize: '11px' }}
                        >
                          Details
                        </button>
                        {isIsolated ? (
                          <button
                            onClick={() => handleUnisolate(node)}
                            className="btn-secondary"
                            style={{
                              padding: '4px 10px',
                              fontSize: '11px',
                              borderColor: 'var(--color-success)',
                              color: 'var(--color-success)',
                            }}
                          >
                            <ShieldCheck size={13} />
                            Restore
                          </button>
                        ) : (
                          <button
                            onClick={() => setIsolatingNode(node)}
                            className="btn-danger"
                            style={{ padding: '4px 10px', fontSize: '11px' }}
                          >
                            <ShieldAlert size={13} />
                            Isolate
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>

      {/* Slide-over Drawer for Node Details */}
      <Drawer
        isOpen={!!selectedNodeId}
        onClose={closeDetailDrawer}
        title="Endpoint Specification"
        subtitle={detailData?.node.hostname ? `Host: ${detailData.node.hostname} (${detailData.node.node_id})` : undefined}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
          {detailLoading || !detailData ? (
                <div
                  style={{
                    textAlign: 'center',
                    padding: '40px',
                    fontFamily: 'var(--font-mono)',
                    color: 'var(--muted-fg)',
                    fontSize: '13px',
                  }}
                >
                  Loading endpoint metadata...
                </div>
              ) : (
                <>
                  <div className="card" style={{ padding: '18px' }}>
                    <h3
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
                      Identity &amp; Telemetry
                    </h3>
                    <div style={{ display: 'grid', gridTemplateColumns: '130px 1fr', gap: '10px', fontSize: '13px' }}>
                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Hostname:</span>
                      <span style={{ fontWeight: 600, fontFamily: 'var(--font-mono)' }}>{detailData.node.hostname}</span>

                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Node ID:</span>
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>{detailData.node.node_id}</span>

                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Machine ID:</span>
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>{detailData.node.machine_id}</span>

                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>OS Version:</span>
                      <span>{detailData.node.os_version}</span>

                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Agent Version:</span>
                      <span style={{ fontFamily: 'var(--font-mono)' }}>{detailData.node.agent_version}</span>

                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Agent Status:</span>
                      <span>
                        <StatusBadge type="agent_status" value={detailData.node.agent_status} />
                      </span>

                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Quarantine:</span>
                      <span>
                        <StatusBadge type="operator_status" value={detailData.node.operator_status} />
                      </span>

                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>First Seen:</span>
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>{detailData.node.first_seen_at}</span>

                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Last Enrolled:</span>
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>{detailData.node.last_enrolled_at}</span>
                    </div>
                  </div>

                  <div className="card" style={{ padding: '18px' }}>
                    <h3
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
                      Recent Health Records
                    </h3>
                    {(!detailData.health_history || detailData.health_history.length === 0) ? (
                      <p style={{ fontSize: '13px', fontFamily: 'var(--font-mono)', color: 'var(--muted-fg)' }}>
                        No recent health records logged for this node.
                      </p>
                    ) : (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                        {detailData.health_history.slice(0, 5).map((h, i) => (
                          <div
                            key={h.recorded_at ? `${h.recorded_at}-${i}` : i}
                            style={{
                              padding: '10px 12px',
                              background: 'var(--bg)',
                              border: '1px solid var(--border-dark)',
                              fontSize: '12px',
                            }}
                          >
                            <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>
                              <span>{h.recorded_at}</span>
                              <span style={{ fontWeight: 600, color: 'var(--color-success)' }}>{h.agent_status}</span>
                            </div>
                            <div
                              style={{
                                marginTop: '4px',
                                display: 'flex',
                                gap: '14px',
                                fontSize: '11px',
                                fontFamily: 'var(--font-mono)',
                                color: 'var(--muted-fg)',
                              }}
                            >
                              <span>Events: {h.events_buffered ?? 0}</span>
                              <span>CPU: {h.process_cpu_percent ?? 0}%</span>
                              <span>RAM: {((h.process_memory_bytes ?? 0) / (1024 * 1024)).toFixed(1)} MB</span>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </>
              )}
            </div>
          </Drawer>

      {/* Network Isolation Modal */}
      <Modal
        isOpen={!!isolatingNode}
        onClose={() => setIsolatingNode(null)}
        title="Isolate Host Network"
      >
        {isolatingNode && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <p style={{ fontSize: '14px', color: 'var(--muted-fg)', lineHeight: 1.5 }}>
              You are about to isolate <strong>{isolatingNode.hostname}</strong>. This command configures host nftables rules to block all network traffic except management communications to the fleet server.
            </p>

            <div>
              <label
                htmlFor="quarantine-justification"
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: '11px',
                  fontWeight: 600,
                  textTransform: 'uppercase',
                  display: 'block',
                  marginBottom: '6px',
                }}
              >
                Quarantine Justification:
              </label>
              <textarea
                id="quarantine-justification"
                value={isolateReason}
                onChange={(e) => setIsolateReason(e.target.value)}
                placeholder="Describe suspected threat activity or incident ticket reference..."
                rows={3}
                className="input-text"
                style={{ resize: 'vertical' }}
              />
            </div>

            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px', marginTop: '4px' }}>
              <button
                type="button"
                onClick={() => setIsolatingNode(null)}
                className="btn-secondary"
                disabled={isolateSubmitting}
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleConfirmIsolate}
                className="btn-danger"
                disabled={isolateSubmitting}
              >
                {isolateSubmitting ? 'Isolating...' : 'Confirm Quarantine'}
              </button>
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
