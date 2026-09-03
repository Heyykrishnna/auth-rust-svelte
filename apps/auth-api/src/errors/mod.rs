use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("User not found")]
    UserNotFound,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Email already registered")]
    EmailAlreadyExists,

    #[error("Session not found or expired")]
    SessionNotFound,

    #[error("Invalid or expired token")]
    InvalidToken(String),

    #[error("Token has expired")]
    TokenExpired,

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Token creation failed: {0}")]
    TokenCreation(String),

    #[error("Password hashing error: {0}")]
    PasswordHash(String),

    #[error("OIDC error: {0}")]
    OidcError(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::UserNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::EmailAlreadyExists => (StatusCode::CONFLICT, self.to_string()),
            AppError::SessionNotFound => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            AppError::TokenCreation(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Token generation failed".to_string(),
            ),
            AppError::PasswordHash(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Authentication error".to_string(),
            ),
            AppError::OidcError(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            ),
            AppError::Redis(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Cache error".to_string()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = json!({
            "error": message,
            "status": status.as_u16(),
        });

        (status, axum::Json(body)).into_response()
    }
}
