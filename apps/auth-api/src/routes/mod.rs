pub mod auth;
pub mod control;
pub mod health;
pub mod oidc;
pub mod sessions;
pub mod users;

use axum::routing::get;
use axum::Router;
use axum_prometheus::metrics_exporter_prometheus::PrometheusHandle;
use axum_prometheus::PrometheusMetricLayer;
use std::sync::OnceLock;
use std::time::Duration;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::middleware::{
    api_rate_limit_layer, csrf_protection_middleware, dynamic_cors_middleware,
    security_headers_middleware, telemetry_correlation_middleware,
};
use crate::services::metrics as platform_metrics;
use crate::AppState;

static PROMETHEUS_PAIR: OnceLock<(PrometheusMetricLayer<'static>, PrometheusHandle)> =
    OnceLock::new();

pub fn build_router(state: AppState) -> Router {
    let (prometheus_layer, metrics_handle) = PROMETHEUS_PAIR
        .get_or_init(|| {
            let pair = PrometheusMetricLayer::pair();
            platform_metrics::init_metrics_descriptors();
            pair
        })
        .clone();

    let metrics_db = state.db.clone();

    let authenticated_api = Router::new()
        .nest("/users", users::routes())
        .nest("/api/users", users::routes())
        .nest("/sessions", sessions::routes())
        .nest("/api/sessions", sessions::routes())
        .layer(api_rate_limit_layer());

    Router::new()
        .merge(health::routes())
        .route(
            "/.well-known/jwks.json",
            get(crate::handlers::client_auth::jwks_endpoint),
        )
        .route(
            "/.well-known/openid-configuration",
            get(crate::handlers::oidc_provider::openid_configuration),
        )
        .nest("/oauth2", oidc::oidc_routes())
        .route(
            "/metrics",
            get(move || {
                let handle = metrics_handle.clone();
                let db = metrics_db.clone();
                async move {
                    platform_metrics::refresh_infrastructure_gauges(&db).await;
                    handle.render()
                }
            }),
        )
        .nest("/auth", auth::routes())
        .nest("/api/auth", auth::routes())
        .nest("/v1/control", control::control_routes())
        .nest("/v1/client", control::client_v1_routes())
        .nest("/v1/backend", control::backend_v1_routes())
        .merge(authenticated_api)
        .layer(axum::middleware::from_fn(csrf_protection_middleware))
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(CatchPanicLayer::new())
        .layer(axum::middleware::from_fn(telemetry_correlation_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            dynamic_cors_middleware,
        ))
        .layer(prometheus_layer)
        .layer(axum::Extension(state.clone()))
        .with_state(state)
}
