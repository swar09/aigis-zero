# Frontend Dashboard — Implementation Timeline

> **Phase**: 7 (Operator Dashboard)
> **Priority**: 🟢 Medium — depends on API backend being functional
> **Estimated Duration**: 7–9 days
> **Depends on**: `api-backend` REST + WebSocket endpoints working

---

## PR Plan

### PR #1 — Vite + React + TypeScript project init
**Branch**: `feat/frontend-init`
**Duration**: 1 day

**Tasks**:
- [ ] `npm create vite@latest . -- --template react-ts`
- [ ] Install deps: axios, @tanstack/react-query, zustand, react-router-dom, recharts
- [ ] Install dev deps: tailwindcss, postcss, autoprefixer
- [ ] Configure Tailwind, PostCSS, Vite proxy to API backend
- [ ] Create base layout (Sidebar + TopBar + content area)
- [ ] Set up React Router with route stubs

### PR #2 — Auth flow and API client
**Branch**: `feat/frontend-auth`
**Duration**: 1.5 days
**Depends on**: PR #1

**Files**:
- `src/api/client.ts` — axios instance with JWT interceptor
- `src/api/auth.ts` — login, refresh, logout API calls
- `src/store/authStore.ts` — Zustand store for JWT + user state
- `src/pages/LoginPage.tsx` — login form

**Tasks**:
- [ ] Axios interceptor: attach Bearer token, auto-refresh on 401
- [ ] Login page with form validation
- [ ] Protected route wrapper (redirect to login if unauthenticated)
- [ ] Persist auth state in memory only (not localStorage — security)

### PR #3 — Node Map page
**Branch**: `feat/frontend-nodes`
**Duration**: 1.5 days
**Depends on**: PR #2

**Files**:
- `src/pages/NodeMapPage.tsx`, `src/pages/NodeDetailPage.tsx`
- `src/hooks/useNodes.ts`, `src/api/nodes.ts`
- `src/components/nodes/{NodeCard, NodeStatusBadge, IsolateButton}.tsx`

**Tasks**:
- [ ] `useNodes()` hook — React Query, polls `GET /nodes` every 30s
- [ ] Node grid with colour-coded status badges (green/yellow/red)
- [ ] Click card → navigate to NodeDetailPage
- [ ] NodeDetailPage: node info + embedded log table + alert count
- [ ] IsolateButton with confirmation modal

### PR #4 — Alerts panel
**Branch**: `feat/frontend-alerts`
**Duration**: 1.5 days
**Depends on**: PR #2

**Files**:
- `src/pages/AlertsPage.tsx`
- `src/hooks/useAlerts.ts`, `src/api/alerts.ts`
- `src/components/alerts/{AlertRow, SeverityBadge, MitreTechniqueTag}.tsx`

**Tasks**:
- [ ] Alerts table with server-side filtering (severity, status, date range)
- [ ] SeverityBadge colour coding (Critical=red, High=orange, Medium=yellow, Low=blue)
- [ ] MITRE technique ID as clickable tag (links to attack.mitre.org)
- [ ] Acknowledge / Dismiss buttons with optimistic UI updates
- [ ] Unacknowledged count badge in sidebar

### PR #5 — Live Logs and WebSocket
**Branch**: `feat/frontend-live`
**Duration**: 1.5 days
**Depends on**: PR #3

**Files**:
- `src/pages/LiveLogsPage.tsx`
- `src/hooks/useWebSocket.ts`
- `src/components/logs/{LogTable, LogTypeFilter}.tsx`

**Tasks**:
- [ ] WebSocket connection manager with auto-reconnect
- [ ] Live log stream appended to circular buffer (max 500 entries)
- [ ] TanStack Table with row virtualisation for performance
- [ ] Client-side type filter (process, file, network, osquery)
- [ ] Real-time alert badge updates via WS `alert_created` events

### PR #6 — Dashboard overview and polish
**Branch**: `feat/frontend-dashboard`
**Duration**: 1 day
**Depends on**: PR #3, #4, #5

**Files**:
- `src/pages/DashboardPage.tsx`

**Tasks**:
- [ ] Summary cards: total nodes, active alerts, events/min
- [ ] Recharts: alert trend (last 24h), severity distribution pie chart
- [ ] Recent alerts list (top 5)
- [ ] Node health overview (healthy/degraded/isolated counts)
- [ ] Responsive layout for tablet/desktop
