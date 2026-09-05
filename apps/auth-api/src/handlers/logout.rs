use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};

use crate::errors::AppError;
use crate::services::authentication::{self, AuthContext};
use crate::AppState;

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(CookieJar, StatusCode), AppError> {
    let token = jar
        .get("session_token")
        .map(|c| c.value().to_string())
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|val| val.to_str().ok())
                .and_then(|val| val.strip_prefix("Bearer ").map(String::from))
        });

    if let Some(token) = token {
        let ctx = AuthContext {
            config: &state.config,
            db: &state.db,
            redis: &state.redis,
        };

        let _ = authentication::logout(ctx, &token).await;
    }

    let mut clear_session = Cookie::build(("session_token", ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::ZERO);

    if let Some(ref domain) = state.config.cookie_domain {
        clear_session = clear_session.domain(domain.clone());
    }

    let mut clear_refresh = Cookie::build(("refresh_token", ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::ZERO);

    if let Some(ref domain) = state.config.cookie_domain {
        clear_refresh = clear_refresh.domain(domain.clone());
    }

    let jar = jar.add(clear_session).add(clear_refresh);

    Ok((jar, StatusCode::NO_CONTENT))
}
