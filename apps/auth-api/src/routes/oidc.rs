use axum::routing::{get, post};
use axum::Router;

use crate::handlers::oidc_provider;
use crate::middleware::{api_rate_limit_layer, auth_rate_limit_layer};
use crate::AppState;

pub fn oidc_routes() -> Router<AppState> {
    let auth_endpoints = Router::new()
        .route("/authorize", get(oidc_provider::oauth2_authorize))
        .route("/token", post(oidc_provider::oauth2_token))
        .route("/revoke", post(oidc_provider::oauth2_revoke))
        .route(
            "/logout",
            get(oidc_provider::oauth2_logout_get).post(oidc_provider::oauth2_logout_post),
        )
        .route(
            "/portal/transactions/:tx_id/login",
            post(oidc_provider::portal_login),
        )
        .route(
            "/portal/transactions/:tx_id/sign-up",
            post(oidc_provider::portal_sign_up),
        )
        .route(
            "/portal/transactions/:tx_id/sign-up/verify",
            post(oidc_provider::portal_verify_sign_up),
        )
        .route(
            "/portal/transactions/:tx_id/consent",
            post(oidc_provider::portal_consent),
        )
        .layer(auth_rate_limit_layer());

    let read_endpoints = Router::new()
        .route("/userinfo", get(oidc_provider::oauth2_userinfo))
        .route(
            "/portal/transactions/:tx_id",
            get(oidc_provider::portal_get_transaction),
        )
        .route("/portal/account", get(oidc_provider::portal_get_account))
        .route(
            "/portal/sessions/:sso_id/revoke",
            post(oidc_provider::portal_revoke_sso_session),
        )
        .layer(api_rate_limit_layer());

    Router::new().merge(auth_endpoints).merge(read_endpoints)
}
