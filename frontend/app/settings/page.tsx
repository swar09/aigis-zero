'use client';

import React, { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import {
  Check,
  CheckCircle2,
  Copy,
  LogOut,
  Settings,
  XCircle,
} from 'lucide-react';
import { checkHealth, clearAuthSession, getStoredUser } from '../../lib/api';
import { UserInfo } from '../../lib/types';

export default function SettingsPage() {
  const router = useRouter();
  const [healthStatus, setHealthStatus] = useState<string>('checking');
  const [operator, setOperator] = useState<UserInfo | null>(null);
  const [copiedEnroll, setCopiedEnroll] = useState(false);

  useEffect(() => {
    document.title = 'Settings';
    setOperator(getStoredUser());

    checkHealth()
      .then((res) => setHealthStatus(res.status || 'healthy'))
      .catch(() => setHealthStatus('unreachable'));
  }, []);

  function handleLogout() {
    clearAuthSession();
    router.push('/login');
  }

  const enrollScript = `# Enrollment snippet for Linux endpoints
curl -sSL https://raw.githubusercontent.com/aigis-zero/install/main/enroll.sh | \\
  sudo bash -s -- \\
  --fleet-server="http://127.0.0.1:50051" \\
  --secret="aigis_node_enrollment_pre_shared_key_change_me"`;

  function copyEnrollSnippet() {
    navigator.clipboard.writeText(enrollScript);
    setCopiedEnroll(true);
    setTimeout(() => setCopiedEnroll(false), 2000);
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '28px', maxWidth: '920px' }}>
      <div style={{ borderBottom: '1px solid var(--border-dark)', paddingBottom: '20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <Settings size={26} style={{ color: 'var(--fg)' }} />
          <h1
            style={{
              fontFamily: 'var(--font-display)',
              fontSize: '32px',
              fontWeight: 700,
              letterSpacing: '-0.5px',
              color: 'var(--fg)',
            }}
          >
            Settings
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
          Environment configuration, service connectivity, and active session details.
        </p>
      </div>

      <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <h2
          style={{
            fontFamily: 'var(--font-display)',
            fontSize: '18px',
            fontWeight: 700,
            color: 'var(--fg)',
            borderBottom: '1px solid var(--border-light)',
            paddingBottom: '10px',
          }}
        >
          Backend Service Status
        </h2>

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: '14px' }}>
          <div
            style={{
              padding: '16px',
              background: 'var(--bg)',
              border: '1px solid var(--border-dark)',
            }}
          >
            <div
              style={{
                fontSize: '11px',
                fontFamily: 'var(--font-mono)',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--muted-fg)',
              }}
            >
              REST API Gateway
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '8px' }}>
              {healthStatus === 'healthy' ? (
                <CheckCircle2 size={16} style={{ color: 'var(--color-success)' }} />
              ) : (
                <XCircle size={16} style={{ color: 'var(--color-danger)' }} />
              )}
              <span style={{ fontSize: '13px', fontFamily: 'var(--font-mono)', fontWeight: 600, textTransform: 'uppercase' }}>
                {healthStatus} (Port 8088)
              </span>
            </div>
          </div>

          <div
            style={{
              padding: '16px',
              background: 'var(--bg)',
              border: '1px solid var(--border-dark)',
            }}
          >
            <div
              style={{
                fontSize: '11px',
                fontFamily: 'var(--font-mono)',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--muted-fg)',
              }}
            >
              Fleet Server gRPC
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '8px' }}>
              <CheckCircle2 size={16} style={{ color: 'var(--color-success)' }} />
              <span style={{ fontSize: '13px', fontFamily: 'var(--font-mono)', fontWeight: 600 }}>
                LISTENING (Port 50051)
              </span>
            </div>
          </div>

          <div
            style={{
              padding: '16px',
              background: 'var(--bg)',
              border: '1px solid var(--border-dark)',
            }}
          >
            <div
              style={{
                fontSize: '11px',
                fontFamily: 'var(--font-mono)',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--muted-fg)',
              }}
            >
              Kafka Pipeline
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '8px' }}>
              <CheckCircle2 size={16} style={{ color: 'var(--color-success)' }} />
              <span style={{ fontSize: '13px', fontFamily: 'var(--font-mono)', fontWeight: 600 }}>
                ACTIVE (Port 9092)
              </span>
            </div>
          </div>
        </div>
      </div>

      <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h2
              style={{
                fontFamily: 'var(--font-display)',
                fontSize: '18px',
                fontWeight: 700,
                color: 'var(--fg)',
              }}
            >
              Agent Enrollment Command
            </h2>
            <p
              style={{
                fontSize: '12px',
                fontFamily: 'var(--font-mono)',
                color: 'var(--muted-fg)',
                marginTop: '3px',
              }}
            >
              Bootstrap an endpoint agent to connect to this fleet controller
            </p>
          </div>
          <button onClick={copyEnrollSnippet} className="btn-secondary" style={{ padding: '6px 12px' }}>
            {copiedEnroll ? <Check size={13} color="var(--color-success)" /> : <Copy size={13} />}
            <span>{copiedEnroll ? 'Copied' : 'Copy Script'}</span>
          </button>
        </div>

        <pre
          style={{
            margin: 0,
            padding: '16px',
            background: 'var(--muted-bg)',
            border: '1px solid var(--border-dark)',
            fontSize: '12px',
            fontFamily: 'var(--font-mono)',
            overflowX: 'auto',
            lineHeight: 1.6,
          }}
        >
          {enrollScript}
        </pre>
      </div>

      <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <h2
          style={{
            fontFamily: 'var(--font-display)',
            fontSize: '18px',
            fontWeight: 700,
            color: 'var(--fg)',
            borderBottom: '1px solid var(--border-light)',
            paddingBottom: '10px',
          }}
        >
          Account Session
        </h2>

        <div style={{ display: 'grid', gridTemplateColumns: '130px 1fr', gap: '12px', fontSize: '13px' }}>
          <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Username:</span>
          <span style={{ fontWeight: 600, fontFamily: 'var(--font-mono)' }}>{operator?.username || 'admin'}</span>

          <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Role:</span>
          <span style={{ fontFamily: 'var(--font-mono)' }}>{operator?.role || 'soc_admin'}</span>

          <span style={{ color: 'var(--muted-fg)', fontFamily: 'var(--font-mono)' }}>Permissions:</span>
          <span style={{ fontFamily: 'var(--font-mono)' }}>
            {(operator?.permissions || ['nodes:read', 'nodes:isolate', 'alerts:read', 'alerts:write', 'logs:read']).join(', ')}
          </span>
        </div>

        <div style={{ paddingTop: '8px' }}>
          <button
            onClick={handleLogout}
            className="btn-secondary"
            style={{ borderColor: 'var(--color-danger)', color: 'var(--color-danger)' }}
          >
            <LogOut size={14} />
            <span>Sign Out Session</span>
          </button>
        </div>
      </div>
    </div>
  );
}
