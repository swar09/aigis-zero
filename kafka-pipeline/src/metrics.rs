//! Consumer group telemetry and Prometheus metrics instrumentation for Kafka pipelines.

use std::sync::atomic::{AtomicU64, Ordering};

/// Pipeline performance and throughput metrics counters.
#[derive(Debug, Default)]
pub struct PipelineMetrics {
    pub events_consumed: AtomicU64,
    pub events_routed_process: AtomicU64,
    pub events_routed_network: AtomicU64,
    pub events_routed_file: AtomicU64,
    pub events_routed_auth: AtomicU64,
    pub events_routed_dlq: AtomicU64,
    pub routing_errors: AtomicU64,
}

impl PipelineMetrics {
    /// Creates a new metrics container.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc_consumed(&self) {
        self.events_consumed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_routed(&self, target_topic: &str) {
        match target_topic {
            "aigis.events.process" => self.events_routed_process.fetch_add(1, Ordering::Relaxed),
            "aigis.events.network" => self.events_routed_network.fetch_add(1, Ordering::Relaxed),
            "aigis.events.file" => self.events_routed_file.fetch_add(1, Ordering::Relaxed),
            "aigis.events.auth" => self.events_routed_auth.fetch_add(1, Ordering::Relaxed),
            _ => self.events_routed_dlq.fetch_add(1, Ordering::Relaxed),
        };
    }

    pub fn inc_errors(&self) {
        self.routing_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn format_prometheus(&self) -> String {
        format!(
            "# HELP aigis_pipeline_events_consumed_total Total raw events consumed by normalization pipeline\n\
             # TYPE aigis_pipeline_events_consumed_total counter\n\
             aigis_pipeline_events_consumed_total {}\n\
             # HELP aigis_pipeline_events_routed_total Total events routed to typed topics\n\
             # TYPE aigis_pipeline_events_routed_total counter\n\
             aigis_pipeline_events_routed_total{{topic=\"aigis.events.process\"}} {}\n\
             aigis_pipeline_events_routed_total{{topic=\"aigis.events.network\"}} {}\n\
             aigis_pipeline_events_routed_total{{topic=\"aigis.events.file\"}} {}\n\
             aigis_pipeline_events_routed_total{{topic=\"aigis.events.auth\"}} {}\n\
             aigis_pipeline_events_routed_total{{topic=\"aigis.events.dlq\"}} {}\n\
             # HELP aigis_pipeline_routing_errors_total Total unparsable event errors encountered\n\
             # TYPE aigis_pipeline_routing_errors_total counter\n\
             aigis_pipeline_routing_errors_total {}\n",
            self.events_consumed.load(Ordering::Relaxed),
            self.events_routed_process.load(Ordering::Relaxed),
            self.events_routed_network.load(Ordering::Relaxed),
            self.events_routed_file.load(Ordering::Relaxed),
            self.events_routed_auth.load(Ordering::Relaxed),
            self.events_routed_dlq.load(Ordering::Relaxed),
            self.routing_errors.load(Ordering::Relaxed)
        )
    }
}
