# Aigis-Zero Console

Management console for the Aigis-Zero platform, built with Next.js App Router and TypeScript.

## Architecture

The console provides five operational views:
* **System Overview (`/`):** Real-time fleet health metrics, active threat counts, and live telemetry throughput over WebSocket.
* **Endpoints Inventory (`/endpoints`):** Enrolled host directory, heartbeat status, operating system metadata, and network isolation controls.
* **Security Detections (`/alerts`):** YARA-X rule detections, MITRE ATT&CK taxonomy classification, threat scoring, and analyst triage controls.
* **Threat Hunt (`/hunt`):** Structured query interface across process, network, socket, file, and auth telemetry with expandable JSON payload inspection.
* **System Settings (`/settings`):** Backend health status, fleet server gRPC connection details, agent bootstrap script, and session management.

## Getting Started

### Prerequisites
* Node.js v20+ (tested with v26)
* Running Aigis-Zero API backend on port 8088

### Development Server

Run the development server on port 3000:
```bash
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) in your browser.

### Production Build

Create an optimized static and server-rendered production build:
```bash
npm run build
npm run start
```

### Linting

Verify ESLint compliance:
```bash
npm run lint
```

## Backend Connectivity

API requests to `/api/v1/:path*`, `/healthz`, and `/readyz` are proxied to the local API backend via Next.js rewrites defined in `next.config.mjs`.

Default backend URL: `http://127.0.0.1:8088`

To override the backend destination during development, set the `BACKEND_URL` environment variable:
```bash
BACKEND_URL=http://localhost:8088 npm run dev
```

## Design Specification

Visual styles follow the Cloudflare light theme tokens defined in `.agents/design-spec.md`:
* Background: `#f6f6f7`
* Surface: `#ffffff`
* Border: `#e5e5e5`
* Primary text: `#1a1a1a`
* Muted text: `#8b8b8b`
* Primary accent: `#3b82f6`
* Success: `#16a34a`
* Warning: `#b45309`
* Danger: `#dc2626`
