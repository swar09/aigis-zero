#!/bin/bash
set -e

echo "Building agent Docker image (this will install Rust & OSQuery)..."
docker build -t edr-agent-dev -f agent/Dockerfile .

echo "Running agent in Docker container..."
# Capabilities required for osquery audit event collection:
#   AUDIT_CONTROL — set/read audit configuration, control audit daemon
#   AUDIT_READ    — read audit log messages via netlink
#   SYS_PTRACE    — process introspection (process_open_files, process_envs, etc.)
#   NET_ADMIN     — network interface and socket inspection
# -v mounts the current project dir to /workspace
# -w sets working directory to /workspace
# EDR_AGENT_CONFIG points to agent.toml in the workspace
docker run --rm -it \
  --name edr-agent \
  -v "$(pwd)":/workspace \
  -w /workspace \
  -e EDR_AGENT_CONFIG=/workspace/agent.toml \
  --add-host=host.docker.internal:host-gateway \
  --cap-add AUDIT_CONTROL \
  --cap-add AUDIT_READ \
  --cap-add SYS_PTRACE \
  --cap-add NET_ADMIN \
  edr-agent-dev \
  bash -c "cargo run -p agent-bin"
