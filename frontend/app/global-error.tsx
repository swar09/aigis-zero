'use client';

import React, { useEffect } from 'react';
import { AlertOctagon, RefreshCw } from 'lucide-react';

export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error('Unhandled global application error:', error);
  }, [error]);

  return (
    <html lang="en">
      <body
        style={{
          margin: 0,
          padding: 0,
          backgroundColor: '#0a0a0a',
          color: '#f5f5f5',
          fontFamily: 'monospace',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          minHeight: '100vh',
        }}
      >
        <div
          style={{
            maxWidth: '560px',
            width: '90%',
            padding: '36px 32px',
            backgroundColor: '#111111',
            border: '2px solid #282828',
            boxShadow: '8px 8px 0px 0px rgba(0, 0, 0, 1)',
            textAlign: 'center',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: '20px',
          }}
        >
          <div
            style={{
              width: '52px',
              height: '52px',
              border: '2px solid #ef4444',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#ef4444',
            }}
          >
            <AlertOctagon size={28} />
          </div>

          <div>
            <h1
              style={{
                fontSize: '22px',
                fontWeight: 700,
                margin: 0,
                color: '#f5f5f5',
              }}
            >
              System Error
            </h1>
            <p
              style={{
                fontSize: '13px',
                color: '#888888',
                marginTop: '8px',
                marginBottom: 0,
              }}
            >
              A critical error occurred while rendering the root application layout.
            </p>
          </div>

          <div
            style={{
              width: '100%',
              backgroundColor: '#181818',
              border: '1px solid #333333',
              padding: '12px 14px',
              fontSize: '12px',
              textAlign: 'left',
              overflowX: 'auto',
              color: '#ef4444',
              boxSizing: 'border-box',
            }}
          >
            {error.message || 'Unknown runtime error'}
          </div>

          <button
            type="button"
            onClick={() => reset()}
            style={{
              padding: '10px 20px',
              backgroundColor: '#f5f5f5',
              color: '#0a0a0a',
              border: 'none',
              cursor: 'pointer',
              fontWeight: 600,
              fontSize: '12px',
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
              display: 'inline-flex',
              alignItems: 'center',
              gap: '8px',
            }}
          >
            <RefreshCw size={14} />
            <span>Reload Application</span>
          </button>
        </div>
      </body>
    </html>
  );
}
