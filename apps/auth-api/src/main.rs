use anyhow::Result;
use std::sync::Arc;
use tracing::info;

mod api;
mod application;
mod config;
mod domain;
mod infrastructure;

use config::AppConfig;
use infrastructure::{postgres::PgPool, redis::RedisPool};

/// Shared application state passed to all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub redis: RedisPool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present (dev only; in production, env vars come from k8s secrets)
    let _ = dotenvy::dotenv();

    // Load config
    let config = Arc::new(AppConfig::from_env()?);

    // Initialize tracing + OpenTelemetry
    infrastructure::telemetry::init_telemetry(&config)?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        host = %config.host,
        port = config.port,
        "Starting auth-api"
    );

    // Connect to PostgreSQL
    let db = infrastructure::postgres::connect(&config.database_url, config.db_max_connections).await?;

    // Run migrations automatically on startup
    sqlx::migrate!("./migrations").run(&db).await?;
    info!("Database migrations applied successfully");

    // Connect to Redis
    let redis = infrastructure::redis::connect(&config.redis_url).await?;
    info!("Connected to Redis");

    let state = AppState {
        config: config.clone(),
        db,
        redis,
    };

    // Build the Axum router
    let app = api::routes::build_router(state);

    // Bind and serve
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!(address = %addr, "Server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Flush telemetry on shutdown
    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}

/// Handle SIGTERM / Ctrl-C for graceful shutdown.
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { info!("Received Ctrl-C, shutting down..."); },
        _ = terminate => { info!("Received SIGTERM, shutting down..."); },
    }
}
