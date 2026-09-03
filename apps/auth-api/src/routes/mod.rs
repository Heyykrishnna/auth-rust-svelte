pub mod auth;
pub mod health;
pub mod sessions;
pub mod users;

use axum::http::{HeaderValue, Method};
use axum::routing::get;
use axum::Router;
use axum_prometheus::PrometheusMetricLayer;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::AppState;

pub fn build_router(state: AppState) -> Router {
    let (prometheus_layer, metrics_handle) = PrometheusMetricLayer::pair();

    let cors_origins: Vec<HeaderValue> = state
        .config
        .cors_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .allow_credentials(true)
        .allow_origin(cors_origins);

    Router::new()
        .merge(health::routes())
        .route(
            "/metrics",
            get(move || async move { metrics_handle.render() }),
        )
        .nest("/auth", auth::routes())
        .nest("/users", users::routes())
        .nest("/sessions", sessions::routes())
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(cors)
        .layer(prometheus_layer)
        .with_state(state)
}
