mod ports;
mod settings;

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio_util::sync::CancellationToken;

use fleet_tracing::{LogFormat, TracingConfig};
use grpc_listener::{FleetServiceImpl, GrpcListenerConfig, GrpcServer, shutdown_signal};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file into standard environment variables.
    // .env should be at the workspace root or the current working directory.
    dotenvy::dotenv().ok();

    let settings = settings::Settings::load().context("failed to load settings")?;

    // Tracing must be initialised before anything else emits spans.
    let log_format = settings
        .log_format
        .parse::<LogFormat>()
        .unwrap_or(LogFormat::Human);

    fleet_tracing::init(&TracingConfig {
        log_level: settings.rust_log.clone(),
        format: log_format,
        service_name: "fleet-server".to_string(),
    })
    .context("failed to initialise tracing")?;

    tracing::info!(
        host = %settings.host,
        port = settings.port,
        "fleet server starting"
    );

    // Connect to Postgres and run pending migrations before accepting any traffic.
    // If DATABASE_URL is wrong or Postgres is down, we fail here with a clear error
    // rather than silently dropping every enrollment that comes in.
    let pg_pool = postgres_interface::connect(&settings.database_url)
        .await
        .context(
            "failed to connect to postgres — check DATABASE_URL and ensure the DB is running",
        )?;

    let (enrollment, heartbeat, event_ingest) = ports::build_ports(pg_pool, &settings.jwt_secret);

    let service = FleetServiceImpl::new(
        Arc::clone(&enrollment) as Arc<dyn fleet_manager::EnrollmentPort>,
        Arc::clone(&heartbeat) as Arc<dyn fleet_manager::HeartbeatPort>,
        Arc::clone(&event_ingest) as Arc<dyn fleet_manager::EventIngestPort>,
        &settings.jwt_secret,
    );

    let grpc_config = GrpcListenerConfig {
        host: settings.host,
        port: settings.port,
        jwt_secret: settings.jwt_secret,
    };

    // CancellationToken propagates the shutdown signal from OS signals
    // to every subsystem that needs it.
    let shutdown_token = CancellationToken::new();

    // Spawn a task that fires the token on SIGINT / SIGTERM.
    {
        let token = shutdown_token.clone();
        tokio::spawn(async move {
            wait_for_signal().await;
            tracing::info!("shutdown signal received, stopping fleet server");
            token.cancel();
        });
    }

    GrpcServer::new(grpc_config, service)
        .serve_until_shutdown(shutdown_signal(shutdown_token))
        .await
        .context("gRPC server error")?;

    tracing::info!("fleet server stopped");
    Ok(())
}

/// Waits for SIGINT (Ctrl-C) or SIGTERM (Docker / systemd stop).
async fn wait_for_signal() {
    use tokio::signal;

    #[cfg(unix)]
    {
        use signal::unix::{SignalKind, signal};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
        tokio::select! {
            _ = signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = signal::ctrl_c().await;
    }
}
