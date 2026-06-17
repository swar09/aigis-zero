use tokio_util::sync::CancellationToken;
use tracing::info;

pub mod consumer;
pub mod event_router;

/// Initializes and runs the Kafka event routing pipeline.
///
/// Consumes events from the `aigis.events.raw` topic and processes them through the event router.
/// The Kafka broker addresses are read from the `KAFKA_BROKERS` environment variable
/// (defaults to `localhost:29092`). The pipeline gracefully shuts down upon receiving SIGINT (Ctrl+C).
///
/// # Errors
///
/// Returns an error if the consumer worker fails to initialize.
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .json()
        .init();

    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:29092".into());
    let shutdown = CancellationToken::new();

    // Start event router consumer
    let router_producer = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("linger.ms", "5")
        .set("compression.type", "lz4")
        .create()
        .expect("Router producer creation failed");

    let processor = event_router::EventRouterProcessor::new(router_producer);
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
        info!("Shutdown signal received");
        shutdown_signal.cancel();
    });

    worker.run().await;

    info!("Kafka pipeline shut down");
    Ok(())
}
