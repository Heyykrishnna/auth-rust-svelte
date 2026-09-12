use axum::routing::{get, post};
use axum::Router;

use crate::handlers;
use crate::middleware::rate_limit_layer;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(handlers::register::register))
        .route("/login", post(handlers::login::login))
        .route("/refresh", post(handlers::refresh::refresh))
        .route("/logout", post(handlers::logout::logout))
        .route(
            "/verify-email",
            get(handlers::verify_email::verify_email)
                .post(handlers::verify_email::verify_email_post),
        )
        .route("/verify-code", post(handlers::verify_email::verify_code))
        .route(
            "/forgot-password",
            post(handlers::password_reset::forgot_password),
        )
        .route(
            "/reset-password",
            post(handlers::password_reset::reset_password),
        )
        .route(
            "/me",
            get(handlers::users::get_me).route_layer(crate::middleware::require_permission(
                crate::models::Permission::ProfileRead,
            )),
        )
        .route("/oidc/:provider", get(handlers::oidc::oidc_url))
        .route(
            "/oidc/:provider/callback",
            post(handlers::oidc::oidc_callback),
        )
        .layer(rate_limit_layer())
}
