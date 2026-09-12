use axum::routing::{delete, get};
use axum::Router;

use crate::handlers;
use crate::middleware::require_permission;
use crate::models::Permission;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::sessions::list_sessions)
                .route_layer(require_permission(Permission::SessionsRead)),
        )
        .route(
            "/:id",
            delete(handlers::sessions::revoke_session)
                .route_layer(require_permission(Permission::SessionsDelete)),
        )
}
