use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::errors::AppError;
use crate::models::{TokenPair, UserProfile};
use crate::services::authentication::{self, AuthContext};
use crate::AppState;

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserProfile,
    pub tokens: TokenPair,
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(String::from);

    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string());

    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    let (user, tokens) =
        authentication::login(ctx, payload.email, payload.password, user_agent, ip_address).await?;

    Ok(Json(LoginResponse { user, tokens }))
}
