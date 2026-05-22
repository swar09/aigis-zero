# Infra — Implementation Timeline

> **Phase**: 0 (Foundation) + Phase 9 (Hardening)
> **Priority**: 🔴 Critical — required for local dev and all integration testing
> **Estimated Duration**: 2–3 days initial, ongoing through project

---

## Overview

Infrastructure definitions for local development (Docker Compose) and production deployment (K8s, Terraform). The docker-compose stack must be running before any service integration testing can begin.

---

## PR Plan

### PR #1 — Docker Compose local dev stack
**Branch**: `feat/docker-compose`
**Duration**: 1 day
**Files**:
- `docker-compose.yml` ← already scaffolded
- `docker-compose.dev.yml` ← dev overrides (debug ports, volumes)
- `.env.example` ← template for required environment variables
- `scripts/init-topics.sh` ← Kafka topic creation script (standalone)

**Tasks**:
- [ ] Verify `docker-compose.yml` syntax and service dependencies
- [ ] Create `.env.example` with all required variables (`POSTGRES_PASSWORD`, etc.)
- [ ] Create `docker-compose.dev.yml` with dev-specific overrides (extra ports, restart policies)
- [ ] Write `scripts/init-topics.sh` for manual topic creation
- [ ] Test `docker-compose up -d` — all services start healthy
- [ ] Verify Kafka UI accessible at `localhost:8090`
- [ ] Verify all 3 PostgreSQL instances accept connections
- [ ] Verify Kafka topics are created by `kafka-init` service
- [ ] Document in `README.md`

**Acceptance Criteria**:
- `docker-compose up -d && docker-compose ps` shows all services healthy
- Kafka topics (`edr.events.raw`, `edr.events.norm`, `edr.alerts`, `edr.health`) exist
- All 3 PostgreSQL databases accept connections

---

### PR #2 — Database migration scripts
**Branch**: `feat/db-migrations`
**Duration**: 1 day
**Depends on**: PR #1

**Files**:
- `scripts/run-migrations.sh` ← applies SQL migrations to all DBs
- `scripts/sql/nodes_001.sql` ← node registry tables
- `scripts/sql/logs_001.sql` ← event log tables (partitioned)
- `scripts/sql/alerts_001.sql` ← alert tables

**Tasks**:
- [ ] Create SQL migration for `edr_nodes` DB (nodes, agent_configs, pending_commands)
- [ ] Create SQL migration for `edr_logs` DB (events table with partitioning)
- [ ] Create SQL migration for `edr_alerts` DB (alerts table with indexes)
- [ ] Write `run-migrations.sh` that runs psql against each DB
- [ ] Test migrations are idempotent (can run twice without error)
- [ ] Add index creation for all query patterns

**Acceptance Criteria**:
- `./scripts/run-migrations.sh` completes without errors
- All tables, indexes, and constraints exist

---

### PR #3 — Service Dockerfiles integration
**Branch**: `feat/service-dockerfiles`
**Duration**: 0.5 day
**Depends on**: After services have basic `main.rs` (Phase 1+)

**Tasks**:
- [ ] Add service definitions to `docker-compose.yml` for fleet-server, kafka-pipeline, rule-engine, api-backend
- [ ] Configure inter-service networking
- [ ] Add health checks for Rust services
- [ ] Add frontend nginx service
- [ ] Test full stack `docker-compose up`

---

### PR #4 — Kubernetes manifests (Phase 9)
**Branch**: `feat/k8s-manifests`
**Duration**: 2 days
**Depends on**: All services functional

**Files**:
- `k8s/manifests/namespace.yaml`
- `k8s/manifests/fleet-server-deployment.yaml`
- `k8s/manifests/kafka-pipeline-deployment.yaml`
- `k8s/manifests/rule-engine-deployment.yaml`
- `k8s/manifests/api-backend-deployment.yaml`
- `k8s/manifests/frontend-deployment.yaml`
- `k8s/manifests/services.yaml`
- `k8s/manifests/configmaps.yaml`
- `k8s/manifests/secrets.yaml`

**Tasks**:
- [ ] Create namespace definition
- [ ] Write Deployment + Service for each Rust service
- [ ] Write ConfigMaps for non-secret config
- [ ] Write Secret templates
- [ ] Add HPA (Horizontal Pod Autoscaler) for fleet-server and kafka-pipeline
- [ ] Add PersistentVolumeClaims for PostgreSQL and Kafka
- [ ] Test with `kubectl apply -f k8s/manifests/`

---

### PR #5 — Observability stack (Phase 9)
**Branch**: `feat/observability`
**Duration**: 1.5 days

**Tasks**:
- [ ] Add Prometheus to docker-compose
- [ ] Add Grafana to docker-compose with pre-configured datasource
- [ ] Create Grafana dashboards (event throughput, alert rate, node health)
- [ ] Document metrics endpoints for each service
- [ ] Add runbooks in `docs/` directory
