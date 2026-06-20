use tokio_util::sync::CancellationToken;
use tracing::info;

pub mod consumer;
pub mod event_router;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
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
