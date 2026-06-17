#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
pub mod command_handler;
pub mod config;
pub mod orchestrator;
pub mod preflight;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use command_handler::CommandHandler;
use event_buffer::EventBuffer;
use fleet_client::FleetClient;
use fleet_client::types::{AgentEvent, EventType};
use osquery_client::OsqueryCollector;

/// Maximum number of consecutive `receive()` errors before the command
/// listener backs off to the maximum delay ceiling.
const CMD_MAX_BACKOFF_ERRORS: u32 = 8;

/// Starting backoff delay on a transport error (50 ms).
const CMD_BACKOFF_BASE_MS: u64 = 50;

/// Ceiling for exponential backoff (≈ 12.8 s).
const CMD_BACKOFF_CEILING_MS: u64 = 12_800;

/// How long the command-listener task sleeps between `try_receive` polls
/// when the channel is empty but healthy.  Keeps CPU near zero while still
/// allowing other tasks to acquire the Mutex within one tick (~5 ms).
const CMD_POLL_INTERVAL_MS: u64 = 5;

pub struct AgentCore {
    pub shutdown: CancellationToken,
    pub osquery: Arc<OsqueryCollector>,
    pub buffer: Arc<EventBuffer>,
    pub command_handler: Arc<CommandHandler>,
    pub fleet_client: Arc<tokio::sync::Mutex<FleetClient>>,
}

