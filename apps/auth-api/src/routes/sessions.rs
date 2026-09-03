use axum::routing::{delete, get};
use axum::Router;

use crate::handlers;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::sessions::list_sessions))
        .route("/:id", delete(handlers::sessions::revoke_session))
}
