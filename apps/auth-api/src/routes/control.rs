use axum::routing::{delete, get, post};
use axum::Router;

use crate::handlers::control_plane;
use crate::middleware::{api_rate_limit_layer, auth_rate_limit_layer, sensitive_rate_limit_layer};
use crate::AppState;

pub fn control_routes() -> Router<AppState> {
    let auth_routes = Router::new()
        .route("/auth/sign-up", post(control_plane::developer_sign_up))
        .route("/auth/sign-in", post(control_plane::developer_sign_in))
        .route("/auth/sign-out", post(control_plane::developer_sign_out))
        .route("/auth/me", get(control_plane::developer_me))
        .layer(auth_rate_limit_layer());

    let credential_mutation_routes = Router::new()
        .route(
            "/applications/:app_id/environments/:env_kind/credentials",
            post(control_plane::create_credential),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/credentials/:cred_id/rotate",
            post(control_plane::rotate_credential),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/credentials/:cred_id",
            delete(control_plane::revoke_credential),
        )
        .layer(sensitive_rate_limit_layer());

    let management_routes = Router::new()
        .route(
            "/workspaces",
            get(control_plane::list_workspaces).post(control_plane::create_workspace),
        )
        .route(
            "/applications",
            get(control_plane::list_applications).post(control_plane::create_application),
        )
        .route(
            "/applications/:app_id",
            get(control_plane::get_application)
                .patch(control_plane::update_application)
                .delete(control_plane::delete_application),
        )
        .route(
            "/applications/:app_id/environments/:env_kind",
            get(control_plane::get_environment).patch(control_plane::update_environment),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/credentials",
            get(control_plane::list_credentials),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/domains",
            get(control_plane::list_domains).post(control_plane::create_domain),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/domains/:domain_id",
            delete(control_plane::delete_domain),
        )
        .layer(api_rate_limit_layer());

    Router::new()
        .merge(auth_routes)
        .merge(credential_mutation_routes)
        .merge(management_routes)
}

pub fn client_v1_routes() -> Router<AppState> {
    Router::new()
        .route("/config", get(control_plane::client_environment_config))
        .layer(api_rate_limit_layer())
}

pub fn backend_v1_routes() -> Router<AppState> {
    Router::new()
        .route("/ping", get(control_plane::backend_ping))
        .layer(api_rate_limit_layer())
}
