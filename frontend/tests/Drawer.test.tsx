import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Drawer } from '../components/ui/Drawer';

describe('Drawer Slide-Over Primitive', () => {
  it('does not render when isOpen is false', () => {
    render(
      <Drawer isOpen={false} onClose={() => {}} title="Event Forensics">
        <div>Event Payload</div>
      </Drawer>
    );
    expect(screen.queryByText('Event Forensics')).toBeNull();
  });

  it('renders title, subtitle, and body when isOpen is true', () => {
    render(
      <Drawer
        isOpen={true}
        onClose={() => {}}
        title="Alert Details"
        subtitle="Host: worker-01.local"
      >
        <div>MITRE ATT&CK Tactic: Lateral Movement</div>
      </Drawer>
    );
    expect(screen.getByText('Alert Details')).toBeDefined();
    expect(screen.getByText('Host: worker-01.local')).toBeDefined();
    expect(
      screen.getByText('MITRE ATT&CK Tactic: Lateral Movement')
    ).toBeDefined();
  });

  it('invokes onClose when clicking the close button', () => {
    const handleClose = vi.fn();
    render(
      <Drawer isOpen={true} onClose={handleClose} title="Detail Pane">
        <div>Content</div>
      </Drawer>
    );

    const closeBtn = screen.getByLabelText('Close detail pane');
    fireEvent.click(closeBtn);
    expect(handleClose).toHaveBeenCalledTimes(1);
  });
});
