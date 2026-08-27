use std::sync::Arc;

use crate::{
    engine::{registry::RegistryHolder, transform},
    error::AppError,
    models::{Alert, TelemetryEvent},
};

/// Extracts string fields from a telemetry event payload that are scannable.
/// Avoids scanning JSON structural characters (braces, quotes, commas) that interfere with string adjacency.
pub fn extract_scannable_buffer(event: &TelemetryEvent) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(1024);

    if let Some(obj) = event.payload.as_object() {
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

        let rules = match current.rule_sets.get(&event.event_type) {
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
