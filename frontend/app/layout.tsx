import React from 'react';
import type { Metadata, Viewport } from 'next';
import { JetBrains_Mono, Playfair_Display, Source_Serif_4 } from 'next/font/google';
import './globals.css';
import { AppShell } from '../components/AppShell';

const fontMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-mono-next',
  display: 'swap',
});

const fontDisplay = Playfair_Display({
  subsets: ['latin'],
  variable: '--font-display-next',
  display: 'swap',
});

const fontBody = Source_Serif_4({
  subsets: ['latin'],
  variable: '--font-body-next',
  display: 'swap',
});

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  themeColor: '#0a0a0a',
};

export const metadata: Metadata = {
  title: 'Aigis-Zero Console',
  description: 'Endpoint Security Management Console',
  icons: {
    icon: [
      { url: '/favicon.svg', type: 'image/svg+xml' },
      { url: '/favicon-96x96.png', sizes: '96x96', type: 'image/png' },
      { url: '/favicon.ico' },
    ],
    apple: [
      { url: '/apple-touch-icon.png', sizes: '180x180', type: 'image/png' },
    ],
  },
  manifest: '/site.webmanifest',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body
        className={`bg-pattern-lines ${fontMono.variable} ${fontDisplay.variable} ${fontBody.variable}`}
        suppressHydrationWarning
      >
        <div className="noise-overlay" aria-hidden="true" />
        <AppShell>{children}</AppShell>
      </body>
    </html>
  );
}
