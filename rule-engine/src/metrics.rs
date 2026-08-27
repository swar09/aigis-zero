use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct EngineMetrics {
    pub events_consumed: AtomicU64,
    pub events_scanned: AtomicU64,
    pub alerts_generated: AtomicU64,
    pub alerts_suppressed: AtomicU64,
    pub alerts_persisted: AtomicU64,
    pub rule_reloads_total: AtomicU64,
    pub rule_reload_failures_total: AtomicU64,
}

impl EngineMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc_consumed(&self) {
        self.events_consumed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_scanned(&self) {
        self.events_scanned.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_alerts_generated(&self) {
        self.alerts_generated.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_alerts_suppressed(&self) {
        self.alerts_suppressed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_alerts_persisted(&self, count: u64) {
        self.alerts_persisted.fetch_add(count, Ordering::Relaxed);
    }

    pub fn inc_rule_reloads(&self) {
        self.rule_reloads_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_rule_reload_failures(&self) {
        self.rule_reload_failures_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn format_prometheus(&self) -> String {
        format!(
            "# HELP aigis_events_consumed_total Total number of events consumed from Kafka\n\
             # TYPE aigis_events_consumed_total counter\n\
             aigis_events_consumed_total {}\n\
             # HELP aigis_events_scanned_total Total number of events evaluated by YARA-X\n\
             # TYPE aigis_events_scanned_total counter\n\
             aigis_events_scanned_total {}\n\
             # HELP aigis_alerts_generated_total Total number of alerts generated\n\
             # TYPE aigis_alerts_generated_total counter\n\
             aigis_alerts_generated_total {}\n\
             # HELP aigis_alerts_suppressed_total Total number of duplicate alerts suppressed\n\
             # TYPE aigis_alerts_suppressed_total counter\n\
             aigis_alerts_suppressed_total {}\n\
             # HELP aigis_alerts_persisted_total Total number of alerts persisted to sinks\n\
             # TYPE aigis_alerts_persisted_total counter\n\
             aigis_alerts_persisted_total {}\n\
             # HELP aigis_rule_reloads_total Total number of successful rule hot-reloads\n\
             # TYPE aigis_rule_reloads_total counter\n\
             aigis_rule_reloads_total {}\n\
             # HELP aigis_rule_reload_failures_total Total number of failed rule hot-reloads\n\
             # TYPE aigis_rule_reload_failures_total counter\n\
             aigis_rule_reload_failures_total {}\n",
            self.events_consumed.load(Ordering::Relaxed),
            self.events_scanned.load(Ordering::Relaxed),
            self.alerts_generated.load(Ordering::Relaxed),
            self.alerts_suppressed.load(Ordering::Relaxed),
            self.alerts_persisted.load(Ordering::Relaxed),
            self.rule_reloads_total.load(Ordering::Relaxed),
            self.rule_reload_failures_total.load(Ordering::Relaxed),
        )
    }
}
