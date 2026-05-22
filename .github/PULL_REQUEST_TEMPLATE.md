## Summary
<!-- What does this PR do? One paragraph. -->

## Type
- [ ] feat — new functionality
- [ ] fix — bug fix
- [ ] chore — dependency update, refactor, tooling
- [ ] docs — documentation only
- [ ] sec — security fix or hardening

## Target Module / Crate
<!-- Check ALL modules this PR touches -->

**SDK**
- [ ] `sdk` — shared types, proto definitions

**Agent Workspace**
- [ ] `agent/agent-core` — binary entry point, orchestrator
- [ ] `agent/ebpf-collector` — eBPF programs and loader
- [ ] `agent/osquery-client` — OSQuery socket IPC client
- [ ] `agent/event-buffer` — local disk buffer (sled)
- [ ] `agent/fleet-client` — gRPC client to Fleet Server
- [ ] `agent/isolation` — IPTables-based network isolation

**Backend Services**
- [ ] `fleet-server` — gRPC fleet server (enrollment, streaming, C2)
- [ ] `kafka-pipeline` — event processor + normaliser + DB writer
- [ ] `rule-engine` — YARA scanning, MITRE mapping, alert generation
- [ ] `api-backend` — REST API + WebSocket for frontend

**Frontend**
- [ ] `frontend` — React/Vite/TypeScript dashboard

**Infrastructure**
- [ ] `infra` — Docker Compose, K8s manifests, Terraform, scripts

## Checklist
- [ ] Linked issue: closes #
- [ ] No secrets or credentials in code
- [ ] Tests added or updated
- [ ] `docker-compose up` tested locally
- [ ] Breaking changes documented in PR description

## How to verify
<!-- Steps for the reviewer -->
