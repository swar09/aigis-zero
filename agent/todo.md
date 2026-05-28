# EDR Agent Stubs and TODOs

This file tracks all the stubs, placeholders, mock implementations, and incomplete parts of the codebase within the `/agent` directory.

---

## 1. Entirely Empty/Stub Crates

The following crates are defined in the workspace but contain no functional logic:

### 🔴 **ebpf-collector**
* **Location**: `crates/ebpf-collector/`
* **Current State**:
  * [lib.rs](file:///Users/swar/C/R/oss/project-edr/agent/crates/ebpf-collector/src/lib.rs) is empty except for a comment: `// eBPF collector — kernel-level event telemetry.`
  * The `bpf/` directory is completely empty.
* **To Do**: Implement eBPF kernel-level event telemetry and hook it up to the agent.

### 🔴 **isolation**
* **Location**: `crates/isolation/`
* **Current State**:
  * [lib.rs](file:///Users/swar/C/R/oss/project-edr/agent/crates/isolation/src/lib.rs) is empty except for a comment: `// Isolation — network quarantine via iptables.`
* **To Do**: Implement network quarantine mechanisms using `iptables` or similar platforms/tools.

---

## 2. Agent Core Orchestrator

### 🟡 **Mock System Metadata**
* **Location**: [orchestrator.rs:L37-42](file:///Users/swar/C/R/oss/project-edr/agent/crates/agent-core/src/orchestrator.rs#L37-42)
* **Stub**:
  ```rust
  let req = RegisterRequest {
      hostname: "mock-hostname".to_string(),
      os_version: "mock-os".to_string(),
      agent_version: "0.1.0".to_string(),
      machine_id: "mock-machine-id".to_string(),
  };
  ```
* **To Do**: Query the host OS for actual hostname, OS version, EDR agent version, and unique machine/hardware ID.

### 🟡 **Enrollment Fallback & Local Buffer Routing**
* **Location**: [orchestrator.rs:L50](file:///Users/swar/C/R/oss/project-edr/agent/crates/agent-core/src/orchestrator.rs#L50)
* **Stub**:
  ```rust
  // We would continue and buffer locally, but stub for now.
  ```
* **To Do**: When the enrollment fails or Fleet Server is offline, route collected telemetry to the local SQLite-backed `EventBuffer` to ensure no data is lost, and retry enrollment/connection in the background.

### 🟡 **Osquery Collector Spawning**
* **Location**: [orchestrator.rs:L54-57](file:///Users/swar/C/R/oss/project-edr/agent/crates/agent-core/src/orchestrator.rs#L54-57)
* **Stub**:
  ```rust
  // Stub: We would also create OsqueryCollector and route events here.
  
  // Keeping main alive
  tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
  ```
* **To Do**: Instantiate `OsqueryCollector`, start the scheduler, start the query loops, and pipe the output to the `FleetClient` event stream or the local buffer. Remove the temporary `sleep` block.

---

## 3. Osquery Client

### 🟡 **Osquery Thrift Unix Socket Client**
* **Location**: [client.rs:L16-44](file:///Users/swar/C/R/oss/project-edr/agent/crates/osquery-client/src/client.rs#L16-44)
* **Stub**:
  ```rust
  pub async fn query(&mut self, sql: &str) -> Result<QueryResponse> {
      // Stub for now. Will require Thrift serialization over UnixStream.
      tracing::debug!("Executing query: {}", sql);
      Ok(QueryResponse { ... })
  }
  ```
* **To Do**: Implement full Apache Thrift serialization and deserialization over `UnixStream` to communicate with the osquery daemon socket instead of returning mocked empty results. Implement `get_query_columns`, `ping`, and `reconnect` logic.

### 🟡 **Query Scheduler Upsert Cleanup**
* **Location**: [scheduler.rs:L70](file:///Users/swar/C/R/oss/project-edr/agent/crates/osquery-client/src/scheduler.rs#L70)
* **Stub**:
  ```rust
  // Note: For now, we just upsert. If full replacement is needed, we'd delete missing ones.
  ```
* **To Do**: When updating query schedules, perform a deletion sync to remove any scheduled queries that are no longer present in the updated fleet config configuration.

### 🟡 **Scheduler Execution Loop**
* **Location**: [scheduler.rs:L76-80](file:///Users/swar/C/R/oss/project-edr/agent/crates/osquery-client/src/scheduler.rs#L76-80)
* **Stub**:
  ```rust
  pub async fn run(self, _tx: mpsc::Sender<OsqueryResult>) {
      // Implement the actual loop here.
      // It will spawn tasks for each query, using OsqueryClient::query, and tracking diffs.
      // Left as stub for now until client is implemented.
  }
  ```
* **To Do**: Write the runtime scheduler loop that runs periodically, schedules/spawns tasks for each query based on their intervals, tracks diffs between runs, and sends differential/snapshot changes to the channel.

---

## 4. Fleet Client

### 🟡 **Mock Enrollment Service**
* **Location**: [enrollment.rs:L9-18](file:///Users/swar/C/R/oss/project-edr/agent/crates/fleet-client/src/enrollment.rs#L9-18)
* **Stub**:
  ```rust
  // Stub: In a real implementation we would make a gRPC call here using the
  // manually constructed or generated FleetServiceClient over the given channel.
  // For now, we mock the response.
  ```
* **To Do**: Replace the mocked UUID/JWT response generation with a real gRPC enrollment call.

### 🟡 **Mock Heartbeat Manager**
* **Location**: [heartbeat.rs:L18-26](file:///Users/swar/C/R/oss/project-edr/agent/crates/fleet-client/src/heartbeat.rs#L18-26)
* **Stub**:
  ```rust
  // Stub: In a real implementation we would make a gRPC call here using the
  // manually constructed or generated FleetServiceClient over the given channel.
  tokio::spawn(async move {
      loop {
          interval.tick().await;
          tracing::debug!("Sending heartbeat for node: {}", node_id);
          // Send HeartbeatRequest { node_id, status: "healthy", events_buffered: 0 }
      }
  });
  ```
* **To Do**: Implement gRPC client heartbeat dispatching, tracking the number of buffered events in the local database, and reacting to server status checks.

### 🟡 **Mock Bidirectional Event Stream**
* **Location**: [stream.rs:L16-24](file:///Users/swar/C/R/oss/project-edr/agent/crates/fleet-client/src/stream.rs#L16-24)
* **Stub**:
  ```rust
  // Stub: In a real implementation we would open a bidirectional stream here.
  // For now, just drain events_rx and log them.
  ```
* **To Do**: Establish a real gRPC bidirectional stream to stream telemetry up and dynamically receive downstream commands (e.g. Isolation, Config Updates, Acks) in real time.

---

## 5. Mock Fleet Server

### 🟡 **Mock Fleet Server Listening Stub**
* **Location**: [main.rs:L116-121](file:///Users/swar/C/R/oss/project-edr/agent/tools/mock-fleet-server/src/main.rs#L116-121)
* **Stub**:
  ```rust
  tracing::info!("Mock Fleet Server listening on 0.0.0.0:50051");
  // Implement actual tonic service when compiling the fleet proto or manually wrapping bytes.
  // Since we're doing manual bytes on the agent, we need to match the gRPC paths here or wait
  // for proper proto codegen in a later step.
  // For now, this is a placeholder that compiles.
  loop { tokio::time::sleep(tokio::time::Duration::from_secs(60)).await; }
  ```
* **To Do**: Replace the simple sleep loop with a fully functioning Tonic gRPC server definition that accepts `RegisterRequest`, handles dynamic config updates, and responds to heartbeats/streams.
