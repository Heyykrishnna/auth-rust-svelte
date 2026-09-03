use axum::{
    http::{HeaderValue, Method},
    Router,
};
use axum_prometheus::PrometheusMetricLayer;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use super::handlers;
use crate::AppState;

/// Build the complete Axum router with all routes and middleware.
pub fn build_router(state: AppState) -> Router {
    // ─── Prometheus Metrics ────────────────────────────────────────────────────
    let (prometheus_layer, metrics_handle) = PrometheusMetricLayer::pair();

    // ─── CORS ─────────────────────────────────────────────────────────────────
    let cors_origins: Vec<HeaderValue> = state
        .config
        .cors_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any)
        .allow_credentials(true)
        .allow_origin(cors_origins);

    // ─── Auth Routes ──────────────────────────────────────────────────────────
    let auth_router = axum::Router::new()
        .route("/register",                axum::routing::post(handlers::auth::register))
        .route("/login",                   axum::routing::post(handlers::auth::login))
        .route("/logout",                  axum::routing::post(handlers::auth::logout))
        .route("/refresh",                 axum::routing::post(handlers::auth::refresh))
        .route("/me",                      axum::routing::get(handlers::auth::me))
        .route("/oidc/:provider",          axum::routing::get(handlers::auth::oidc_url))
        .route("/oidc/:provider/callback", axum::routing::post(handlers::auth::oidc_callback));

    // ─── Full Router ──────────────────────────────────────────────────────────
    Router::new()
        // Health endpoints (no auth)
        .route("/health",  axum::routing::get(handlers::health::health_check))
        .route("/ready",   axum::routing::get(handlers::health::readiness_check))
        .route("/metrics", axum::routing::get(move || async move { metrics_handle.render() }))
        // Auth API
        .nest("/auth", auth_router)
        // Middleware stack
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(cors)
                .layer(prometheus_layer),
        )
        .with_state(state)
}
