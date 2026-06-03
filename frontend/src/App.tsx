import { useState, useEffect, useRef } from 'react'
import './App.css'
import AsciiBackground from './AsciiBackground';

function App() {
  // Auth states
  const [isLoggedIn, setIsLoggedIn] = useState<boolean>(() => {
    try {
      return !!localStorage.getItem('aigis_auth');
    } catch {
      return false;
    }
  });
  const [loggedUser, setLoggedUser] = useState(() => localStorage.getItem('aigis_user') || 'operator');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [loginError, setLoginError] = useState('');

  // Dashboard states
  const [activeTab, setActiveTab] = useState('nodes');
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setUserMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Custom cursor position state
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const [isHovered, setIsHovered] = useState(false);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      setPosition({ x: e.clientX, y: e.clientY });
    };

    window.addEventListener('mousemove', handleMouseMove);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
    };
  }, []);

  useEffect(() => {
    const handleMouseOver = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (target && typeof target.closest === 'function') {
        if (
          target.tagName === 'A' || 
          target.tagName === 'BUTTON' || 
          target.closest('a') || 
          target.closest('button') || 
          target.closest('.feature-card') ||
          target.closest('.pricing-card')
        ) {
          setIsHovered(true);
          return;
        }
      }
      setIsHovered(false);
    };

    window.addEventListener('mouseover', handleMouseOver);
    return () => {
      window.removeEventListener('mouseover', handleMouseOver);
    };
  }, []);

  const handleLogin = (e: React.FormEvent) => {
    e.preventDefault();
    if (username.trim() === 'admin' && password.trim() === 'admin') {
      localStorage.setItem('aigis_auth', 'true');
      localStorage.setItem('aigis_user', username.trim());
      setLoggedUser(username.trim());
      setIsLoggedIn(true);
      setLoginError('');
    } else {
      setLoginError('[ERROR] INVALID CREDENTIALS. ACCESS DENIED.');
    }
  };

  const handleLogout = () => {
    localStorage.removeItem('aigis_auth');
    localStorage.removeItem('aigis_user');
    setIsLoggedIn(false);
    setUsername('');
    setPassword('');
    setUserMenuOpen(false);
  };

  if (!isLoggedIn) {
    return (
      <>
        <div 
          className={`custom-cursor ${isHovered ? 'hovered' : ''}`}
          style={{ left: `${position.x}px`, top: `${position.y}px` }}
        />
        <div className="login-outer-wrapper bg-pattern-grid">
          <AsciiBackground />
          <div className="noise-overlay" aria-hidden="true" />
        <div className="login-card">
          <div className="login-logo-row" style={{ flexDirection: 'column', gap: '16px' }}>
            <div style={{ border: '2px solid var(--border-dark)', padding: '10px', display: 'inline-flex', alignItems: 'center', justifyContent: 'center', backgroundColor: 'var(--bg)' }}>
              <img src="/logo.png" alt="" width="72" height="72" style={{ display: 'block' }} />
            </div>
            <span className="brand">AIGIS-ZERO</span>
          </div>

          {loginError && (
            <div className="login-error-box" role="alert">
              {loginError}
            </div>
          )}

          <form onSubmit={handleLogin}>
            <div className="login-form-group">
              <div>
                <label htmlFor="op-username" className="login-input-label">Username</label>
                <input
                  id="op-username"
                  type="text"
                  placeholder="????????"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  required
                  className="login-input"
                  autoComplete="username"
                />
              </div>

              <div>
                <label htmlFor="op-password" className="login-input-label">Security Key</label>
                <input
                  id="op-password"
                  type="password"
                  placeholder="••••••••"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  required
                  className="login-input"
                  autoComplete="current-password"
                />
              </div>
            </div>

            <button type="submit" className="btn-primary" style={{ width: '100%', cursor: 'pointer' }}>
              Authenticate →
            </button>
          </form>
        </div>
      </div>
      </>
    );
  }

  return (
    <>
      <div 
        className={`custom-cursor ${isHovered ? 'hovered' : ''}`}
        style={{ left: `${position.x}px`, top: `${position.y}px` }}
      />
      {/* Global Noise Overlay & Subtle Lines Grid background */}
      <div className="noise-overlay" aria-hidden="true" />
      
      {/* Accessibility Skip Link */}
      <a href="#main-content" className="skip-link">Skip to main content</a>

      <div className="app-container bg-pattern-lines">
        
        {/* Navigation / Header */}
        <header className="app-header">
          <div className="brand" style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <img src="/logo.png" alt="" width="40" height="40" style={{ display: 'block' }} />
            AIGIS-ZERO
          </div>
          
          <nav className="nav-links" aria-label="Main Navigation">
            <button className={`nav-link ${activeTab === 'nodes' ? 'active' : ''}`} onClick={() => setActiveTab('nodes')} style={{ background: 'transparent', border: 'none', cursor: 'pointer', padding: 0 }}>NODES</button>
            <button className={`nav-link ${activeTab === 'alerts' ? 'active' : ''}`} onClick={() => setActiveTab('alerts')} style={{ background: 'transparent', border: 'none', cursor: 'pointer', padding: 0 }}>ALERTS</button>
            <button className={`nav-link ${activeTab === 'deploy' ? 'active' : ''}`} onClick={() => setActiveTab('deploy')} style={{ background: 'transparent', border: 'none', cursor: 'pointer', padding: 0 }}>DEPLOY</button>
            <button className={`nav-link ${activeTab === 'settings' ? 'active' : ''}`} onClick={() => setActiveTab('settings')} style={{ background: 'transparent', border: 'none', cursor: 'pointer', padding: 0 }}>SETTINGS</button>
            
            <div className="user-dropdown-container" style={{ position: 'relative' }} ref={dropdownRef}>
              <button 
                className="status-badge" 
                onClick={() => setUserMenuOpen(!userMenuOpen)}
                style={{ background: 'transparent', cursor: 'pointer', color: 'var(--fg)' }}
                title="User Profile"
              >
                <span className="status-dot" aria-hidden="true" style={{ backgroundColor: '#0f0' }} />
                <span>{loggedUser}</span>
              </button>
              
              {userMenuOpen && (
                <div className="user-dropdown-menu" style={{
                  position: 'absolute',
                  top: '100%',
                  right: '0',
                  marginTop: '6px',
                  backgroundColor: 'var(--bg)',
                  border: '1px solid var(--border-dark)',
                  padding: '4px 0',
                  minWidth: '100px',
                  zIndex: 100
                }}>
                  <button 
                    onClick={handleLogout} 
                    style={{ 
                      width: '100%', 
                      textAlign: 'left', 
                      background: 'transparent', 
                      border: 'none', 
                      cursor: 'pointer', 
                      padding: '4px 12px',
                      color: 'var(--fg)',
                      fontFamily: 'var(--font-mono)',
                      fontSize: '11px',
                      textTransform: 'uppercase'
                    }}
                  >
                    Logout
                  </button>
                </div>
              )}
            </div>
          </nav>
        </header>

        <main id="main-content">
          <section className="section-padding">
            <h1 className="hero-large-text" style={{ fontSize: '40px', marginBottom: '16px' }}>
              {activeTab.toUpperCase()}
            </h1>
            <p className="hero-desc" style={{ color: 'var(--muted-fg)' }}>
              This section is currently being provisioned.
            </p>
          </section>
        </main>

      </div>
    </>
  )
}

export default App
