'use client';

import React, { useEffect, useState } from 'react';
import Link from 'next/link';
import {
  Activity,
  AlertTriangle,
  ArrowRight,
  LayoutDashboard,
  Radio,
  Server,
  Shield,
  Zap,
} from 'lucide-react';
import { getAlerts, getNodes } from '../lib/api';
import { AlertEntity, NodeSummaryDto } from '../lib/types';
import { MetricCard } from '../components/MetricCard';
import { StatusBadge } from '../components/StatusBadge';
import { useLiveEvents } from '../hooks/useLiveEvents';

export default function OverviewPage() {
  const [nodes, setNodes] = useState<NodeSummaryDto[]>([]);
  const [alerts, setAlerts] = useState<AlertEntity[]>([]);
  const [loading, setLoading] = useState(true);

  const { events, connectionState, totalEventCount, eventsPerSecond } = useLiveEvents({
    maxEvents: 10,
  });

  useEffect(() => {
    async function loadData() {
      setLoading(true);
      try {
        const [nodesData, alertsData] = await Promise.all([
          getNodes({ limit: 100 }),
          getAlerts({ limit: 100 }),
        ]);
        setNodes(nodesData);
        setAlerts(alertsData);
      } catch (err) {
        console.error('Failed to load overview data', err);
      } finally {
        setLoading(false);
      }
    }
    loadData();
  }, []);

  const totalNodes = nodes.length;
  const onlineNodes = nodes.filter(
    (n) => n.agent_status === 'healthy' || n.agent_status === 'online'
  ).length;
  const isolatedNodes = nodes.filter((n) => n.operator_status === 'isolated').length;
  const criticalAlerts = alerts.filter(
    (a) => a.severity.toLowerCase() === 'critical' && a.status !== 'resolved' && a.status !== 'dismissed'
  ).length;
  const highAlerts = alerts.filter(
    (a) => a.severity.toLowerCase() === 'high' && a.status !== 'resolved' && a.status !== 'dismissed'
  ).length;
  const activeAlerts = alerts.filter(
    (a) => a.status !== 'resolved' && a.status !== 'dismissed'
  ).length;

  const recentAlerts = alerts.slice(0, 6);

  useEffect(() => {
    document.title = 'Dashboard';
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '32px' }}>
      <div style={{ borderBottom: '1px solid var(--border-dark)', paddingBottom: '20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <LayoutDashboard size={26} style={{ color: 'var(--fg)' }} />
          <h1
            style={{
              fontFamily: 'var(--font-display)',
              fontSize: '32px',
              fontWeight: 700,
              letterSpacing: '-0.5px',
              color: 'var(--fg)',
            }}
          >
            Dashboard
          </h1>
        </div>
        <p
          style={{
            fontSize: '14px',
            color: 'var(--muted-fg)',
            marginTop: '6px',
            fontFamily: 'var(--font-mono)',
          }}
        >
          Fleet telemetry, active detection metrics, and infrastructure health.
        </p>
      </div>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))',
          gap: '20px',
        }}
      >
        <MetricCard
          title="Total Endpoints"
          value={totalNodes}
          subtitle={`${onlineNodes} online / ${isolatedNodes} isolated`}
          subtitleColor="var(--color-success)"
          icon={<Server size={18} />}
          loading={loading}
        />
        <MetricCard
          title="Active Alerts"
          value={activeAlerts}
          subtitle={`${criticalAlerts} critical / ${highAlerts} high`}
          subtitleColor={criticalAlerts > 0 ? 'var(--color-danger)' : 'var(--muted-fg)'}
          icon={<AlertTriangle size={18} />}
          loading={loading}
        />
        <MetricCard
          title="Telemetry Throughput"
          value={`${eventsPerSecond} /sec`}
          subtitle={`Ingested: ${totalEventCount.toLocaleString()} events`}
          subtitleColor="var(--fg)"
          icon={<Activity size={18} />}
        />
        <MetricCard
          title="WebSocket Channel"
          value={connectionState.toUpperCase()}
          subtitle={`Status: ${connectionState === 'connected' ? 'Live Telemetry' : 'Reconnecting'}`}
          subtitleColor={connectionState === 'connected' ? 'var(--color-success)' : 'var(--color-warning)'}
          icon={<Radio size={18} />}
        />
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: '28px' }}>
        <div className="card">
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginBottom: '20px',
              borderBottom: '1px solid var(--border-light)',
              paddingBottom: '14px',
            }}
          >
            <div>
              <h2
                style={{
                  fontFamily: 'var(--font-display)',
                  fontSize: '20px',
                  fontWeight: 700,
                  color: 'var(--fg)',
                }}
              >
                Recent Detections
              </h2>
              <p
                style={{
                  fontSize: '12px',
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--muted-fg)',
                  marginTop: '2px',
                }}
              >
                Security alerts requiring review
              </p>
            </div>
            <Link
              href="/alerts"
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: '11px',
                fontWeight: 600,
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--fg)',
                display: 'inline-flex',
                alignItems: 'center',
                gap: '6px',
                borderBottom: '1px solid var(--border-dark)',
                paddingBottom: '2px',
              }}
            >
              View all alerts <ArrowRight size={13} />
            </Link>
          </div>

          <div className="table-container" style={{ border: 'none' }}>
            <table className="data-table">
              <thead>
                <tr>
                  <th>SEVERITY</th>
                  <th>DESCRIPTION</th>
                  <th>HOST</th>
                  <th>TECHNIQUE</th>
                  <th>STATUS</th>
                </tr>
              </thead>
              <tbody>
                {loading ? (
                  <tr>
                    <td
                      colSpan={5}
                      style={{
                        textAlign: 'center',
                        padding: '32px',
                        fontFamily: 'var(--font-mono)',
                        color: 'var(--muted-fg)',
                        fontSize: '13px',
                      }}
                    >
                      Loading telemetry detections...
                    </td>
                  </tr>
                ) : recentAlerts.length === 0 ? (
                  <tr>
                    <td
                      colSpan={5}
                      style={{
                        textAlign: 'center',
                        padding: '40px',
                        fontFamily: 'var(--font-mono)',
                        color: 'var(--muted-fg)',
                        fontSize: '13px',
                      }}
                    >
                      No active alerts recorded. System clean.
                    </td>
                  </tr>
                ) : (
                  recentAlerts.map((alert) => (
                    <tr key={alert.alert_id}>
                      <td>
                        <StatusBadge type="severity" value={alert.severity} />
                      </td>
                      <td
                        style={{
                          maxWidth: '300px',
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                          fontWeight: 500,
                        }}
                      >
                        {alert.description}
                      </td>
                      <td style={{ fontFamily: 'var(--font-mono)', fontSize: '13px', color: 'var(--muted-fg)' }}>
                        {alert.hostname}
                      </td>
                      <td style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
                        {alert.mitre_technique_id || 'N/A'}
                      </td>
                      <td>
                        <StatusBadge type="alert_status" value={alert.status} />
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
          <div className="card">
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                marginBottom: '12px',
                borderBottom: '1px solid var(--border-light)',
                paddingBottom: '12px',
              }}
            >
              <Zap size={16} style={{ color: 'var(--fg)' }} />
              <h2
                style={{
                  fontFamily: 'var(--font-display)',
                  fontSize: '18px',
                  fontWeight: 700,
                  color: 'var(--fg)',
                }}
              >
                Live Event Feed
              </h2>
            </div>
            <p
              style={{
                fontSize: '11px',
                fontFamily: 'var(--font-mono)',
                color: 'var(--muted-fg)',
                marginBottom: '16px',
                textTransform: 'uppercase',
                letterSpacing: '0.04em',
              }}
            >
              WebSocket stream
            </p>

            {events.length === 0 ? (
              <div
                style={{
                  padding: '32px 16px',
                  textAlign: 'center',
                  color: 'var(--muted-fg)',
                  fontFamily: 'var(--font-mono)',
                  fontSize: '12px',
                  background: 'var(--muted-bg)',
                  border: '1px solid var(--border-dark)',
                }}
              >
                Waiting for incoming events...
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                {events.map((evt, idx) => (
                  <div
                    key={`${evt.type}-${'id' in evt.data ? (evt.data as { id: string }).id : evt.data.timestamp_ns}-${idx}`}
                    style={{
                      padding: '10px 12px',
                      background: 'var(--bg)',
                      border: '1px solid var(--border-dark)',
                      fontSize: '12px',
                    }}
                  >
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                      <span
                        style={{
                          fontWeight: 700,
                          fontFamily: 'var(--font-mono)',
                          textTransform: 'uppercase',
                          color: 'var(--fg)',
                          letterSpacing: '0.05em',
                          fontSize: '11px',
                        }}
                      >
                        {evt.type}
                      </span>
                      <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)', fontSize: '11px' }}>
                        {'hostname' in evt.data ? evt.data.hostname : evt.data.node_id.slice(0, 8)}
                      </span>
                    </div>
                    <div
                      style={{
                        marginTop: '4px',
                        fontFamily: 'var(--font-mono)',
                        fontSize: '12px',
                        color: 'var(--fg)',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {evt.type === 'log'
                        ? `${evt.data.event_type}: ${JSON.stringify(evt.data.payload).slice(0, 48)}...`
                        : evt.type === 'alert'
                        ? evt.data.description
                        : `Heartbeat: status=${evt.data.agent_status}`}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="card">
            <h2
              style={{
                fontFamily: 'var(--font-display)',
                fontSize: '18px',
                fontWeight: 700,
                color: 'var(--fg)',
                marginBottom: '16px',
                borderBottom: '1px solid var(--border-light)',
                paddingBottom: '10px',
              }}
            >
              Control Actions
            </h2>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
              <Link
                href="/endpoints"
                className="btn-secondary"
                style={{ justifyContent: 'flex-start', padding: '10px 14px' }}
              >
                <Server size={15} />
                <span>Fleet Endpoints Inventory</span>
              </Link>
              <Link
                href="/hunt"
                className="btn-secondary"
                style={{ justifyContent: 'flex-start', padding: '10px 14px' }}
              >
                <Activity size={15} />
                <span>Threat Hunt Telemetry</span>
              </Link>
              <Link
                href="/settings"
                className="btn-secondary"
                style={{ justifyContent: 'flex-start', padding: '10px 14px' }}
              >
                <Shield size={15} />
                <span>System and Connectivity Settings</span>
              </Link>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
