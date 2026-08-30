use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub mod consumer;
pub mod event_router;
pub mod health;
pub mod metrics;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").json().init();

    info!("Initializing Aigis-Zero Normalization & Routing Pipeline");

    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:29092".into());
    let health_port: u16 = std::env::var("HEALTH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8082);

    let shutdown = CancellationToken::new();
    let pipeline_metrics = Arc::new(metrics::PipelineMetrics::new());

    // 1. Start Axum Health & Prometheus Metrics Server
    let health_state = health::PipelineHealthState {
        metrics: pipeline_metrics.clone(),
    };
    let health_router = health::create_health_router(health_state);
    let health_addr = format!("0.0.0.0:{health_port}");
    let health_listener = tokio::net::TcpListener::bind(&health_addr).await?;
    info!(addr = %health_addr, "Health and Prometheus metrics server listening");

    let _health_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(health_listener, health_router).await {
            error!(error = %e, "Pipeline health server terminated unexpectedly");
        }
    });

    // 2. Start event router producer with tuned throughput settings
    let router_producer: rdkafka::producer::FutureProducer = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("linger.ms", "5")
        .set("compression.type", "lz4")
        .set("batch.num.messages", "10000")
        .set("queue.buffering.max.messages", "100000")
        .set("queue.buffering.max.kbytes", "65536")
        .set("acks", "1")
        .create()
        .expect("Router producer creation failed");

    let processor = event_router::EventRouterProcessor::new(router_producer, pipeline_metrics);
    let worker = consumer::ConsumerWorker::new(
        &brokers,
        "aigis-event-router",
        &["aigis.events.raw"],
        Box::new(processor),
        shutdown.clone(),
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    let shutdown_signal = shutdown.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutdown signal received; initiating graceful termination");
        shutdown_signal.cancel();
    });

    worker.run().await;

    info!("Kafka normalization pipeline shut down successfully");
    Ok(())
}
