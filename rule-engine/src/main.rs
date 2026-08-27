use std::{sync::Arc, time::Duration};

use edr_rule_engine::{
    config::RuleEngineConfig,
    db::create_pool,
    engine::{
        AlertSignature, EngineRegistry, RegistryHolder, ShardedDeduplicator, TypedRuleCompiler, YaraScannerEngine,
    },
    error::AppError,
    health::{AppState, create_health_router},
    kafka::{AlertKafkaProducer, DlqProducer, TelemetryConsumer},
    metrics::EngineMetrics,
    mitre::MitreTaxonomy,
    models::Alert,
    sink::{AlertSink, DualAlertSink},
};
use tokio::{sync::mpsc, time::Instant};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,edr_rule_engine=debug".into()),
        )
        .init();

    info!("Initializing Aigis-Zero EDR Rule Engine");

    let config = RuleEngineConfig::from_env().map_err(|e| {
        error!(error = %e, "Configuration error");
        e
    })?;

    // 1. In-memory MITRE ATT&CK taxonomy
    let mitre = if config.mitre_taxonomy_path.exists() {
        info!(path = %config.mitre_taxonomy_path.display(), "Loading MITRE ATT&CK taxonomy");
        MitreTaxonomy::load_from_file(&config.mitre_taxonomy_path)?
    } else {
        warn!(
            path = %config.mitre_taxonomy_path.display(),
            "MITRE taxonomy file not found; initializing empty taxonomy"
        );
        MitreTaxonomy::default()
    };
    info!(techniques_count = mitre.len(), "MITRE taxonomy loaded into memory");

    // 2. Compile YARA rules per event type
    info!(path = %config.rules_directory.display(), "Compiling YARA rules");
    let rule_sets = TypedRuleCompiler::compile_all(&config.rules_directory)?;
    let registry = EngineRegistry::new(rule_sets, mitre);
    let registry_holder = Arc::new(RegistryHolder::new(registry));

    // 3. Sharded alert deduplicator
    let deduplicator = Arc::new(ShardedDeduplicator::new(
        config.dedup_capacity,
        Duration::from_secs(config.dedup_suppression_window_secs),
    )?);

    // 4. PostgreSQL connection pool
    let pool = create_pool(&config.database_url, config.db_pool_max_size)
        .map_err(|e| AppError::DatabasePool(format!("Failed to build connection pool: {e}")))?;
    info!(
        max_size = config.db_pool_max_size,
        "Diesel-Async connection pool initialized"
    );

    // 5. Producers & Sinks
    let dlq_producer = DlqProducer::new(&config.kafka_brokers, &config.dlq_topic)?;
    let alert_producer = AlertKafkaProducer::new(&config.kafka_brokers, &config.alerts_topic)?;
    let alert_sink = Arc::new(DualAlertSink::new(alert_producer, pool.clone()));

    // 6. Metrics & Health State
    let metrics = Arc::new(EngineMetrics::new());
    let shutdown_token = CancellationToken::new();

    // 7. Channels
    let (event_tx, mut event_rx) = mpsc::channel(config.channel_capacity);
    let (alert_tx, mut alert_rx) = mpsc::channel::<Alert>(config.channel_capacity);

    // 8. Scanner Worker Loop
    let scanner_engine = Arc::new(YaraScannerEngine::new(registry_holder.clone()));
    let scanner_metrics = metrics.clone();
    let scanner_dedup = deduplicator.clone();
    let scanner_alert_tx = alert_tx.clone();
    let scanner_shutdown = shutdown_token.clone();

    let scanner_handle = tokio::spawn(async move {
        info!("Scanner evaluation loop started");
        loop {
            tokio::select! {
                _ = scanner_shutdown.cancelled() => {
                    info!("Scanner loop received shutdown signal; draining channel");
                    while let Ok(event) = event_rx.try_recv() {
                        scanner_metrics.inc_scanned();
                        if let Ok(alerts) = scanner_engine.evaluate(&event) {
                            for alert in alerts {
                                scanner_metrics.inc_alerts_generated();
                                let sig = AlertSignature {
                                    node_id: alert.node_id,
                                    rule_identifier: alert.description.clone(),
                                    mitre_technique: alert.mitre_technique_id.clone(),
                                };
                                if scanner_dedup.check_and_record(&sig).await {
                                    let _ = scanner_alert_tx.send(alert).await;
                                } else {
                                    scanner_metrics.inc_alerts_suppressed();
                                }
                            }
                        }
                    }
                    break;
                }
                event_opt = event_rx.recv() => {
                    match event_opt {
                        Some(event) => {
                            scanner_metrics.inc_scanned();
                            match scanner_engine.evaluate(&event) {
                                Ok(alerts) => {
                                    for alert in alerts {
                                        scanner_metrics.inc_alerts_generated();
                                        let sig = AlertSignature {
                                            node_id: alert.node_id,
                                            rule_identifier: alert.description.clone(),
                                            mitre_technique: alert.mitre_technique_id.clone(),
                                        };
                                        if scanner_dedup.check_and_record(&sig).await {
                                            if let Err(e) = scanner_alert_tx.send(alert).await {
                                                error!(error = %e, "Alert channel closed unexpectedly");
                                            }
                                        } else {
                                            scanner_metrics.inc_alerts_suppressed();
                                        }
                                    }
                                }
                                Err(err) => {
                                    warn!(error = %err, event_id = %event.id, "Evaluation failed on telemetry event");
                                }
                            }
                        }
                        None => break,
                    }
                }
            }
        }
        info!("Scanner evaluation loop terminated");
    });

    // 9. Alert Batcher Task
    let batch_sink = alert_sink.clone();
    let batch_metrics = metrics.clone();
    let batch_shutdown = shutdown_token.clone();
    let batch_max_size = config.batch_max_size;
    let batch_flush_interval = Duration::from_millis(config.batch_flush_interval_ms);

    let batcher_handle = tokio::spawn(async move {
        info!("Alert batching sink task started");
        let mut buffer = Vec::with_capacity(batch_max_size);
        let mut last_flush = Instant::now();

        loop {
            tokio::select! {
                _ = batch_shutdown.cancelled() => {
                    info!("Batcher received shutdown signal; flushing pending alerts");
                    while let Ok(alert) = alert_rx.try_recv() {
                        buffer.push(alert);
                    }
                    if !buffer.is_empty() {
                        if let Err(e) = batch_sink.send_batch(&buffer).await {
                            error!(error = %e, "Failed to flush alert batch during shutdown");
                        } else {
                            batch_metrics.inc_alerts_persisted(buffer.len() as u64);
                        }
                    }
                    break;
                }
                alert_opt = alert_rx.recv() => {
                    match alert_opt {
                        Some(alert) => {
                            buffer.push(alert);
                            if buffer.len() >= batch_max_size || last_flush.elapsed() >= batch_flush_interval {
                                if let Err(e) = batch_sink.send_batch(&buffer).await {
                                    error!(error = %e, count = buffer.len(), "Failed to persist alert batch");
                                } else {
                                    batch_metrics.inc_alerts_persisted(buffer.len() as u64);
                                }
                                buffer.clear();
                                last_flush = Instant::now();
                            }
                        }
                        None => {
                            if !buffer.is_empty() {
                                let _ = batch_sink.send_batch(&buffer).await;
                            }
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(batch_flush_interval) => {
                    if !buffer.is_empty() && last_flush.elapsed() >= batch_flush_interval {
                        if let Err(e) = batch_sink.send_batch(&buffer).await {
                            error!(error = %e, count = buffer.len(), "Periodic batch flush error");
                        } else {
                            batch_metrics.inc_alerts_persisted(buffer.len() as u64);
                        }
                        buffer.clear();
                        last_flush = Instant::now();
                    }
                }
            }
        }
        info!("Alert batcher task terminated");
    });

    // 10. Health & Metrics Server
    let health_state = AppState {
        pool: pool.clone(),
        metrics: metrics.clone(),
    };
    let health_router = create_health_router(health_state);
    let health_addr = format!("0.0.0.0:{}", config.health_port);
    let health_listener = tokio::net::TcpListener::bind(&health_addr).await?;
    info!(addr = %health_addr, "Health and Prometheus metrics server listening");

    let health_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(health_listener, health_router).await {
            error!(error = %e, "Health server terminated with error");
        }
    });

    // 11. SIGHUP Rule Hot-Reload Listener
    let reload_registry = registry_holder.clone();
    let reload_rules_dir = config.rules_directory.clone();
    let reload_mitre_path = config.mitre_taxonomy_path.clone();
    let reload_metrics = metrics.clone();

    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut sighup) = signal(SignalKind::hangup()) {
            while sighup.recv().await.is_some() {
                info!("SIGHUP received; initiating rule hot-reload");
                match TypedRuleCompiler::compile_all(&reload_rules_dir) {
                    Ok(new_rules) => {
                        let new_mitre = if reload_mitre_path.exists() {
                            MitreTaxonomy::load_from_file(&reload_mitre_path).unwrap_or_default()
                        } else {
                            MitreTaxonomy::default()
                        };
                        let new_reg = EngineRegistry::new(new_rules, new_mitre);
                        reload_registry.swap(new_reg);
                        reload_metrics.inc_rule_reloads();
                        info!("Rule hot-reload completed successfully");
                    }
                    Err(err) => {
                        reload_metrics.inc_rule_reload_failures();
                        error!(error = %err, "Rule compilation failed during SIGHUP; retaining previous ruleset");
                    }
                }
            }
        }
    });

    // 12. Kafka Telemetry Consumer
    let consumer = Arc::new(TelemetryConsumer::new(
        &config.kafka_brokers,
        &config.kafka_group_id,
        &config.kafka_topics,
        event_tx,
        dlq_producer,
        shutdown_token.clone(),
    )?);

    let consumer_handle = tokio::spawn(async move {
        consumer.run().await;
    });

    // 13. Wait for Termination Signal (SIGINT / SIGTERM)
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl+C received; initiating graceful shutdown");
        }
    }

    shutdown_token.cancel();
    let _ = tokio::join!(consumer_handle, scanner_handle, batcher_handle);
    health_handle.abort();

    info!("Aigis-Zero EDR Rule Engine shutdown complete");
    Ok(())
}
