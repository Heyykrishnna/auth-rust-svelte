use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tracing::{error, info};

use auth_api::config::AppConfig;
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
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "webhook_worker=info,auth_api=info".into()),
        )
        .json()
        .init();

    let config = AppConfig::from_env()?;
    let poll_ms: u64 = std::env::var("WEBHOOK_WORKER_POLL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    let batch_size: i64 = std::env::var("WEBHOOK_WORKER_BATCH_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
        .clamp(1, 500);

    let db = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .connect(&config.database_url)
        .await?;

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
    info!("Webhook delivery worker stopped cleanly");
    Ok(())
}