impl AgentCore {
    /// Start all background tasks and block until the shutdown token fires.
    ///
    /// # Parameters
    /// - `agent_uuid`: The node UUID assigned during enrollment.  Passed into
    ///   `OsqueryCollector::start` so that every `OsqueryResult` carries the
    ///   correct identity before it is serialised into an `AgentEvent`.
    pub async fn run(&self, agent_uuid: &str) -> Result<()> {
        let shutdown = self.shutdown.clone();

        // TASK 1: OSQuery Polling and Buffering Loop
        //
        // `OsqueryCollector::start` spawns its own internal scheduler task
        // and returns the *consumer* end of an MPSC channel (buffer = 100).
        // We own the Receiver here; the scheduler task holds the Sender.
        //
        // Backpressure: when `buffer.push()` is slow (SQLite lock contention)
        // the Tokio MPSC back-pressure naturally throttles the scheduler
        // because its sends will block once the channel fills to 100 items.
        // We do *not* need an additional semaphore here.
        let mut results_rx = self.osquery.start(agent_uuid).await;

        let buffer_task = self.buffer.clone();
        let agent_uuid_owned = agent_uuid.to_string();
        let shutdown_osq = shutdown.clone();

        let osquery_task = tokio::spawn(async move {
            info!(agent_uuid = %agent_uuid_owned, "OSQuery polling task started");

            loop {
                tokio::select! {
                    // Biased select: check shutdown first so we exit promptly
                    // even when events are arriving continuously.
                    biased;

                    _ = shutdown_osq.cancelled() => {
                        info!("OSQuery polling task: shutdown signal received, draining remaining events");

                        // Graceful drain
                        // Consume whatever is already sitting in the MPSC
                        // buffer so we do not lose events that the scheduler
                        // already produced before the token fired.
                        while let Ok(result) = results_rx.try_recv() {
                            if let Some(json) = encode_osquery_result(&result)
                                && let Err(e) = buffer_task.push(json).await {
                                    error!(error = %e, "Failed to buffer OSQuery result during shutdown drain");
                                }
                        }
                        break;
                    }

                    // Normal path: block until the next OsqueryResult arrives
                    // or the sender side drops (collector task exited).
                    result = results_rx.recv() => {
                        match result {
                            None => {
                                // Sender dropped: scheduler task exited unexpectedly.
                                warn!("OSQuery collector channel closed; polling task exiting");
                                break;
                            }
                            Some(osq_result) => {
                                // Serialisation
                                // `encode_osquery_result` handles both
                                // serde errors and clock-jump edge cases
                                // internally; it never panics.
                                let Some(event_json) = encode_osquery_result(&osq_result) else {
                                    // Error already logged inside helper
                                    continue;
                                };

                                debug!(
                                    query = %osq_result.query_name,
                                    rows  = osq_result.rows.len(),
                                    "Buffering OSQuery event"
                                );

                                // Buffer push
                                // `EventBuffer::push` offloads the actual
                                // SQLite INSERT onto `spawn_blocking`, so
                                // this await yields the async thread back to
                                // Tokio for the duration of the disk I/O.
                                // SQLite lock contention → the task waits
                                // inside spawn_blocking without consuming
                                // an async worker thread.
                                if let Err(e) = buffer_task.push(event_json).await {
                                    // Do NOT crash the loop.  Log and continue
                                    // so that a transient WAL-lock burst
                                    // does not drop the entire stream.
                                    error!(
                                        query = %osq_result.query_name,
                                        error = %e,
                                        "Failed to push OSQuery event to buffer; event dropped"
                                    );
                                }
                            }
                        }
                    }
                }
            }

            info!("OSQuery polling task exited cleanly");
        });

        // TASK 2: Command Listener Loop
        //
        // LOCK CONTENTION DESIGN — why we do NOT do:
        //
        //   loop { let mut c = fleet.lock().await; c.receive().await; }
        //
        // That pattern holds the Mutex for the *entire* duration of the
        // blocking `recv()` inside `receive()`, which can be seconds or
        // minutes between commands.  While the Mutex is held, the heartbeat
        // task and the event-drain task cannot acquire it, causing starvation.
        //
        // SOLUTION — cooperative non-blocking poll:
        //
        //   1. Lock the Mutex.
        //   2. Call `try_receive()` — returns immediately (no `.await`).
        //   3. Unlock the Mutex (lock guard drops at end of block).
        //   4. If a message was ready → process it (no lock needed).
        //   5. If the channel was empty → sleep CMD_POLL_INTERVAL_MS (5 ms)
        //      then go to step 1.  During that sleep every other task can
        //      freely acquire the lock.
        //
        // This keeps peak latency < 5 ms for commands while consuming
        // essentially zero CPU when no commands are arriving.
        //
        // ALTERNATIVE: Extract `inbound_rx` out of FleetClient into its own
        // `Arc<Mutex<Receiver<ServerMessage>>>` so the command listener can
        // hold that lock (cheap) without blocking the send path.  That is the
        // architecturally cleanest solution but requires a larger refactor;
        // it is tracked as a follow-up TODO below.
        //
        // TODO(follow-up): Split FleetClient into `FleetSink` (outbound_tx,
        // methods: send/enroll/heartbeat/send_events) and `FleetSource`
        // (inbound_rx, method: receive) so the two halves can be locked
        // independently, eliminating the poll interval entirely.

        let cmd_handler = self.command_handler.clone();
        let fleet = self.fleet_client.clone();
        let shutdown_cmd = shutdown.clone();

        let command_task = tokio::spawn(async move {
            info!("Command listener task started");

            // Consecutive transport error counter for backoff.
            let mut consecutive_errors: u32 = 0;

            loop {
                // Check shutdown first (biased)
                if shutdown_cmd.is_cancelled() {
                    info!("Command listener task: shutdown signal received, exiting");
                    break;
                }

                // Non-blocking poll (lock held < 1 µs)
                //
                // We acquire the lock, call try_receive (synchronous, no
                // await), then immediately drop the guard.  The total time
                // the Mutex is held equals one MPSC `try_recv` call which
                // is O(1) and lock-free on the happy path.
                let poll_result = {
                    let mut client = fleet.lock().await;
                    client.try_receive()
                    // `client` guard drops here — Mutex released.
                };

                match poll_result {
                    // No message yet
                    Ok(None) => {
                        // Reset error counter: the transport is healthy.
                        consecutive_errors = 0;

                        // Yield to the scheduler for one poll interval.
                        // Other tasks (heartbeat, drain) can acquire the
                        // fleet_client Mutex during this sleep.
                        tokio::select! {
                            biased;
                            _ = shutdown_cmd.cancelled() => {
                                info!("Command listener task: shutdown during poll sleep");
                                break;
                            }
                            _ = tokio::time::sleep(
                                    Duration::from_millis(CMD_POLL_INTERVAL_MS)) => {}
                        }
                    }

                    // Message received
                    Ok(Some(msg)) => {
                        consecutive_errors = 0;

                        debug!(command = ?msg.command, "Received ServerCommand");

                        // Dispatch to CommandHandler.  The handler is
                        // Arc-wrapped and does not need the fleet_client
                        // lock, so we process the command without holding
                        // any mutex.
                        match cmd_handler.handle(msg).await {
                            Ok(response) => {
                                debug!(response = ?response, "Command handled successfully");
                            }
                            Err(e) => {
                                warn!(error = %e, "CommandHandler returned error; continuing");
                            }
                        }
                    }

                    // Transport / channel error
                    Err(e) => {
                        consecutive_errors = consecutive_errors.saturating_add(1);

                        // Exponential back-off: 50 ms → 100 → 200 → … → 12 800 ms
                        let backoff_ms = (CMD_BACKOFF_BASE_MS
                            * 2u64.pow(consecutive_errors.min(CMD_MAX_BACKOFF_ERRORS)))
                        .min(CMD_BACKOFF_CEILING_MS);

                        error!(
                            error            = %e,
                            consecutive      = consecutive_errors,
                            backoff_ms       = backoff_ms,
                            "Command listener: transport error; backing off"
                        );

                        // Attempt to reconnect to the fleet server using the stored token
                        {
                            let mut client = fleet.lock().await;
                            let token = client.token().map(|s| s.to_string());
                            if let Err(reconnect_err) = client.connect(token.as_deref()).await {
                                warn!(error = %reconnect_err, "Failed to reconnect to fleet server");
                            } else {
                                info!("Successfully re-established connection to fleet server");
                                consecutive_errors = 0;
                            }
                        }

                        // Respect shutdown even during backoff sleep.
                        tokio::select! {
                            biased;
                            _ = shutdown_cmd.cancelled() => {
                                info!("Command listener task: shutdown during error backoff");
                                break;
                            }
                            _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => {}
                        }
                    }
                }
            }

            info!("Command listener task exited cleanly");
        });

        // Wait for shutdown signal
        shutdown.cancelled().await;
        info!("AgentCore: shutdown token fired, awaiting task cleanup");

        // Give tasks up to 5 s to finish their graceful drain / backoff
        // sleep before we hard-abort them.  In practice both tasks react
        // to the cancellation token within one poll cycle (≤ 5 ms for the
        // command task, ≤ one SQLite round-trip for the osquery task).
        let grace = Duration::from_secs(5);
        let _ = tokio::time::timeout(grace, osquery_task).await;
        let _ = tokio::time::timeout(grace, command_task).await;

        info!("AgentCore: all tasks exited, shutdown complete");
        Ok(())
    }
}

