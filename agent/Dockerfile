# ==========================================
# Stage 1: Build the Aigis-Zero Agent
# ==========================================
FROM rust:1.85-slim-bookworm AS builder

# Install build dependencies for compiling static musl binary + Protobuf
RUN apt-get update && apt-get install -y \
    musl-tools \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /workspace
# Copy the entire workspace (assumes docker build is run from the root directory)
COPY . .

# Build the edr-agent statically
RUN cargo build --bin edr-agent --target x86_64-unknown-linux-musl --release

# ==========================================
# Stage 2: Runtime Image with Systemd + Osquery
# ==========================================
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Install systemd, nftables, and utilities
RUN apt-get update && apt-get install -y \
    systemd \
    systemd-sysv \
    nftables \
    curl \
    gnupg \
    software-properties-common \
    && rm -rf /var/lib/apt/lists/*

# Install osquery
RUN curl -L https://pkg.osquery.io/deb/osquery_key.pub | apt-key add - && \
    add-apt-repository 'deb [arch=amd64] https://pkg.osquery.io/deb deb main' && \
    apt-get update && apt-get install -y osquery && \
    rm -rf /var/lib/apt/lists/*

# Mask unnecessary systemd services inside a container
RUN systemctl mask systemd-udevd.service \
                   systemd-udevd-kernel.socket \
                   systemd-udevd-control.socket \
                   systemd-modules-load.service \
                   sys-kernel-debug.mount \
                   sys-kernel-tracing.mount \
                   systemd-journald-audit.socket

# 1. Install agent binary
COPY --from=builder /workspace/target/x86_64-unknown-linux-musl/release/edr-agent /usr/sbin/aigis-zero
RUN chmod 755 /usr/sbin/aigis-zero

# 2. Setup directories and permissions
RUN mkdir -p /etc/aigis-zero /var/lib/aigis-zero /var/log/aigis-zero && \
    chmod 700 /etc/aigis-zero /var/lib/aigis-zero /var/log/aigis-zero

# 3. Copy agent config
COPY agent/agent.toml /etc/aigis-zero/config.toml
RUN chmod 600 /etc/aigis-zero/config.toml

# 4. Copy osquery config
COPY agent/osquery/osquery.conf /etc/osquery/
COPY agent/osquery/osquery.flags /etc/osquery/
RUN chmod 600 /etc/osquery/osquery.conf /etc/osquery/osquery.flags

# 5. Copy tunables
COPY agent/sysctl/60-aigis-zero.conf /etc/sysctl.d/
COPY agent/limits/99-aigis-zero.conf /etc/security/limits.d/

# 6. Copy and enable systemd services
COPY agent/systemd/aigis-zero.service /etc/systemd/system/
RUN mkdir -p /etc/systemd/system/osqueryd.service.d
COPY agent/systemd/osqueryd.service.d/aigis-zero.conf /etc/systemd/system/osqueryd.service.d/

RUN systemctl enable osqueryd.service && \
    systemctl enable aigis-zero.service

# Use systemd as the entrypoint
STOPSIGNAL SIGRTMIN+3
CMD ["/lib/systemd/systemd"]

# ==========================================
# HOW TO BUILD AND RUN:
# 
# 1. Build (from the workspace root directory, NOT inside /agent):
#    docker build -t aigis-zero-agent -f agent/Dockerfile .
#
# 2. Run with elevated privileges (required for eBPF, systemd, and nftables):
#    docker run -d --name agent-node \
#      --privileged \
#      --cgroupns=host \
#      -v /sys/fs/cgroup:/sys/fs/cgroup:rw \
#      aigis-zero-agent
#
# 3. Check logs:
#    docker exec -it agent-node journalctl -u aigis-zero -f
# ==========================================
