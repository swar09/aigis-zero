import React from 'react';
import Link from 'next/link';
import { ArrowLeft, FileQuestion } from 'lucide-react';

export default function NotFound() {
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
          maxWidth: '520px',
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
            border: '2px solid var(--border-dark)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: 'var(--fg)',
          }}
        >
          <FileQuestion size={28} />
        </div>

        <div>
          <span
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: '12px',
              fontWeight: 600,
              color: 'var(--muted-fg)',
              letterSpacing: '0.08em',
              textTransform: 'uppercase',
            }}
          >
            HTTP 404
          </span>
          <h1
            style={{
              fontFamily: 'var(--font-display)',
              fontSize: '24px',
              fontWeight: 700,
              color: 'var(--fg)',
              marginTop: '4px',
            }}
          >
            Resource Not Found
          </h1>
          <p
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: '13px',
              color: 'var(--muted-fg)',
              marginTop: '6px',
            }}
          >
            The requested console route or resource does not exist.
          </p>
        </div>

        <Link
          href="/"
          className="btn-primary"
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '8px',
            marginTop: '8px',
          }}
        >
          <ArrowLeft size={14} />
          <span>Return to Overview</span>
        </Link>
      </div>
    </div>
  );
}
