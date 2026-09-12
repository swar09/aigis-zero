'use client';

import React from 'react';
import { getStoredUser } from '../lib/api';

export interface CanProps {
  permission: string;
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

export function Can({ permission, children, fallback = null }: CanProps) {
  const user = getStoredUser();

  if (!user) {
    return <>{fallback}</>;
  }

  // Admin role has access to all actions
  if (user.role === 'admin') {
    return <>{children}</>;
  }

  if (Array.isArray(user.permissions) && user.permissions.includes(permission)) {
    return <>{children}</>;
  }

  return <>{fallback}</>;
}
