use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use crate::AppState;

pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();

    let redis_ok = match state.redis.get().await {
        Ok(mut conn) => {
            let res: Result<String, redis::RedisError> =
                redis::cmd("PING").query_async(&mut conn).await;
            res.is_ok()
        }
        Err(_) => false,
    };

    let all_ok = db_ok && redis_ok;
    let status_code = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
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
