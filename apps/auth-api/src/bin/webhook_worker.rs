use std::time::Duration;

use axum::routing::get;
use axum::Router;
use axum_prometheus::PrometheusMetricLayer;
use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tracing::{error, info};

use auth_api::config::AppConfig;
use auth_api::middleware::init_worker_telemetry;
use auth_api::services::metrics as platform_metrics;
use auth_api::services::webhooks::run_worker_tick;

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sigterm) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            sigterm.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let config = AppConfig::from_env()?;
    init_worker_telemetry(&config)?;

    let poll_ms: u64 = std::env::var("WEBHOOK_WORKER_POLL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    let batch_size: i64 = std::env::var("WEBHOOK_WORKER_BATCH_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
        .clamp(1, 500);
    let metrics_port: u16 = std::env::var("WEBHOOK_WORKER_METRICS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9091);

    let db = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .connect(&config.database_url)
        .await?;

    let (_prom_layer, metrics_handle) = PrometheusMetricLayer::pair();
    platform_metrics::init_metrics_descriptors();

    let metrics_db = db.clone();
    let metrics_app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/metrics",
            get(move || {
                let handle = metrics_handle.clone();
                let pool = metrics_db.clone();
                async move {
                    platform_metrics::refresh_infrastructure_gauges(&pool).await;
                    handle.render()
                }
            }),
        );

    let metrics_addr = format!("0.0.0.0:{metrics_port}");
    if let Ok(listener) = tokio::net::TcpListener::bind(&metrics_addr).await {
        info!(address = %metrics_addr, "Webhook worker metrics server listening");
        tokio::spawn(async move {
            let _ = axum::serve(listener, metrics_app).await;
        });
    }

    info!(
        poll_ms = poll_ms,
        batch_size = batch_size,
        "Starting webhook delivery worker"
    );

    let mut shutdown = Box::pin(shutdown_signal());
    let mut ticker = tokio::time::interval(Duration::from_millis(poll_ms));

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                info!("Received shutdown signal; draining webhook worker");
                break;
            }
            _ = ticker.tick() => {
                match run_worker_tick(&db, &config, batch_size).await {
                    Ok(stats) => {
                        if stats.outbox_events_claimed > 0 || stats.deliveries_processed > 0 {
                            info!(
                                outbox_claimed = stats.outbox_events_claimed,
                                deliveries_enqueued = stats.deliveries_enqueued,
                                processed = stats.deliveries_processed,
                                succeeded = stats.deliveries_succeeded,
                                failed = stats.deliveries_failed,
                                dead_lettered = stats.deliveries_dead_lettered,
                                "Completed webhook worker tick"
                            );
                        }
                    }
                    Err(err) => {
                        error!(error = %err, "Webhook worker tick failed");
                    }
                }
            }
        }
    }

    db.close().await;
    opentelemetry::global::shutdown_tracer_provider();
    info!("Webhook delivery worker stopped cleanly");
    Ok(())
}
