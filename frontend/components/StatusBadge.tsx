'use client';

import React from 'react';

interface StatusBadgeProps {
  type: 'severity' | 'agent_status' | 'operator_status' | 'alert_status';
  value: string;
}

export function StatusBadge({ type, value }: StatusBadgeProps) {
  const norm = (value || '').toLowerCase();

  let bg = '#FFFFFF';
  let color = '#000000';
  let border = 'var(--border-dark)';
  let dotColor: string | null = null;

  if (type === 'severity') {
    switch (norm) {
      case 'critical':
        bg = '#000000';
        color = '#FFFFFF';
        border = '#000000';
        dotColor = '#ef4444';
        break;
      case 'high':
        bg = '#FFFFFF';
        color = '#b91c1c';
        border = '#b91c1c';
        dotColor = '#b91c1c';
        break;
      case 'medium':
        bg = '#FFFFFF';
        color = '#c2410c';
        border = '#c2410c';
        dotColor = '#c2410c';
        break;
      case 'low':
      default:
        bg = '#FFFFFF';
        color = 'var(--muted-fg)';
        border = 'var(--border-light)';
        break;
    }
  } else if (type === 'agent_status') {
    switch (norm) {
      case 'online':
      case 'healthy':
        bg = '#FFFFFF';
        color = '#15803d';
        border = '#15803d';
        dotColor = '#16a34a';
        break;
      case 'degraded':
        bg = '#FFFFFF';
        color = '#b45309';
        border = '#b45309';
        dotColor = '#d97706';
        break;
      case 'offline':
      default:
        bg = '#FFFFFF';
        color = '#b91c1c';
        border = '#b91c1c';
        dotColor = '#dc2626';
        break;
    }
  } else if (type === 'operator_status') {
    switch (norm) {
      case 'isolated':
        bg = '#000000';
        color = '#FFFFFF';
        border = '#000000';
        dotColor = '#ef4444';
        break;
      case 'active':
      case 'normal':
      default:
        bg = '#FFFFFF';
        color = '#000000';
        border = 'var(--border-dark)';
        dotColor = '#16a34a';
        break;
    }
  } else if (type === 'alert_status') {
    switch (norm) {
      case 'open':
      case 'active':
        bg = '#FFFFFF';
        color = '#b91c1c';
        border = '#b91c1c';
        dotColor = '#dc2626';
        break;
      case 'investigating':
      case 'acknowledged':
        bg = '#FFFFFF';
        color = '#c2410c';
        border = '#c2410c';
        dotColor = '#ea580c';
        break;
      case 'resolved':
      case 'dismissed':
      case 'false_positive':
        bg = '#FFFFFF';
        color = '#15803d';
        border = '#15803d';
        dotColor = '#16a34a';
        break;
      default:
        bg = '#FFFFFF';
        color = 'var(--muted-fg)';
        border = 'var(--border-light)';
        break;
    }
  }

  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '6px',
        padding: '2px 8px',
        fontSize: '11px',
        fontFamily: 'var(--font-mono)',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: '0.05em',
        backgroundColor: bg,
        color: color,
        border: `1px solid ${border}`,
        lineHeight: '16px',
        whiteSpace: 'nowrap',
      }}
    >
      {dotColor && (
        <span
          className="status-dot"
          style={{
            backgroundColor: dotColor,
            width: '5px',
            height: '5px',
          }}
        />
      )}
      <span>{value.replace('_', ' ')}</span>
    </span>
  );
}
