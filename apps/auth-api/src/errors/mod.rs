use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiErrorEnvelope {
    pub error: ApiErrorBody,
    pub status: u16,
}

impl ApiErrorEnvelope {
    pub fn new(
        status: StatusCode,
        code: impl Into<String>,
        message: impl Into<String>,
        request_id: Option<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self {
            error: ApiErrorBody {
                code: code.into(),
                message: message.into(),
                request_id,
                details,
            },
            status: status.as_u16(),
        }
    }
}

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

    #[error("Too many requests: {0}")]
    TooManyRequests(String),

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

impl AppError {
    pub fn to_envelope(&self, request_id: Option<String>) -> (StatusCode, ApiErrorEnvelope) {
        let (status, code, message) = match self {
            AppError::UserNotFound => (StatusCode::NOT_FOUND, "USER_NOT_FOUND", self.to_string()),
            AppError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "INVALID_CREDENTIALS",
                self.to_string(),
            ),
            AppError::EmailAlreadyExists => (
                StatusCode::CONFLICT,
                "EMAIL_ALREADY_EXISTS",
                self.to_string(),
            ),
            AppError::SessionNotFound => (
                StatusCode::UNAUTHORIZED,
                "SESSION_NOT_FOUND",
                self.to_string(),
            ),
            AppError::InvalidToken(msg) => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN", msg.clone()),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "TOKEN_EXPIRED", self.to_string()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            AppError::TooManyRequests(msg) => (
                StatusCode::TOO_MANY_REQUESTS,
                "TOO_MANY_REQUESTS",
                msg.clone(),
            ),
            AppError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                msg.clone(),
            ),
            AppError::TokenCreation(err) => {
                tracing::error!(error = %err, "Token generation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "TOKEN_GENERATION_FAILED",
                    "Token generation failed".to_string(),
                )
            }
            AppError::PasswordHash(err) => {
                tracing::error!(error = %err, "Password operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "AUTHENTICATION_ERROR",
                    "Authentication error".to_string(),
                )
            }
            AppError::OidcError(msg) => {
                tracing::warn!(error = %msg, "OIDC provider error");
                (StatusCode::BAD_GATEWAY, "OIDC_ERROR", msg.clone())
            }
            AppError::Database(err) => {
                tracing::error!(error = %err, "Database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "A database error occurred".to_string(),
                )
            }
            AppError::Redis(err) => {
                tracing::error!(error = %err, "Redis error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CACHE_ERROR",
                    "A cache error occurred".to_string(),
                )
            }
            AppError::Internal(err) => {
                tracing::error!(error = %err, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An unexpected error occurred".to_string(),
                )
            }
        };

        (
            status,
            ApiErrorEnvelope::new(status, code, message, request_id, None),
        )
    }

    pub fn into_response_with_request_id(self, request_id: Option<String>) -> Response {
        let (status, envelope) = self.to_envelope(request_id);
        (status, axum::Json(envelope)).into_response()
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        self.into_response_with_request_id(None)
    }
}
