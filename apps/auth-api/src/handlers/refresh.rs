use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::TokenPair;
use crate::services::authentication::{self, AuthContext};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<TokenPair>, AppError> {
    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    let tokens = authentication::refresh(ctx, &payload.refresh_token).await?;

    Ok(Json(tokens))
}
