import React from 'react';
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MetricCard } from '../components/MetricCard';

describe('MetricCard', () => {
  it('renders title and value correctly', () => {
    render(<MetricCard title="ACTIVE THREATS" value={17} />);
    expect(screen.getByText('ACTIVE THREATS')).toBeDefined();
    expect(screen.getByText('17')).toBeDefined();
  });

  it('renders subtitle when provided', () => {
    render(
      <MetricCard
        title="FLEET ONLINE"
        value="42/50"
        subtitle="84% reporting healthy"
      />
    );
    expect(screen.getByText('84% reporting healthy')).toBeDefined();
  });

  it('displays loading state placeholder', () => {
    render(<MetricCard title="QUEUED EVENTS" value={100} loading={true} />);
    expect(screen.getByText('...')).toBeDefined();
  });
});
