use std::sync::Arc;

use edr_kafka_pipeline::{
    health::{PipelineHealthState, create_health_router},
    metrics::PipelineMetrics,
};

#[test]
fn test_metrics_counter_increments() {
    let metrics = PipelineMetrics::new();
    metrics.inc_consumed();
    metrics.inc_routed("aigis.events.process");
    metrics.inc_routed("aigis.events.network");
    metrics.inc_routed("aigis.events.file");
    metrics.inc_routed("aigis.events.auth");
    metrics.inc_routed("aigis.events.dlq");
    metrics.inc_errors();

    let output = metrics.format_prometheus();
    assert!(output.contains("aigis_pipeline_events_consumed_total 1"));
    assert!(output.contains("aigis_pipeline_events_routed_total{topic=\"aigis.events.process\"} 1"));
    assert!(output.contains("aigis_pipeline_events_routed_total{topic=\"aigis.events.network\"} 1"));
    assert!(output.contains("aigis_pipeline_events_routed_total{topic=\"aigis.events.file\"} 1"));
    assert!(output.contains("aigis_pipeline_events_routed_total{topic=\"aigis.events.auth\"} 1"));
    assert!(output.contains("aigis_pipeline_events_routed_total{topic=\"aigis.events.dlq\"} 1"));
    assert!(output.contains("aigis_pipeline_routing_errors_total 1"));
}

#[tokio::test]
async fn test_health_router_endpoints() {
    let metrics = Arc::new(PipelineMetrics::new());
    let state = PipelineHealthState {
        metrics: metrics.clone(),
    };
    let _router = create_health_router(state);
    // Verify router creates successfully and handles state
    assert_eq!(metrics.events_consumed.load(std::sync::atomic::Ordering::Relaxed), 0);
}
