pub mod auth;
pub mod health;
pub mod sessions;
pub mod users;

use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN};
use axum::http::{HeaderName, HeaderValue, Method};
use axum::routing::get;
use axum::Router;
use axum_prometheus::PrometheusMetricLayer;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::middleware::{api_rate_limit_layer, csrf_protection_middleware, security_headers_middleware};
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
        .allow_headers([
            AUTHORIZATION,
            ACCEPT,
            CONTENT_TYPE,
            ORIGIN,
            HeaderName::from_static("x-csrf-token"),
            HeaderName::from_static("x-request-id"),
        ])
        .allow_credentials(true)
        .allow_origin(cors_origins);

    let authenticated_api = Router::new()
        .nest("/users", users::routes())
        .nest("/api/users", users::routes())
        .nest("/sessions", sessions::routes())
        .nest("/api/sessions", sessions::routes())
        .layer(api_rate_limit_layer());

    Router::new()
        .merge(health::routes())
        .route(
            "/metrics",
            get(move || async move { metrics_handle.render() }),
        )
        .nest("/auth", auth::routes())
        .nest("/api/auth", auth::routes())
        .merge(authenticated_api)
        .layer(axum::middleware::from_fn(csrf_protection_middleware))
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(cors)
        .layer(prometheus_layer)
        .layer(axum::Extension(state.clone()))
        .with_state(state)
}
