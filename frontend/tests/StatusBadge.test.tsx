import React from 'react';
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StatusBadge } from '../components/StatusBadge';

describe('StatusBadge', () => {
  it('renders critical severity badge correctly', () => {
    render(<StatusBadge type="severity" value="critical" />);
    expect(screen.getByText('critical')).toBeDefined();
  });

  it('renders online agent_status badge correctly', () => {
    render(<StatusBadge type="agent_status" value="online" />);
    expect(screen.getByText('online')).toBeDefined();
  });

  it('renders isolated operator_status badge correctly', () => {
    render(<StatusBadge type="operator_status" value="isolated" />);
    expect(screen.getByText('isolated')).toBeDefined();
  });

  it('replaces underscores in status values with spaces', () => {
    render(<StatusBadge type="alert_status" value="false_positive" />);
    expect(screen.getByText('false positive')).toBeDefined();
  });
});
