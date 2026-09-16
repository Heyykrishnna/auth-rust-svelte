use crate::config::AppConfig;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};

pub fn build_session_cookie<'a>(config: &AppConfig, token: String) -> Cookie<'a> {
    let mut cookie = Cookie::build(("session_token", token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(
            config.jwt_access_expiry_secs as i64,
        ));

    if config.cookie_secure {
        cookie = cookie.secure(true);
    }
    if let Some(ref domain) = config.cookie_domain {
        cookie = cookie.domain(domain.clone());
    }
    cookie.build()
}

pub fn build_refresh_cookie<'a>(config: &AppConfig, token: String) -> Cookie<'a> {
    let mut cookie = Cookie::build(("refresh_token", token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(
            config.jwt_refresh_expiry_secs as i64,
        ));

    if config.cookie_secure {
        cookie = cookie.secure(true);
    }
    if let Some(ref domain) = config.cookie_domain {
        cookie = cookie.domain(domain.clone());
    }
    cookie.build()
}

pub fn build_csrf_cookie<'a>(config: &AppConfig, token: String) -> Cookie<'a> {
    let mut cookie = Cookie::build(("csrf_token", token))
        .path("/")
        .http_only(false)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(
            config.jwt_refresh_expiry_secs as i64,
        ));

    if config.cookie_secure {
        cookie = cookie.secure(true);
    }
    if let Some(ref domain) = config.cookie_domain {
        cookie = cookie.domain(domain.clone());
    }
    cookie.build()
}

pub fn attach_auth_cookies(
    jar: CookieJar,
    config: &AppConfig,
    access_token: String,
    refresh_token: String,
) -> CookieJar {
    let csrf_token = crate::middleware::generate_csrf_token();
    let csrf_cookie = build_csrf_cookie(config, csrf_token);
    let session_cookie = build_session_cookie(config, access_token);
    let refresh_cookie = build_refresh_cookie(config, refresh_token);
    jar.add(session_cookie).add(refresh_cookie).add(csrf_cookie)
}

pub fn clear_auth_cookies(jar: CookieJar, config: &AppConfig) -> CookieJar {
    let mut clear_session = Cookie::build(("session_token", ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::ZERO);

    if let Some(ref domain) = config.cookie_domain {
        clear_session = clear_session.domain(domain.clone());
    }

    let mut clear_refresh = Cookie::build(("refresh_token", ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::ZERO);

    if let Some(ref domain) = config.cookie_domain {
        clear_refresh = clear_refresh.domain(domain.clone());
    }

    let mut clear_csrf = Cookie::build(("csrf_token", ""))
        .path("/")
        .http_only(false)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::ZERO);

    if let Some(ref domain) = config.cookie_domain {
        clear_csrf = clear_csrf.domain(domain.clone());
    }

    jar.add(clear_session.build())
        .add(clear_refresh.build())
        .add(clear_csrf.build())
}
