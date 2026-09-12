# syntax=docker/dockerfile:1.7
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    libprotobuf-dev \
    cmake \
    g++ \
    make \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/workspace/target \
    cargo build --bin edr-agent --release && \
    cp /workspace/target/release/edr-agent /usr/sbin/aigis-zero

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    systemd \
    systemd-sysv \
    nftables \
    curl \
    gnupg \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /etc/apt/keyrings && \
    curl -fsSL https://pkg.osquery.io/deb/osquery_key.pub | gpg --dearmor -o /etc/apt/keyrings/osquery.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/osquery.gpg] https://pkg.osquery.io/deb deb main" > /etc/apt/sources.list.d/osquery.list && \
    apt-get update && apt-get install -y --no-install-recommends osquery && \
    rm -rf /var/lib/apt/lists/*

RUN systemctl mask systemd-udevd.service \
                   systemd-udevd-kernel.socket \
                   systemd-udevd-control.socket \
                   systemd-modules-load.service \
                   sys-kernel-debug.mount \
                   sys-kernel-tracing.mount \
                   systemd-journald-audit.socket

COPY --from=builder --chmod=0755 /usr/sbin/aigis-zero /usr/sbin/aigis-zero

RUN mkdir -p /etc/aigis-zero /var/lib/aigis-zero /var/log/aigis-zero && \
    chmod 700 /etc/aigis-zero /var/lib/aigis-zero /var/log/aigis-zero

COPY agent/agent.toml /etc/aigis-zero/config.toml
RUN chmod 600 /etc/aigis-zero/config.toml

COPY agent/osquery/osquery.conf /etc/osquery/
COPY agent/osquery/osquery.flags /etc/osquery/
RUN chmod 600 /etc/osquery/osquery.conf /etc/osquery/osquery.flags

COPY agent/sysctl/60-aigis-zero.conf /etc/sysctl.d/
COPY agent/limits/99-aigis-zero.conf /etc/security/limits.d/

COPY agent/systemd/aigis-zero.service /etc/systemd/system/
RUN mkdir -p /etc/systemd/system/osqueryd.service.d
COPY agent/systemd/osqueryd.service.d/aigis-zero.conf /etc/systemd/system/osqueryd.service.d/

RUN systemctl enable osqueryd.service && \
    systemctl enable aigis-zero.service

STOPSIGNAL SIGRTMIN+3
CMD ["/lib/systemd/systemd"]

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
  CMD systemctl is-active --quiet aigis-zero || exit 1
