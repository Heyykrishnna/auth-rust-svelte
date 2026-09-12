use axum::routing::{delete, get, put};
use axum::Router;

use crate::handlers;
use crate::middleware::require_permission;
use crate::models::Permission;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/me",
            get(handlers::users::get_me).route_layer(require_permission(Permission::ProfileRead)),
        )
        .route(
            "/profile",
            put(handlers::users::update_profile)
                .route_layer(require_permission(Permission::ProfileWrite)),
        )
        .route(
            "/",
            get(handlers::users::list_users).route_layer(require_permission(Permission::UsersRead)),
        )
        .route(
            "/:id",
            delete(handlers::users::delete_user)
                .route_layer(require_permission(Permission::UsersDelete)),
        )
}
