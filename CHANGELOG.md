# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - 2026-06-03
**Author:** Swar (@swar09)

### Added
- **Fleet Server Backend**: Completed the core architecture and domain logic for the `fleet-server` binary.
- **Database Layer (`postgres-interface`)**: Added PostgreSQL persistence implementation using `sqlx`, ensuring strict compile-time verification of all SQL queries.
- **Node Enrollment (`node-enrollment`)**: Implemented endpoint registration logic with 24-hour secure JWT token generation.
- **Health Tracking (`health-tracker`)**: Added time-series heartbeat recording for nodes.
- **Infrastructure**: Added `docker-compose.yml`, `Dockerfile`, and SQL migrations (`nodes`, `enrollment_events`, `node_health`, and seed data) to spin up the local development database on port 5433.
- **Configuration**: Integrated `dotenvy` for robust `.env` file parsing across the workspace. Added a `.env.example` file.
- **CI Tooling**: Created a `local-ci.sh` script to streamline local CI checks (`cargo fmt`, `clippy`, `test`).

### Changed
- **Status Management (Security)**: `operator_status` is now strictly separated from `agent_status`. Heartbeats can no longer overwrite operator-assigned states.
- **Database Updates**: Transitioned from using PostgreSQL `xmax` system columns for UPSERT detection to explicit `SELECT FOR UPDATE` patterns to guarantee query reliability and pass static SQL validation.

### Fixed
- **Clock Drift Vulnerability**: Fixed a silent clock failure bug in the JWT signing process that could occur if system clocks drifted prior to the UNIX epoch.
- **Missing Dependencies**: Added missing `sqlx` and `tonic` dependencies in domain crates.
- **Zero-Warning CI**: Resolved various `unused_import`, `dead_code`, and `clippy::cast_possible_truncation` warnings to strictly adhere to the project's zero-warning CI policy.

### Security / Warnings
- **Database URL Compilation Requirement**: Because `sqlx` is used for compile-time query verification, the `DATABASE_URL` environment variable must be exported and point to a live, fully-migrated database in order to compile the project (`cargo check`, `cargo test`, etc.).
