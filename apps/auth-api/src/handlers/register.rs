use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::errors::AppError;
use crate::models::{TokenPair, UserProfile};
use crate::services::authentication::{self, AuthContext};
use crate::services::cookies::attach_auth_cookies;
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
pub struct RegisterInitiateResponse {
    pub status: String,
    pub email: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterVerifyRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(equal = 6, message = "Verification code must be 6 digits"))]
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user: UserProfile,
    pub tokens: TokenPair,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResendOtpRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterInitiateResponse>), AppError> {


    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    let email = authentication::initiate_registration(
        ctx,
        payload.email,
        payload.password,
        payload.display_name,
    )
    .await?;

    Ok((
        StatusCode::OK,
        Json(RegisterInitiateResponse {
            status: "pending_verification".to_string(),
            email,
            message: "Verification code sent to your email".to_string(),
        }),
    ))
}

pub async fn verify_register_otp(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(payload): Json<RegisterVerifyRequest>,
) -> Result<(StatusCode, CookieJar, Json<RegisterResponse>), AppError> {
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

    let (user, tokens) = authentication::verify_registration_otp(
        ctx,
        payload.email,
        payload.code,
        user_agent,
        ip_address,
    )
    .await?;

    let jar = attach_auth_cookies(
        jar,
        &state.config,
        tokens.access_token.clone(),
        tokens.refresh_token.clone(),
    );

    Ok((
        StatusCode::CREATED,
        jar,
        Json(RegisterResponse { user, tokens }),
    ))
}

pub async fn resend_register_otp(
    State(state): State<AppState>,
    Json(payload): Json<ResendOtpRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    authentication::resend_registration_otp(ctx, payload.email).await?;

    Ok(Json(MessageResponse {
        message: "Verification code resent successfully".to_string(),
    }))
}
