import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Modal } from '../components/ui/Modal';

describe('Modal Dialog Primitive', () => {
  it('does not render when isOpen is false', () => {
    render(
      <Modal isOpen={false} onClose={() => {}} title="Test Modal">
        <div>Modal Body</div>
      </Modal>
    );
    expect(screen.queryByText('Test Modal')).toBeNull();
    expect(screen.queryByText('Modal Body')).toBeNull();
  });

  it('renders title and children when isOpen is true', () => {
    render(
      <Modal isOpen={true} onClose={() => {}} title="Quarantine Confirmation">
        <div>Are you sure you want to isolate this node?</div>
      </Modal>
    );
    expect(screen.getByText('Quarantine Confirmation')).toBeDefined();
    expect(
      screen.getByText('Are you sure you want to isolate this node?')
    ).toBeDefined();
  });

  it('calls onClose when close button is clicked', () => {
    const handleClose = vi.fn();
    render(
      <Modal isOpen={true} onClose={handleClose} title="Closeable Modal">
        <div>Body Content</div>
      </Modal>
    );

    const closeBtn = screen.getByLabelText('Close dialog');
    fireEvent.click(closeBtn);
    expect(handleClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when Escape key is pressed', () => {
    const handleClose = vi.fn();
    render(
      <Modal isOpen={true} onClose={handleClose} title="Escape Test">
        <div>Body</div>
      </Modal>
    );

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(handleClose).toHaveBeenCalledTimes(1);
  });
});
