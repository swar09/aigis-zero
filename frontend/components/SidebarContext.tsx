'use client';

import React, { createContext, useContext, useEffect, useState } from 'react';

interface SidebarContextType {
  isSidebarOpen: boolean;
  toggleSidebar: () => void;
  openSidebar: () => void;
  closeSidebar: () => void;
}

const SidebarContext = createContext<SidebarContextType>({
  isSidebarOpen: true,
  toggleSidebar: () => {},
  openSidebar: () => {},
  closeSidebar: () => {},
});

export function SidebarProvider({ children }: { children: React.ReactNode }) {
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);

  useEffect(() => {
    try {
      const saved = localStorage.getItem('aigis_sidebar_open');
      if (saved !== null) {
        setIsSidebarOpen(saved === 'true');
      }
    } catch {
      // Ignore local storage errors
    }
  }, []);

  const toggleSidebar = () => {
    setIsSidebarOpen((prev) => {
      const next = !prev;
      try {
        localStorage.setItem('aigis_sidebar_open', String(next));
      } catch {
        // Ignore local storage errors
      }
      return next;
    });
  };

  const openSidebar = () => {
    setIsSidebarOpen(true);
    try {
      localStorage.setItem('aigis_sidebar_open', 'true');
    } catch {
      // Ignore local storage errors
    }
  };

  const closeSidebar = () => {
    setIsSidebarOpen(false);
    try {
      localStorage.setItem('aigis_sidebar_open', 'false');
    } catch {
      // Ignore local storage errors
    }
  };

  return (
    <SidebarContext.Provider
      value={{
        isSidebarOpen,
        toggleSidebar,
        openSidebar,
        closeSidebar,
      }}
    >
      {children}
    </SidebarContext.Provider>
  );
}

export function useSidebar() {
  return useContext(SidebarContext);
}
