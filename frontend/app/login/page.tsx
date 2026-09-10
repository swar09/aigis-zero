'use client';

import React, { useState, useEffect } from 'react';
import Image from 'next/image';
import { useRouter } from 'next/navigation';
import { AlertCircle, Lock, User } from 'lucide-react';
import { login } from '../../lib/api';
import { AsciiBackground } from '../../components/AsciiBackground';

export default function LoginPage() {
  const router = useRouter();
  const [username, setUsername] = useState('admin');
  const [password, setPassword] = useState('admin_super_secret_password_change_me');
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    document.title = 'Sign In';
  }, []);

  async function handleLogin(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setSubmitting(true);

    try {
      await login(username, password);
      router.push('/');
    } catch (err: unknown) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Authentication failed. Please check your username and password.');
      }
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        width: '100vw',
        height: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        overflow: 'hidden',
        backgroundColor: '#000000',
        zIndex: 10,
      }}
    >
      <AsciiBackground theme="dark" opacity={0.18} />
      <div className="noise-overlay" aria-hidden="true" />
      <div
        className="bg-pattern-grid"
        style={{
          position: 'absolute',
          inset: 0,
          pointerEvents: 'none',
          opacity: 0.12,
        }}
      />

      <div
        className="card"
        style={{
          width: '100%',
          maxWidth: '420px',
          padding: '40px 36px',
          display: 'flex',
          flexDirection: 'column',
          gap: '22px',
          border: '3px solid var(--border-dark)',
          background: 'var(--bg)',
          zIndex: 20,
          position: 'relative',
          boxShadow: '16px 16px 0px 0px rgba(255, 255, 255, 0.14)',
        }}
      >
        <div style={{ textAlign: 'center' }}>
          <div
            style={{
              border: '2px solid var(--border-dark)',
              padding: '10px',
              display: 'inline-flex',
              alignItems: 'center',
              justifyContent: 'center',
              backgroundColor: 'var(--bg)',
              marginBottom: '16px',
            }}
          >
            <Image
              src="/logo.png"
              alt="Aigis-Zero Logo"
              width={64}
              height={64}
              style={{ display: 'block' }}
              priority
            />
          </div>
          <h1
            style={{
              fontFamily: 'var(--font-display)',
              fontSize: '26px',
              fontWeight: 700,
              letterSpacing: '-0.5px',
              color: 'var(--fg)',
            }}
          >
            AIGIS-ZERO
          </h1>
          <p
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: '12px',
              color: 'var(--muted-fg)',
              marginTop: '6px',
            }}
          >
            Sign in to your account
          </p>
        </div>

        {error && (
          <div
            id="login-error"
            role="alert"
            style={{
              padding: '10px 12px',
              background: 'var(--bg)',
              border: '1px solid var(--color-danger)',
              color: 'var(--color-danger)',
              fontSize: '12px',
              fontFamily: 'var(--font-mono)',
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
            }}
          >
            <AlertCircle size={15} style={{ flexShrink: 0 }} />
            <span>{error}</span>
          </div>
        )}

        <form onSubmit={handleLogin} style={{ display: 'flex', flexDirection: 'column', gap: '18px' }}>
          <div>
            <label
              htmlFor="login-username"
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: '11px',
                fontWeight: 600,
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--fg)',
                display: 'block',
                marginBottom: '6px',
              }}
            >
              Username
            </label>
            <div style={{ position: 'relative' }}>
              <User
                size={15}
                aria-hidden="true"
                style={{
                  position: 'absolute',
                  left: '12px',
                  top: '50%',
                  transform: 'translateY(-50%)',
                  color: 'var(--muted-fg)',
                }}
              />
              <input
                id="login-username"
                name="username"
                type="text"
                required
                autoComplete="username"
                aria-invalid={!!error}
                aria-describedby={error ? 'login-error' : undefined}
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="input-text"
                style={{ paddingLeft: '38px' }}
                placeholder="Username"
              />
            </div>
          </div>

          <div>
            <label
              htmlFor="login-password"
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: '11px',
                fontWeight: 600,
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--fg)',
                display: 'block',
                marginBottom: '6px',
              }}
            >
              Password
            </label>
            <div style={{ position: 'relative' }}>
              <Lock
                size={15}
                aria-hidden="true"
                style={{
                  position: 'absolute',
                  left: '12px',
                  top: '50%',
                  transform: 'translateY(-50%)',
                  color: 'var(--muted-fg)',
                }}
              />
              <input
                id="login-password"
                name="password"
                type="password"
                required
                autoComplete="current-password"
                aria-invalid={!!error}
                aria-describedby={error ? 'login-error' : undefined}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="input-text"
                style={{ paddingLeft: '38px' }}
                placeholder="Password"
              />
            </div>
          </div>

          <button
            type="submit"
            className="btn-primary"
            disabled={submitting}
            style={{ width: '100%', padding: '12px', marginTop: '4px' }}
          >
            {submitting ? 'Signing in...' : 'Sign in'}
          </button>
        </form>
      </div>
    </div>
  );
}
