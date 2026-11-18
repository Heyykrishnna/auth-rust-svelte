use anyhow::Result;
use deadpool_redis::{Config as RedisConfig, Runtime as RedisRuntime};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing::info;

use auth_api::config::AppConfig;
use auth_api::{middleware, routes, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    let config = Arc::new(AppConfig::from_env()?);

    middleware::init_telemetry(&config)?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        host = %config.host,
        port = config.port,
        "Starting auth-api"
    );

    let db = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;
    info!("Database migrations applied successfully");

    let redis_cfg = RedisConfig::from_url(&config.redis_url);
    let redis = redis_cfg.create_pool(Some(RedisRuntime::Tokio1))?;
    info!("Redis connection pool initialized");

    let state = AppState {
        config: config.clone(),
        db,
        redis,
    };

    let app = routes::build_router(state.clone());
    let metrics_host = std::env::var("METRICS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let metrics_port = std::env::var("METRICS_PORT")
        .unwrap_or_else(|_| "9090".to_string())
        .parse::<u16>()?;
    let metrics_listener =
        tokio::net::TcpListener::bind((metrics_host.as_str(), metrics_port)).await?;
    let (shutdown_sender, mut shutdown_receiver) = tokio::sync::watch::channel(false);
    let metrics_task = tokio::spawn(async move {
        axum::serve(metrics_listener, routes::metrics_router(state))
            .with_graceful_shutdown(async move {
                let _ = shutdown_receiver.changed().await;
            })
            .await
    });

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!(address = %addr, "Server listening");

    let result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await;
    shutdown_sender.send(true)?;
    metrics_task.await??;
    result?;

    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}

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
        _ = ctrl_c => {
            info!("Received Ctrl-C, shutting down...");
        },
        _ = terminate => {
            info!("Received SIGTERM, shutting down...");
        },
    }
}
