use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};

use crate::errors::AppError;
use crate::services::authentication::{self, AuthContext};
use crate::AppState;

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let auth_header = headers
        .get("authorization")
        .and_then(|val| val.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid authorization scheme".to_string()))?;

    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    authentication::logout(ctx, token).await?;

    Ok(StatusCode::NO_CONTENT)
}
