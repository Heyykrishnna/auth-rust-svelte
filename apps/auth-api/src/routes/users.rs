use axum::routing::{get, put};
use axum::Router;

use crate::handlers;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(handlers::users::get_me))
        .route("/profile", put(handlers::users::update_profile))
}
