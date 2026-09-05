use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
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
    jar: CookieJar,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, CookieJar, Json<RegisterResponse>), AppError> {
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

    Ok((StatusCode::CREATED, jar, Json(RegisterResponse { user, tokens })))
}
