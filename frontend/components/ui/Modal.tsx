'use client';

import React, { useEffect, useRef } from 'react';
import { X } from 'lucide-react';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  maxWidth?: string;
}

export function Modal({
  isOpen,
  onClose,
  title,
  children,
  maxWidth = '520px',
}: ModalProps) {
  const contentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;

    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        onClose();
      }
    }

    // Prevent scrolling behind the modal
    const originalOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      className="dialog-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          onClose();
        }
      }}
      role="presentation"
    >
      <div
        ref={contentRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="modal-dialog-title"
        className="dialog-content"
        style={{ maxWidth }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '16px 20px',
            borderBottom: '1px solid var(--border-dark)',
            background: 'var(--muted-bg)',
          }}
        >
          <h2
            id="modal-dialog-title"
            style={{
              fontFamily: 'var(--font-display)',
              fontSize: '18px',
              fontWeight: 700,
              color: 'var(--fg)',
            }}
          >
            {title}
          </h2>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close dialog"
            style={{
              background: 'transparent',
              border: 'none',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'var(--fg)',
              padding: '4px',
            }}
          >
            <X size={18} />
          </button>
        </div>

        <div style={{ padding: '24px 20px' }}>{children}</div>
      </div>
    </div>
  );
}
