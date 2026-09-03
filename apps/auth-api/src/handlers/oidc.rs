use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

use crate::errors::AppError;
use crate::services::authentication::{self, AuthContext};
use crate::AppState;

pub async fn oidc_url(
    State(state): State<AppState>,
    Path(provider): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    let url = authentication::get_oidc_authorization_url(ctx, &provider).await?;
    Ok(Json(json!({ "authorization_url": url })))
}

pub async fn oidc_callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, AppError> {
    let code = body["code"].as_str().unwrap_or("").to_string();
    let state_param = body["state"].as_str().unwrap_or("").to_string();

    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    authentication::handle_oidc_callback(ctx, &provider, code, state_param).await?;
    Ok(StatusCode::OK)
}
