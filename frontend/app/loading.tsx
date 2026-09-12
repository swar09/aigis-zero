import React from 'react';

export default function Loading() {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '32px' }}>
      {/* Header Skeleton */}
      <div style={{ borderBottom: '1px solid var(--border-dark)', paddingBottom: '20px' }}>
        <div className="skeleton" style={{ width: '240px', height: '36px', marginBottom: '10px' }} />
        <div className="skeleton" style={{ width: '420px', height: '18px' }} />
      </div>

      {/* Metric Cards Skeleton Grid */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))',
          gap: '20px',
        }}
      >
        {Array.from({ length: 4 }).map((_, i) => (
          <div
            key={`metric-skel-${i}`}
            className="card"
            style={{
              minHeight: '130px',
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'space-between',
            }}
          >
            <div className="skeleton" style={{ width: '120px', height: '14px' }} />
            <div className="skeleton" style={{ width: '80px', height: '32px' }} />
            <div className="skeleton" style={{ width: '160px', height: '12px' }} />
          </div>
        ))}
      </div>

      {/* Main Content Area Skeleton */}
      <div className="card" style={{ minHeight: '360px', padding: '24px' }}>
        <div className="skeleton" style={{ width: '200px', height: '24px', marginBottom: '20px' }} />
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={`content-skel-${i}`} className="skeleton" style={{ width: '100%', height: '40px' }} />
          ))}
        </div>
      </div>
    </div>
  );
}
