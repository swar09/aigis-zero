'use client';

import React from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import {
  AlertTriangle,
  Crosshair,
  FileText,
  LayoutDashboard,
  Network,
  Server,
  Settings,
} from 'lucide-react';

interface NavItem {
  name: string;
  href: string;
  icon: React.ComponentType<{ size?: number; style?: React.CSSProperties }>;
}

const NAV_ITEMS: NavItem[] = [
  { name: 'Dashboard', href: '/', icon: LayoutDashboard },
  { name: 'Alerts', href: '/alerts', icon: AlertTriangle },
  { name: 'Endpoints', href: '/endpoints', icon: Server },
  { name: 'Threat Hunt', href: '/hunt', icon: Crosshair },
  { name: 'Logs', href: '/logs', icon: FileText },
  { name: 'Network', href: '/network', icon: Network },
  { name: 'Settings', href: '/settings', icon: Settings },
];

export function Sidebar() {
  const pathname = usePathname();

  if (pathname === '/login') {
    return null;
  }

  return (
    <aside
      style={{
        width: 'var(--sidebar-width)',
        background: 'var(--bg)',
        borderRight: '1px solid var(--border-dark)',
        display: 'flex',
        flexDirection: 'column',
        height: 'calc(100vh - var(--topbar-height))',
        position: 'sticky',
        top: 'var(--topbar-height)',
        padding: '20px 16px',
        overflowY: 'auto',
        flexShrink: 0,
      }}
    >
      <nav style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
        {NAV_ITEMS.map((item) => {
          const isActive = pathname === item.href || (item.href !== '/' && pathname.startsWith(item.href));
          const Icon = item.icon;

          return (
            <Link
              key={item.href}
              href={item.href}
              className={`sidebar-link ${isActive ? 'active' : ''}`}
            >
              <Icon
                size={16}
                style={{
                  color: isActive ? 'var(--fg)' : 'var(--color-text-muted)',
                  flexShrink: 0,
                }}
              />
              <span>{item.name}</span>
            </Link>
          );
        })}
      </nav>
    </aside>
  );
}
