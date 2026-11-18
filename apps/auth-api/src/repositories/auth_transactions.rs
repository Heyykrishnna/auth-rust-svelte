use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{EnvironmentId, UserId};
use crate::repositories::db::PgTx;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthTransactionRecord {
    pub id: Uuid,
    pub environment_id: EnvironmentId,
    pub flow_type: String,
    pub state_hash: Option<String>,
    pub nonce: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub authorization_code_hash: Option<String>,
    pub user_id: Option<UserId>,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(clippy::too_many_arguments)]
pub async fn create_auth_transaction<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    flow_type: &str,
    state_hash: Option<&str>,
    nonce: Option<&str>,
    code_challenge: Option<&str>,
    code_challenge_method: Option<&str>,
    redirect_uri: Option<&str>,
    scope: Option<&str>,
    authorization_code_hash: Option<&str>,
    user_id: Option<UserId>,
    expires_at: DateTime<Utc>,
) -> Result<AuthTransactionRecord, AppError>
where
    E: PgExecutor<'e>,
{
    let rec = sqlx::query_as::<_, AuthTransactionRecord>(
        r#"
        INSERT INTO authentication_transactions (
            environment_id, flow_type, state_hash, nonce, code_challenge, code_challenge_method,
            redirect_uri, scope, authorization_code_hash, user_id, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING id, environment_id, flow_type, state_hash, nonce, code_challenge, code_challenge_method, redirect_uri, scope, authorization_code_hash, user_id, status, expires_at, consumed_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(flow_type)
    .bind(state_hash)
    .bind(nonce)
    .bind(code_challenge)
    .bind(code_challenge_method)
    .bind(redirect_uri)
    .bind(scope)
    .bind(authorization_code_hash)
    .bind(user_id)
    .bind(expires_at)
    .fetch_one(executor)
    .await?;

    Ok(rec)
}

pub async fn consume_auth_transaction_by_state_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    flow_type: &str,
    state_hash: &str,
) -> Result<Option<AuthTransactionRecord>, AppError> {
    let rec = sqlx::query_as::<_, AuthTransactionRecord>(
        r#"
        SELECT id, environment_id, flow_type, state_hash, nonce, code_challenge, code_challenge_method, redirect_uri, scope, authorization_code_hash, user_id, status, expires_at, consumed_at, created_at, updated_at
        FROM authentication_transactions
        WHERE environment_id = $1 AND flow_type = $2 AND state_hash = $3
        FOR UPDATE
        "#,
    )
    .bind(environment_id)
    .bind(flow_type)
    .bind(state_hash)
    .fetch_optional(&mut **tx)
    .await?;

    let Some(record) = rec else {
        return Ok(None);
    };

    if record.consumed_at.is_some() || record.status != "pending" || Utc::now() > record.expires_at
    {
        return Ok(None);
    }

    sqlx::query(
        r#"
        UPDATE authentication_transactions
        SET status = 'consumed', consumed_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(record.id)
    .execute(&mut **tx)
    .await?;

    Ok(Some(record))
}

pub async fn find_pending_auth_transaction_by_state_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    flow_type: &str,
    state_hash: &str,
) -> Result<Option<AuthTransactionRecord>, AppError> {
    let rec = sqlx::query_as::<_, AuthTransactionRecord>(
        r#"
        SELECT id, environment_id, flow_type, state_hash, nonce, code_challenge, code_challenge_method, redirect_uri, scope, authorization_code_hash, user_id, status, expires_at, consumed_at, created_at, updated_at
        FROM authentication_transactions
        WHERE environment_id = $1 AND flow_type = $2 AND state_hash = $3
        FOR UPDATE
        "#,
    )
    .bind(environment_id)
    .bind(flow_type)
    .bind(state_hash)
    .fetch_optional(&mut **tx)
    .await?;

    match rec {
        Some(r)
            if r.consumed_at.is_none() && r.status == "pending" && Utc::now() <= r.expires_at =>
        {
            Ok(Some(r))
        }
        _ => Ok(None),
    }
}

pub async fn update_auth_transaction_scope_in_tx(
    tx: &mut PgTx<'_>,
    id: Uuid,
    scope: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE authentication_transactions
        SET scope = $2, expires_at = $3, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(scope)
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn mark_auth_transaction_consumed_in_tx(
    tx: &mut PgTx<'_>,
    id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE authentication_transactions
        SET status = 'consumed', consumed_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn revoke_auth_transaction_in_tx(tx: &mut PgTx<'_>, id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE authentication_transactions
        SET status = 'revoked', consumed_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn delete_pending_by_state_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    flow_type: &str,
    state_hash: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM authentication_transactions
        WHERE environment_id = $1 AND flow_type = $2 AND state_hash = $3
        "#,
    )
    .bind(environment_id)
    .bind(flow_type)
    .bind(state_hash)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
