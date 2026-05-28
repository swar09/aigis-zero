#!/bin/bash
set -e

echo "Building agent Docker image (this will install Rust & OSQuery)..."
docker build -t edr-agent-dev -f agent/Dockerfile .

echo "Running agent in Docker container..."
# -v mounts the current project dir to /workspace
# -w sets working directory to /workspace
# --privileged or CAP_AUDIT_CONTROL etc might be needed for osquery audit, but let's stick to simple run for now
# We pass EDR_AGENT_CONFIG to use the agent.toml in the workspace
docker run --rm -it \
  --name edr-agent \
  -v "$(pwd)":/workspace \
  -w /workspace \
  -e EDR_AGENT_CONFIG=/workspace/agent.toml \
  --add-host=host.docker.internal:host-gateway \
  edr-agent-dev \
  bash -c "cargo run -p agent-bin"
