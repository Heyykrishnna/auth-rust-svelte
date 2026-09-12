use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::models::{Permission, Role};
use crate::repositories::authorization as authz_repo;
use crate::repositories::sessions as session_repo;

pub async fn resolve_user_roles_and_permissions(
    db: &PgPool,
    user_id: Uuid,
) -> Result<(HashSet<Role>, HashSet<Permission>), AppError> {
    let role_strings = authz_repo::get_user_roles(db, user_id).await?;
    let perm_strings = authz_repo::get_user_permissions(db, user_id).await?;

    let roles: HashSet<Role> = if role_strings.is_empty() {
        let mut set = HashSet::new();
        set.insert(Role::User);
        set
    } else {
        role_strings.into_iter().map(Role::from).collect()
    };

    let permissions: HashSet<Permission> = if perm_strings.is_empty() {
        let mut set = HashSet::new();
        set.insert(Permission::ProfileRead);
        set.insert(Permission::ProfileWrite);
        set.insert(Permission::SessionsRead);
        set.insert(Permission::SessionsDelete);
        set
    } else {
        perm_strings.into_iter().map(Permission::from).collect()
    };

    Ok((roles, permissions))
}

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

pub fn ensure_user_or_admin(
    user: &AuthenticatedUser,
    target_user_id: Uuid,
) -> Result<(), AppError> {
    if user.user_id == target_user_id || user.has_role(&Role::Admin) {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "Access denied to requested resource".to_string(),
        ))
    }
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

pub async fn ensure_session_ownership_or_admin(
    db: &PgPool,
    user: &AuthenticatedUser,
    session_id: Uuid,
) -> Result<(), AppError> {
    if user.has_role(&Role::Admin) {
        return Ok(());
    }
    ensure_session_ownership(db, user.user_id, session_id).await
}
