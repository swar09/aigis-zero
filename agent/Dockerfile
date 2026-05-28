FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies, GPG, and repository tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    gnupg \
    wget \
    net-tools \
    iptables \
    ipset \
    netfilter-persistent \
    git \
    openssh-server \
    sudo \
    # Required for osquery audit event collection
    auditd \
    libaudit1 \
    iproute2 \
    && rm -rf /var/lib/apt/lists/*

# Add official OSQuery repository and install OSQuery
RUN curl -L https://pkg.osquery.io/deb/pubkey.gpg | gpg --dearmor -o /usr/share/keyrings/osquery.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/osquery.gpg] https://pkg.osquery.io/deb deb main" | tee /etc/apt/sources.list.d/osquery.list && \
    apt-get update && \
    apt-get install -y osquery protobuf-compiler build-essential pkg-config libssl-dev cmake && \
    rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
ENV CARGO_TARGET_DIR="/tmp/target"

# Ensure required directories exist for osquery socket and configurations
RUN mkdir -p /var/osquery /etc/osquery /var/log/osquery

# Setup osquery.flags — events and audit enabled.
# The container must be run with --cap-add AUDIT_CONTROL --cap-add AUDIT_READ
# (or --privileged) for the audit subsystem to function.
RUN printf -- \
'--disable_events=false\n\
--disable_audit=false\n\
--audit_allow_process_events=true\n\
--audit_allow_sockets=true\n\
--audit_allow_config=true\n\
--audit_persist=true\n\
--events_max=50000\n\
--socket=/var/osquery/osquery.em\n\
--host_identifier=hostname\n\
--schedule_splay_percent=10\n\
' > /etc/osquery/osquery.flags

# Setup basic osquery.conf
RUN printf '{\n\
  "options": {\n\
    "disable_events": "false",\n\
    "disable_audit": "false",\n\
    "audit_allow_process_events": "true",\n\
    "audit_allow_sockets": "true",\n\
    "audit_allow_config": "true",\n\
    "audit_persist": "true",\n\
    "events_max": "50000",\n\
    "host_identifier": "hostname",\n\
    "schedule_splay_percent": "10"\n\
  },\n\
  "schedule": {\n\
    "running_processes": {\n\
      "query": "SELECT pid, name, path, cmdline, uid, parent FROM processes;",\n\
      "interval": 30\n\
    }\n\
  }\n\
}' > /etc/osquery/osquery.conf

# Expose directory for volume mounting if needed
VOLUME ["/var/osquery", "/etc/osquery"]

# Setup startup entrypoint script to launch osqueryd in background and start shell
RUN printf '#!/bin/bash\n\
# Start osqueryd daemon in background\n\
echo "Starting osqueryd..."\n\
osqueryd --flagfile=/etc/osquery/osquery.flags --config_path=/etc/osquery/osquery.conf --verbose &\n\
\n\
# Execute passed command or default to bash\n\
exec "$@"\n\
' > /usr/local/bin/entrypoint.sh && chmod +x /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["/bin/bash"]
