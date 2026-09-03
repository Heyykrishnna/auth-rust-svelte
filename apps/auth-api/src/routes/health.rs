use axum::routing::get;
use axum::Router;

use crate::handlers;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health::health_check))
        .route("/ready", get(handlers::health::readiness_check))
}
