use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::services::authentication::{self, AuthContext};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyEmailResponse {
    pub message: String,
}

pub async fn verify_email(
    State(state): State<AppState>,
    Query(query): Query<VerifyEmailQuery>,
) -> Result<Json<VerifyEmailResponse>, AppError> {
    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    authentication::verify_email(ctx, &query.token).await?;

    Ok(Json(VerifyEmailResponse {
        message: "Email successfully verified".to_string(),
    }))
}

pub async fn verify_email_post(
    State(state): State<AppState>,
    Json(payload): Json<VerifyEmailQuery>,
) -> Result<Json<VerifyEmailResponse>, AppError> {
    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    authentication::verify_email(ctx, &payload.token).await?;

    Ok(Json(VerifyEmailResponse {
        message: "Email successfully verified".to_string(),
    }))
}
