use axum::routing::{get, post};
use axum::Router;

use crate::handlers;
use crate::middleware::{auth_rate_limit_layer, require_permission, sensitive_rate_limit_layer};
use crate::AppState;

pub fn routes() -> Router<AppState> {
    let sensitive_routes = Router::new()
        .route(
            "/forgot-password",
            post(handlers::password_reset::forgot_password),
        )
        .route(
            "/reset-password",
            post(handlers::password_reset::reset_password),
        )
        .route("/verify-code", post(handlers::verify_email::verify_code))
        .route(
            "/register/verify",
            post(handlers::register::verify_register_otp),
        )
        .route(
            "/register/resend-otp",
            post(handlers::register::resend_register_otp),
        )
        .layer(sensitive_rate_limit_layer());

    let auth_routes = Router::new()
        .route("/csrf", get(handlers::csrf::csrf_token))
        .route("/register", post(handlers::register::register))
        .route("/login", post(handlers::login::login))
        .route("/refresh", post(handlers::refresh::refresh))
        .route("/logout", post(handlers::logout::logout))
        .route(
            "/verify-email",
            get(handlers::verify_email::verify_email)
                .post(handlers::verify_email::verify_email_post),
        )
        .route(
            "/me",
            get(handlers::users::get_me)
                .route_layer(require_permission(crate::models::Permission::ProfileRead)),
        )
        .route("/oidc/:provider", get(handlers::oidc::oidc_url))
        .route(
            "/oidc/:provider/callback",
            post(handlers::oidc::oidc_callback),
        )
        .layer(auth_rate_limit_layer());

    Router::new().merge(sensitive_routes).merge(auth_routes)
}
