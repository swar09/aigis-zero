'use client';

import React, { useEffect, useState } from 'react';
import Image from 'next/image';
import { usePathname, useRouter } from 'next/navigation';
import { LogOut, Search, User } from 'lucide-react';
import { clearAuthSession, getStoredUser } from '../lib/api';
import { UserInfo } from '../lib/types';
import { CommandPalette } from './CommandPalette';
import { useSidebar } from './SidebarContext';

export function TopBar() {
  const pathname = usePathname();
  const router = useRouter();
  const [user, setUser] = useState<UserInfo | null>(null);
  const [isCommandOpen, setIsCommandOpen] = useState(false);
  const { isSidebarOpen, toggleSidebar } = useSidebar();

  useEffect(() => {
    setUser(getStoredUser());
  }, [pathname]);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setIsCommandOpen((prev) => !prev);
      }
    }
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  function handleLogout() {
    clearAuthSession();
    router.push('/login');
  }

  if (pathname === '/login') {
    return null;
  }

  return (
    <>
      <header
        style={{
          height: 'var(--topbar-height)',
          background: 'var(--bg)',
          borderBottom: '1px solid var(--border-dark)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '0 28px',
          position: 'sticky',
          top: 0,
          zIndex: 100,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <button
            onClick={toggleSidebar}
            title={isSidebarOpen ? 'Collapse sidebar' : 'Expand sidebar'}
            aria-label="Toggle sidebar"
            className="topbar-brand-btn"
          >
            <div
              style={{
                border: '1px solid var(--border-dark)',
                padding: '3px',
                display: 'inline-flex',
                alignItems: 'center',
                justifyContent: 'center',
                backgroundColor: 'var(--bg)',
              }}
            >
              <Image
                src="/logo.png"
                alt="Aigis-Zero Logo"
                width={26}
                height={26}
                style={{ display: 'block' }}
                priority
              />
            </div>
            <span
              style={{
                fontFamily: 'var(--font-display)',
                fontSize: '18px',
                fontWeight: 700,
                letterSpacing: '-0.5px',
                color: 'var(--fg)',
              }}
            >
              AIGIS-ZERO
            </span>
          </button>

          <span
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '6px',
              padding: '3px 8px',
              fontSize: '11px',
              fontFamily: 'var(--font-mono)',
              border: '1px solid var(--border-dark)',
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
              color: 'var(--fg)',
              background: 'var(--muted-bg)',
            }}
          >
            <span className="status-dot" />
            LOCAL CLUSTER
          </span>
        </div>

        <div style={{ flex: 1, maxWidth: '440px', margin: '0 24px' }}>
          <button
            onClick={() => setIsCommandOpen(true)}
            style={{
              width: '100%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '7px 12px',
              fontSize: '12px',
              fontFamily: 'var(--font-mono)',
              color: 'var(--color-text-muted)',
              background: 'var(--bg)',
              border: '1px solid var(--border-dark)',
              cursor: 'pointer',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Search size={14} />
              <span>Search telemetry, hosts, and alerts...</span>
            </div>
            <kbd
              style={{
                fontSize: '10px',
                fontFamily: 'var(--font-mono)',
                padding: '1px 6px',
                background: 'var(--muted-bg)',
                border: '1px solid var(--border-dark)',
              }}
            >
              Ctrl+K
            </kbd>
          </button>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <span className="status-dot" />
            <span
              style={{
                fontSize: '11px',
                fontFamily: 'var(--font-mono)',
                letterSpacing: '0.05em',
                textTransform: 'uppercase',
                color: 'var(--color-text-muted)',
              }}
            >
              Gateway Live
            </span>
          </div>

          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              padding: '4px 10px',
              background: 'var(--bg)',
              border: '1px solid var(--border-dark)',
            }}
          >
            <User size={13} style={{ color: 'var(--fg)' }} />
            <span
              style={{
                fontSize: '12px',
                fontFamily: 'var(--font-mono)',
                fontWeight: 600,
                color: 'var(--fg)',
              }}
            >
              {user?.username || 'admin'}
            </span>
          </div>

          <button
            onClick={handleLogout}
            title="Sign out"
            style={{
              background: 'transparent',
              border: '1px solid var(--border-dark)',
              cursor: 'pointer',
              color: 'var(--fg)',
              display: 'flex',
              alignItems: 'center',
              padding: '6px 8px',
              transition: 'background 0.12s ease',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = 'var(--muted-bg)';
              e.currentTarget.style.color = 'var(--color-danger)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.color = 'var(--fg)';
            }}
          >
            <LogOut size={15} />
          </button>
        </div>
      </header>

      <CommandPalette isOpen={isCommandOpen} onClose={() => setIsCommandOpen(false)} />
    </>
  );
}