// Helper — OsqueryResult to AgentEvent JSON

/// Converts an `OsqueryResult` into a JSON string suitable for `EventBuffer::push`.
///
/// Returns `None` (and logs a warning) if serialisation fails so the caller
/// can continue without crashing the polling loop.
///
/// Clock-jump robustness: `OsqueryResult::timestamp_ns` is produced by the
/// scheduler using `chrono::Utc::now().timestamp_nanos_opt()`.  If the
/// system clock jumps backwards (NTP step, VM snapshot restore), the
/// timestamp will appear to go backwards.  We do *not* try to correct this
/// here — doing so correctly requires a monotonic clock that maps to wall
/// time, which is scheduler-level logic.  Instead we propagate whatever the
/// scheduler produced and let the server-side pipeline de-duplicate by
/// `sequence_id` (UUID v4) rather than timestamp ordering.
fn encode_osquery_result(result: &osquery_client::types::OsqueryResult) -> Option<String> {
    // Serialise the full OsqueryResult as the event payload.
    let payload = match serde_json::to_value(result) {
        Ok(v) => v,
        Err(e) => {
            warn!(
                query = %result.query_name,
                error = %e,
                "Failed to serialise OsqueryResult payload; event dropped"
            );
            return None;
        }
    };

    let event = AgentEvent {
        node_id: result.agent_uuid.clone(),
        event_type: EventType::Osquery as i32,
        payload,
        // Pass through the scheduler's nanosecond timestamp verbatim.
        // The scheduler already checked `timestamp_nanos_opt()`; if it
        // returned None it will have substituted 0, which is detectable
        // by the server as a sentinel value.
        timestamp_ns: result.timestamp_ns,
        sequence_id: Uuid::new_v4().to_string(),
    };

    match serde_json::to_string(&event) {
        Ok(json) => Some(json),
        Err(e) => {
            warn!(
                query = %result.query_name,
                error = %e,
                "Failed to serialize AgentEvent to JSON; event dropped"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edr_sdk::proto::fleet::{
        AckCommand, ConfigUpdateCommand, ServerCommand, server_command::Command,
    };
    use osquery_client::types::{ColumnEntry, OsqueryResult, OsqueryResultRow, ResultAction};
    use std::path::PathBuf;

    #[test]
    fn test_osquery_result_encoding_happy_path() {
        let result = OsqueryResult {
            query_name: "test_query".to_string(),
            agent_uuid: "test-agent-123".to_string(),
            timestamp_ns: 1718660000000000000,
            rows: vec![OsqueryResultRow {
                columns: vec![ColumnEntry {
                    name: "col1".to_string(),
                    value: "val1".to_string(),
                }],
            }],
            action: ResultAction::Snapshot,
        };

        let encoded = encode_osquery_result(&result);
        assert!(encoded.is_some());

        let json_str = encoded.unwrap();
        let event: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(event["node_id"], "test-agent-123");
        assert_eq!(event["event_type"], EventType::Osquery as i32);
        assert_eq!(event["timestamp_ns"], 1718660000000000000i64);

        let payload = &event["payload"];
        assert_eq!(payload["query_name"], "test_query");
        assert_eq!(payload["action"], "SNAPSHOT");
    }

    #[tokio::test]
    async fn test_command_handling_ack() {
        let collector = OsqueryCollector::new(osquery_client::OsqueryConfig {
            socket_path: PathBuf::from("/tmp/osquery-test.em"),
            db_path: PathBuf::from("/tmp/events-test.db"),
        })
        .await
        .unwrap();

        let handler = CommandHandler {
            osquery: Arc::new(collector),
            isolation: isolation::IsolationManager::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                50051,
            ),
        };

        let cmd = ServerCommand {
            command: Some(Command::Ack(AckCommand {
                sequence_id: "test-seq-123".to_string(),
            })),
        };

        let res = handler.handle(cmd).await;
        assert!(res.is_ok());
        let val = res.unwrap();
        assert_eq!(val["status"], "acked");
    }

    #[tokio::test]
    async fn test_command_handling_config_update() {
        let collector = OsqueryCollector::new(osquery_client::OsqueryConfig {
            socket_path: PathBuf::from("/tmp/osquery-test.em"),
            db_path: PathBuf::from("/tmp/events-test.db"),
        })
        .await
        .unwrap();

        let handler = CommandHandler {
            osquery: Arc::new(collector),
            isolation: isolation::IsolationManager::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                50051,
            ),
        };

        let cmd = ServerCommand {
            command: Some(Command::ConfigUpdate(ConfigUpdateCommand {
                config: Some(edr_sdk::proto::fleet::AgentConfig {
                    osquery_schedule: vec![],
                    heartbeat_interval_secs: 60,
                    batch_size: 100,
                }),
            })),
        };

        let res = handler.handle(cmd).await;
        assert!(res.is_ok());
        let val = res.unwrap();
        assert_eq!(val["status"], "config_updated");
    }

    #[tokio::test]
    async fn test_shutdown_signal_cancellation() {
        let token = CancellationToken::new();
        let token_clone = token.clone();

        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = token_clone.cancelled() => {
                    true
                }
                _ = tokio::time::sleep(Duration::from_millis(500)) => {
                    false
                }
            }
        });

        token.cancel();
        let result = handle.await.unwrap();
        assert!(result);
    }
}
