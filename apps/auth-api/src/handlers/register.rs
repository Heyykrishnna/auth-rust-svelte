use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::errors::AppError;
use crate::models::{TokenPair, UserProfile};
use crate::services::authentication::{self, AuthContext};
use crate::AppState;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    #[validate(length(
        min = 2,
        max = 100,
        message = "Display name must be between 2 and 100 characters"
    ))]
    pub display_name: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user: UserProfile,
    pub tokens: TokenPair,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    let (user, tokens) =
        authentication::register(ctx, payload.email, payload.password, payload.display_name)
            .await?;

    Ok((StatusCode::CREATED, Json(RegisterResponse { user, tokens })))
}
