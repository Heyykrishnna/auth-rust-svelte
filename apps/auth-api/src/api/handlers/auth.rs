use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use tracing::instrument;

use crate::{
    application::{
        login::{self, LoginRequest},
        logout, oidc,
        refresh::{self, RefreshRequest},
        register::{self, RegisterRequest},
    },
    domain::{errors::DomainError, token::validate_access_token},
    infrastructure::postgres,
    AppState,
};

// ─── POST /auth/register ──────────────────────────────────────────────────────

#[instrument(skip(state, body))]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> impl IntoResponse {
    match register::register(&state, body).await {
        Ok(res) => (StatusCode::CREATED, Json(json!({
            "user": res.user,
            "tokens": res.tokens,
        }))).into_response(),
        Err(e) => e.into_response(),
    }
}

// ─── POST /auth/login ─────────────────────────────────────────────────────────

#[instrument(skip(state, headers, body))]
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(String::from);

    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string());

    match login::login(&state, body, user_agent, ip_address).await {
        Ok(res) => Json(json!({
            "user": res.user,
            "tokens": res.tokens,
        })).into_response(),
        Err(e) => e.into_response(),
    }
}

// ─── POST /auth/logout ────────────────────────────────────────────────────────

#[instrument(skip(state, headers))]
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_bearer_token(&headers);
    match token {
        Some(t) => match logout::logout(&state, t).await {
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => e.into_response(),
        },
        None => DomainError::InvalidToken("No bearer token provided".to_string()).into_response(),
    }
}

// ─── POST /auth/refresh ───────────────────────────────────────────────────────

#[instrument(skip(state, body))]
pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> impl IntoResponse {
    match refresh::refresh(&state, body).await {
        Ok(tokens) => Json(tokens).into_response(),
        Err(e) => e.into_response(),
    }
}

// ─── GET /auth/me ─────────────────────────────────────────────────────────────

#[instrument(skip(state, headers))]
pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => return DomainError::InvalidToken("No bearer token provided".to_string()).into_response(),
    };

    let claims = match validate_access_token(token, &state.config.jwt_secret) {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    let user_id = match uuid::Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return DomainError::InvalidToken("Invalid user ID".to_string()).into_response(),
    };

    match postgres::find_user_by_id(&state.db, user_id).await {
        Ok(Some(user)) => Json(crate::domain::user::UserProfile::from(user)).into_response(),
        Ok(None) => DomainError::UserNotFound.into_response(),
        Err(e) => DomainError::Database(e).into_response(),
    }
}

// ─── GET /auth/oidc/:provider ─────────────────────────────────────────────────

pub async fn oidc_url(
    State(state): State<AppState>,
    Path(provider): Path<String>,
) -> impl IntoResponse {
    let provider = match provider.as_str() {
        "google" => oidc::OidcProvider::Google,
        "github" => oidc::OidcProvider::Github,
        _ => return DomainError::OidcError(format!("Unknown provider: {provider}")).into_response(),
    };

    match oidc::get_authorization_url(&state, provider).await {
        Ok(url) => Json(json!({ "authorization_url": url })).into_response(),
        Err(e) => e.into_response(),
    }
}

// ─── POST /auth/oidc/:provider/callback ───────────────────────────────────────

pub async fn oidc_callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let provider = match provider.as_str() {
        "google" => oidc::OidcProvider::Google,
        "github" => oidc::OidcProvider::Github,
        _ => return DomainError::OidcError(format!("Unknown provider: {provider}")).into_response(),
    };

    let code = body["code"].as_str().unwrap_or("").to_string();
    let state_param = body["state"].as_str().unwrap_or("").to_string();

    match oidc::handle_callback(&state, provider, code, state_param).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => e.into_response(),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn extract_bearer_token<'a>(headers: &'a HeaderMap) -> Option<&'a str> {
    headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
}
