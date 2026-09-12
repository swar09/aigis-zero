'use client';

import React from 'react';

interface MetricCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  subtitleColor?: string;
  icon?: React.ReactNode;
  loading?: boolean;
}

export function MetricCard({
  title,
  value,
  subtitle,
  subtitleColor = 'var(--color-text-muted)',
  icon,
  loading = false,
}: MetricCardProps) {
  return (
    <div
      className="card"
      style={{
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        minHeight: '130px',
        padding: '20px 24px',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span
          style={{
            fontFamily: 'var(--font-mono)',
            fontSize: '11px',
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: '0.06em',
            color: 'var(--color-text-muted)',
          }}
        >
          {title}
        </span>
        {icon && (
          <div style={{ color: 'var(--fg)', display: 'flex', alignItems: 'center' }}>
            {icon}
          </div>
        )}
      </div>

      <div>
        <div
          style={{
            fontFamily: 'var(--font-mono)',
            fontSize: '28px',
            fontWeight: 700,
            color: 'var(--color-text-primary)',
            marginTop: '8px',
            lineHeight: '32px',
          }}
        >
          {loading ? '...' : value}
        </div>
        {subtitle && (
          <div
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: '11px',
              color: subtitleColor,
              marginTop: '6px',
              fontWeight: 500,
              letterSpacing: '0.02em',
            }}
          >
            {subtitle}
          </div>
        )}
      </div>
    </div>
  );
}
