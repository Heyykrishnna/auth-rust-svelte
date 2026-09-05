use axum::extract::State;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::TokenPair;
use crate::services::authentication::{self, AuthContext};
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

    Ok((jar, Json(tokens)))
}
