use axum::extract::State;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::TokenPair;
use crate::services::authentication::{self, AuthContext};
use crate::services::cookies::attach_auth_cookies;
use crate::AppState;

#[derive(Debug, Deserialize, Default)]
pub struct RefreshRequest {
    pub refresh_token: Option<String>,
}

pub async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
    payload: Option<Json<RefreshRequest>>,
) -> Result<(CookieJar, Json<TokenPair>), AppError> {
    let refresh_token = payload
        .and_then(|Json(p)| p.refresh_token)
        .or_else(|| jar.get("refresh_token").map(|c| c.value().to_string()))
        .ok_or_else(|| AppError::Unauthorized("Missing refresh token".to_string()))?;

    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    let tokens = authentication::refresh(ctx, &refresh_token).await?;

    let jar = attach_auth_cookies(
        jar,
        &state.config,
        tokens.access_token.clone(),
        tokens.refresh_token.clone(),
    );

    Ok((jar, Json(tokens)))
}
