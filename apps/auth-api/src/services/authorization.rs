use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::repositories::sessions as session_repo;

pub fn ensure_user_matches(
    authenticated_user_id: Uuid,
    target_user_id: Uuid,
) -> Result<(), AppError> {
    if authenticated_user_id != target_user_id {
        return Err(AppError::Forbidden(
            "Access denied to requested resource".to_string(),
        ));
    }
    Ok(())
}

pub async fn ensure_session_ownership(
    db: &PgPool,
    authenticated_user_id: Uuid,
    session_id: Uuid,
) -> Result<(), AppError> {
    let session = session_repo::find_pg_session_by_id(db, session_id)
        .await?
        .ok_or(AppError::SessionNotFound)?;

    if session.user_id != authenticated_user_id {
        return Err(AppError::Forbidden(
            "Access denied to requested session".to_string(),
        ));
    }

    Ok(())
}
