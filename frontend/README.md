# Aigis-Zero Console

Management console for the Aigis-Zero platform, built with Next.js App Router and TypeScript.

For operator workflows, reverse proxy configurations, and production deployment runbooks, see the [Frontend guide](guide.md).

## Architecture

The console provides five operational views:
* **System Overview (`/`):** Real-time fleet health metrics, active threat counts, and live telemetry throughput over WebSocket.
* **Endpoints Inventory (`/endpoints`):** Enrolled host directory, heartbeat status, operating system metadata, and network isolation controls.
* **Security Detections (`/alerts`):** YARA-X rule detections, MITRE ATT&CK taxonomy classification, threat scoring, and analyst triage controls.
* **Threat Hunt (`/hunt`):** Structured query interface across process, network, socket, file, and auth telemetry with expandable JSON payload inspection.
* **System Settings (`/settings`):** Backend health status, fleet server gRPC connection details, agent bootstrap script, and session management.

## Getting Started

### Prerequisites
* Node.js v20+ (tested with v20 and v22)
* Running Aigis-Zero API backend on port 8080

### Development Server

Run the development server on port 3000:
```bash
npm install
BACKEND_URL=http://localhost:8080 npm run dev
```

Open [http://localhost:3000](http://localhost:3000) in your browser. Default login: `admin` / `admin`.

### Production Build

Create an optimized static and server-rendered production build:
```bash
npm run build
npm run start
```

### Quality and Testing

```bash
# Verify ESLint compliance
npm run lint

# Verify TypeScript types
npm run typecheck

# Run test suite
npm run test
```

## Backend Connectivity

API requests to `/api/v1/:path*`, `/healthz`, and `/readyz` are proxied to the API backend via Next.js rewrites defined in `next.config.mjs`.

Default backend URL in Docker: `http://api-backend:8080`
Default backend URL in local development: `http://localhost:8080`

Override the backend destination during development using `BACKEND_URL`:
```bash
BACKEND_URL=http://localhost:8080 npm run dev
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

## Additional Resources

- [Frontend deployment and operations guide](guide.md): In-depth guide covering standalone mode, Docker builds, and proxy rewrites.
