'use client';

import React from 'react';
import { usePathname } from 'next/navigation';
import { TopBar } from './TopBar';
import { Sidebar } from './Sidebar';
import { SidebarProvider, useSidebar } from './SidebarContext';

function AppShellContent({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const { isSidebarOpen } = useSidebar();
  const isLoginPage = pathname === '/login';

  if (isLoginPage) {
    return (
      <div style={{ position: 'relative', width: '100vw', height: '100vh', overflow: 'hidden' }}>
        {children}
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', minHeight: '100vh', position: 'relative' }}>
      <TopBar />
      <div style={{ display: 'flex', flex: 1, position: 'relative', zIndex: 1 }}>
        {isSidebarOpen && <Sidebar />}

        <main
          style={{
            flex: 1,
            padding: '32px 40px',
            maxWidth: isSidebarOpen ? '1440px' : '1600px',
            width: '100%',
            margin: '0 auto',
            overflowX: 'auto',
            transition: 'max-width 0.15s ease',
          }}
        >
          {children}
        </main>
      </div>
    </div>
  );
}

export function AppShell({ children }: { children: React.ReactNode }) {
  return (
    <SidebarProvider>
      <AppShellContent>{children}</AppShellContent>
    </SidebarProvider>
  );
}
