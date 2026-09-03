use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use crate::AppState;

/// GET /health — liveness probe
pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// GET /ready — readiness probe (checks DB and Redis connectivity)
pub async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    // Check PostgreSQL
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .is_ok();

    // Check Redis
    let redis_ok = state.redis.get().await
        .map(|mut conn| {
            futures::executor::block_on(async {
                redis::cmd("PING")
                    .query_async::<String>(&mut conn)
                    .await
                    .is_ok()
            })
        })
        .unwrap_or(false);

    let all_ok = db_ok && redis_ok;
    let status_code = if all_ok {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(json!({
            "status": if all_ok { "ready" } else { "not ready" },
            "checks": {
                "postgres": if db_ok { "ok" } else { "error" },
                "redis": if redis_ok { "ok" } else { "error" },
            }
        })),
    )
}
