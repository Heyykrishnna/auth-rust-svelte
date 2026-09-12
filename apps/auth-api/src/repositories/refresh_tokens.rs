use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::RefreshToken;

pub async fn create_refresh_token(pool: &PgPool, token: &RefreshToken) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (id, user_id, session_id, token_hash, family_id, expires_at, revoked_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#
    )
    .bind(token.id)
    .bind(token.user_id)
    .bind(token.session_id)
    .bind(&token.token_hash)
    .bind(token.family_id)
    .bind(token.expires_at)
    .bind(token.revoked_at)
    .bind(token.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_by_token_hash(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<RefreshToken>, AppError> {
    let token = sqlx::query_as::<_, RefreshToken>(
        r#"
        SELECT id, user_id, session_id, token_hash, family_id, expires_at, revoked_at, created_at
        FROM refresh_tokens
        WHERE token_hash = $1
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    Ok(token)
}

pub async fn revoke_token(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = NOW()
        WHERE id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn revoke_family(pool: &PgPool, family_id: Uuid) -> Result<u64, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = NOW()
        WHERE family_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(family_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn revoke_all_user_tokens(pool: &PgPool, user_id: Uuid) -> Result<u64, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = NOW()
        WHERE user_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
