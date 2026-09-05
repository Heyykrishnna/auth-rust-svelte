use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
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
    pub message: String,
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<(CookieJar, Json<LoginResponse>), AppError> {
    // 1. Validate request
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

    let mut session_cookie = Cookie::build(("session_token", tokens.access_token.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(state.config.jwt_access_expiry_secs as i64));

    if state.config.cookie_secure {
        session_cookie = session_cookie.secure(true);
    }

    if let Some(ref domain) = state.config.cookie_domain {
        session_cookie = session_cookie.domain(domain.clone());
    }

    let mut refresh_cookie = Cookie::build(("refresh_token", tokens.refresh_token.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(state.config.jwt_refresh_expiry_secs as i64));

    if state.config.cookie_secure {
        refresh_cookie = refresh_cookie.secure(true);
    }

    if let Some(ref domain) = state.config.cookie_domain {
        refresh_cookie = refresh_cookie.domain(domain.clone());
    }

    let jar = jar.add(session_cookie).add(refresh_cookie);

    Ok((
        jar,
        Json(LoginResponse {
            user,
            tokens,
            message: "Login successful".to_string(),
        }),
    ))
}
