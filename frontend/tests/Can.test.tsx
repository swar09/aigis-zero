import React from 'react';
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Can } from '../components/Can';
import { setAuthSession } from '../lib/api';

describe('Can RBAC Permission Component', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('renders fallback when no user is logged in', () => {
    render(
      <Can
        permission="nodes:isolate"
        fallback={<div>Access Restricted</div>}
      >
        <button>Isolate Host</button>
      </Can>
    );

    expect(screen.getByText('Access Restricted')).toBeDefined();
    expect(screen.queryByText('Isolate Host')).toBeNull();
  });

  it('renders children when user has admin role', () => {
    setAuthSession('test-token', {
      username: 'root_admin',
      role: 'admin',
      permissions: [],
    });

    render(
      <Can
        permission="nodes:isolate"
        fallback={<div>Access Restricted</div>}
      >
        <button>Isolate Host</button>
      </Can>
    );

    expect(screen.getByText('Isolate Host')).toBeDefined();
    expect(screen.queryByText('Access Restricted')).toBeNull();
  });

  it('renders children when user explicitly has the required permission', () => {
    setAuthSession('test-token', {
      username: 'analyst_bob',
      role: 'soc_analyst',
      permissions: ['nodes:read', 'nodes:isolate'],
    });

    render(
      <Can
        permission="nodes:isolate"
        fallback={<div>Access Restricted</div>}
      >
        <button>Isolate Host</button>
      </Can>
    );

    expect(screen.getByText('Isolate Host')).toBeDefined();
  });

  it('renders fallback when user lacks the required permission', () => {
    setAuthSession('test-token', {
      username: 'viewer_charlie',
      role: 'viewer',
      permissions: ['nodes:read', 'alerts:read'],
    });

    render(
      <Can
        permission="nodes:isolate"
        fallback={<div>Access Restricted</div>}
      >
        <button>Isolate Host</button>
      </Can>
    );

    expect(screen.getByText('Access Restricted')).toBeDefined();
    expect(screen.queryByText('Isolate Host')).toBeNull();
  });
});
