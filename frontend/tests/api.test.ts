import { describe, it, expect, beforeEach } from 'vitest';
import {
  ApiError,
  clearAuthSession,
  getAuthToken,
  getStoredUser,
  setAuthSession,
} from '../lib/api';

describe('API Client & Session Utilities', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('correctly sets and retrieves auth session in localStorage', () => {
    const user = {
      username: 'analyst_1',
      role: 'soc_analyst',
      permissions: ['nodes:read', 'alerts:read'],
    };
    const token = 'sample.jwt.token';

    setAuthSession(token, user);

    expect(getAuthToken()).toBe(token);
    expect(getStoredUser()).toEqual(user);
  });

  it('clears auth session and cookies on logout', () => {
    const user = {
      username: 'admin',
      role: 'admin',
      permissions: ['*'],
    };
    setAuthSession('test-token', user);

    clearAuthSession();

    expect(getAuthToken()).toBeNull();
    expect(getStoredUser()).toBeNull();
  });

  it('instantiates ApiError with status and code', () => {
    const error = new ApiError('Resource not found', 404, 'NOT_FOUND');
    expect(error.name).toBe('ApiError');
    expect(error.message).toBe('Resource not found');
    expect(error.status).toBe(404);
    expect(error.code).toBe('NOT_FOUND');
  });
});
