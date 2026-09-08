'use client';

import React, { useEffect } from 'react';
import { AlertOctagon, RefreshCw } from 'lucide-react';

export default function ErrorBoundary({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error('Unhandled runtime error in console layout:', error);
  }, [error]);

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '50vh',
        padding: '40px 20px',
        textAlign: 'center',
      }}
    >
      <div
        className="card"
        style={{
          maxWidth: '560px',
          width: '100%',
          padding: '36px 32px',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: '20px',
          border: '2px solid var(--border-dark)',
          boxShadow: '8px 8px 0px 0px rgba(0, 0, 0, 1)',
        }}
      >
        <div
          style={{
            width: '52px',
            height: '52px',
            border: '2px solid var(--color-danger)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: 'var(--color-danger)',
          }}
        >
          <AlertOctagon size={28} />
        </div>

        <div>
          <h1
            style={{
              fontFamily: 'var(--font-display)',
              fontSize: '24px',
              fontWeight: 700,
              color: 'var(--fg)',
            }}
          >
            Runtime Component Error
          </h1>
          <p
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: '12px',
              color: 'var(--muted-fg)',
              marginTop: '6px',
            }}
          >
            An unexpected error occurred while rendering this view.
          </p>
        </div>

        <div
          style={{
            width: '100%',
            background: 'var(--muted-bg)',
            border: '1px solid var(--border-light)',
            padding: '12px 14px',
            fontFamily: 'var(--font-mono)',
            fontSize: '12px',
            textAlign: 'left',
            overflowX: 'auto',
            color: 'var(--fg)',
          }}
        >
          {error.message || 'Unknown application exception'}
        </div>

        <div style={{ display: 'flex', gap: '12px', width: '100%', marginTop: '8px' }}>
          <button
            type="button"
            onClick={() => reset()}
            className="btn-primary"
            style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}
          >
            <RefreshCw size={14} />
            <span>Try Again</span>
          </button>
          <button
            type="button"
            onClick={() => { window.location.href = '/'; }}
            className="btn-secondary"
            style={{ flex: 1 }}
          >
            Return to Overview
          </button>
        </div>
      </div>
    </div>
  );
}
