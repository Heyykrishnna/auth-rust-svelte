use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::models::SessionResponse;
use crate::services::authorization::ensure_session_ownership;
use crate::services::sessions as session_service;
use crate::AppState;

pub async fn list_sessions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<SessionResponse>>, AppError> {
    let sessions = session_service::list_user_sessions(&state.db, user.user_id, None).await?;
    Ok(Json(sessions))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    ensure_session_ownership(&state.db, user.user_id, session_id).await?;
    session_service::invalidate_session(&state.db, &state.redis, session_id).await?;

    let _ = crate::services::audit::log_event(
        &state.db,
        Some(user.user_id),
        crate::models::AuditEvent::SessionRevoked,
        None,
        None,
        serde_json::json!({ "session_id": session_id }),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}
