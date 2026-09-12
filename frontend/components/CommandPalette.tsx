'use client';

import React, { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import {
  AlertTriangle,
  Crosshair,
  FileText,
  LayoutDashboard,
  Network,
  Search,
  Server,
  Settings,
  X,
} from 'lucide-react';

interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
}

interface CommandItem {
  id: string;
  label: string;
  category: string;
  href: string;
  icon: React.ComponentType<{ size?: number; style?: React.CSSProperties }>;
}

const COMMAND_ITEMS: CommandItem[] = [
  { id: 'dashboard', label: 'Dashboard', category: 'Shortcuts', href: '/', icon: LayoutDashboard },
  { id: 'alerts', label: 'Alerts', category: 'Shortcuts', href: '/alerts', icon: AlertTriangle },
  { id: 'endpoints', label: 'Endpoints', category: 'Shortcuts', href: '/endpoints', icon: Server },
  { id: 'hunt', label: 'Threat Hunt', category: 'Shortcuts', href: '/hunt', icon: Crosshair },
  { id: 'logs', label: 'Logs', category: 'Shortcuts', href: '/logs', icon: FileText },
  { id: 'network', label: 'Network', category: 'Shortcuts', href: '/network', icon: Network },
  { id: 'settings', label: 'Settings', category: 'Shortcuts', href: '/settings', icon: Settings },
];

export function CommandPalette({ isOpen, onClose }: CommandPaletteProps) {
  const router = useRouter();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        if (isOpen) {
          onClose();
        }
      } else if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  const filtered = COMMAND_ITEMS.filter((item) =>
    item.label.toLowerCase().includes(query.toLowerCase()) ||
    item.category.toLowerCase().includes(query.toLowerCase())
  );

  function executeItem(item: CommandItem) {
    onClose();
    router.push(item.href);
  }

  if (!isOpen) return null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.5)',
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'center',
        paddingTop: '120px',
        zIndex: 1000,
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: '100%',
          maxWidth: '560px',
          background: 'var(--color-surface)',
          border: '2px solid var(--border-dark)',
          overflow: 'hidden',
          boxShadow: '0 16px 32px rgba(0, 0, 0, 0.2)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            padding: '12px 16px',
            borderBottom: '1px solid var(--border-dark)',
            gap: '10px',
            background: 'var(--bg)',
          }}
        >
          <Search size={16} style={{ color: 'var(--color-text-muted)' }} />
          <input
            type="text"
            placeholder="Type a command or jump to page..."
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            autoFocus
            style={{
              flex: 1,
              border: 'none',
              outline: 'none',
              fontFamily: 'var(--font-mono)',
              fontSize: '13px',
              color: 'var(--color-text-primary)',
              background: 'transparent',
            }}
          />
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <X size={16} />
          </button>
        </div>

        <div style={{ maxHeight: '320px', overflowY: 'auto', padding: '6px' }}>
          {filtered.length === 0 ? (
            <div
              style={{
                padding: '24px',
                textAlign: 'center',
                color: 'var(--color-text-muted)',
                fontFamily: 'var(--font-mono)',
                fontSize: '12px',
              }}
            >
              No commands matching &quot;{query}&quot;
            </div>
          ) : (
            filtered.map((item, index) => {
              const isSelected = index === selectedIndex;
              const Icon = item.icon;
              return (
                <div
                  key={item.id}
                  onClick={() => executeItem(item)}
                  onMouseEnter={() => setSelectedIndex(index)}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '10px 12px',
                    cursor: 'pointer',
                    background: isSelected ? 'var(--muted-bg)' : 'transparent',
                    border: isSelected ? '1px solid var(--border-dark)' : '1px solid transparent',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                    <Icon size={16} style={{ color: 'var(--fg)' }} />
                    <span
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: '13px',
                        color: 'var(--color-text-primary)',
                        fontWeight: 500,
                      }}
                    >
                      {item.label}
                    </span>
                  </div>
                  <span
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontSize: '11px',
                      color: 'var(--color-text-muted)',
                      textTransform: 'uppercase',
                    }}
                  >
                    {item.category}
                  </span>
                </div>
              );
            })
          )}
        </div>

        <div
          style={{
            padding: '8px 16px',
            background: 'var(--muted-bg)',
            borderTop: '1px solid var(--border-dark)',
            display: 'flex',
            justifyContent: 'space-between',
            fontFamily: 'var(--font-mono)',
            fontSize: '11px',
            color: 'var(--color-text-muted)',
          }}
        >
          <span>Use Esc to close</span>
          <span>Quick search</span>
        </div>
      </div>
    </div>
  );
}
