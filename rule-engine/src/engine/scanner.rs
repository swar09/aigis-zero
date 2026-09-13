use std::sync::Arc;

use crate::{
    engine::{registry::RegistryHolder, transform},
    error::AppError,
    models::{Alert, TelemetryEvent},
};

fn scan_object(obj: &serde_json::Map<String, serde_json::Value>, buffer: &mut Vec<u8>) {
    for key in [
        "cmdline",
        "cmd",
        "command_line",
        "path",
        "target_path",
        "parent_path",
        "cwd",
        "name",
        "remote_address",
        "local_address",
        "user",
        "description",
        "args",
        "syscall",
        "action",
    ] {
        if let Some(serde_json::Value::String(val)) = obj.get(key) {
            buffer.extend_from_slice(val.as_bytes());
            buffer.push(b'\n');
        }
    }
}

/// Extracts string fields from a telemetry event payload that are scannable.
/// Avoids scanning JSON structural characters (braces, quotes, commas) that interfere with string adjacency.
pub fn extract_scannable_buffer(event: &TelemetryEvent) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(1024);

    if let Some(obj) = event.payload.as_object() {
        scan_object(obj, &mut buffer);
        if let Some(rows) = obj.get("rows").and_then(|r| r.as_array()) {
            for row in rows {
                if let Some(row_obj) = row.as_object() {
                    scan_object(row_obj, &mut buffer);
                    if let Some(cols) = row_obj.get("columns").and_then(|c| c.as_array()) {
                        for col in cols {
                            if let Some(col_obj) = col.as_object()
                                && let Some(serde_json::Value::String(val)) = col_obj.get("value")
                            {
                                buffer.extend_from_slice(val.as_bytes());
                                buffer.push(b'\n');
                            }
                        }
                    }
                }
            }
        }
    }

    if buffer.is_empty()
        && let Ok(bytes) = serde_json::to_vec(&event.payload)
    {
        buffer = bytes;
    }

    buffer
}

pub struct YaraScannerEngine {
    registry: Arc<RegistryHolder>,
}

impl YaraScannerEngine {
    pub fn new(registry: Arc<RegistryHolder>) -> Self {
        Self { registry }
    }

    pub fn evaluate(&self, event: &TelemetryEvent) -> Result<Vec<Alert>, AppError> {
        let current = self.registry.load();

        let category = match event.event_type.as_str() {
            "process" | "processes" | "running_processes" | "process_start" | "process_end" | "process_events"
            | "bpf_process_events" | "osquery_result" | "osquery_snapshot" => "process",
            "network" | "listening_ports" | "network_connect" | "network_listen" | "socket_events"
            | "bpf_socket_events" => "network",
            "file" | "file_create" | "file_modify" | "file_delete" | "file_events" => "file",
            "auth" | "users" | "user_login" | "user_logout" | "logged_in_users" => "auth",
            "custom" => "custom",
            "osquery" => event
                .payload
                .get("query_name")
                .and_then(|v| v.as_str())
                .map_or("custom", |qn| match qn {
                    "running_processes" | "processes" => "process",
                    "listening_ports" | "network" => "network",
                    "users" => "auth",
                    _ => "custom",
                }),
            other => other,
        };

        let rules = match current.rule_sets.get(category) {
            Some(rules) => rules,
            None => return Ok(Vec::new()),
        };

        let scan_buffer = extract_scannable_buffer(event);
        if scan_buffer.is_empty() {
            return Ok(Vec::new());
        }

        let mut scanner = yara_x::Scanner::new(rules);
        let results = scanner.scan(&scan_buffer).map_err(|e| AppError::ScanFailure {
            event_id: event.id.clone(),
            message: e.to_string(),
        })?;

        let mut alerts = Vec::new();
        for matching_rule in results.matching_rules() {
            let alert = transform::build_alert(event, &matching_rule, &current.mitre);
            alerts.push(alert);
        }

        Ok(alerts)
    }
}
