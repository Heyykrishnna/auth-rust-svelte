use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor};
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SigningKeyRecord {
    pub id: Uuid,
    pub kid: String,
    pub algorithm: String,
    pub public_jwk: serde_json::Value,
    pub private_key_encrypted: Vec<u8>,
    pub kek_version: String,
    pub status: String,
    pub activated_at: Option<DateTime<Utc>>,
    pub retires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_signing_key<'e, E>(
    executor: E,
    kid: &str,
    algorithm: &str,
    public_jwk: serde_json::Value,
    private_key_encrypted: &[u8],
    kek_version: &str,
    status: &str,
    activated_at: Option<DateTime<Utc>>,
) -> Result<SigningKeyRecord, AppError>
where
    E: PgExecutor<'e>,
{
    let record = sqlx::query_as::<_, SigningKeyRecord>(
        r#"
        INSERT INTO signing_keys (
            kid, algorithm, public_jwk, private_key_encrypted, kek_version, status, activated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, kid, algorithm, public_jwk, private_key_encrypted, kek_version, status, activated_at, retires_at, revoked_at, created_at, updated_at
        "#,
    )
    .bind(kid)
    .bind(algorithm)
    .bind(public_jwk)
    .bind(private_key_encrypted)
    .bind(kek_version)
    .bind(status)
    .bind(activated_at)
    .fetch_one(executor)
    .await?;

    Ok(record)
}

pub async fn find_active_signing_key<'e, E>(
    executor: E,
) -> Result<Option<SigningKeyRecord>, AppError>
where
    E: PgExecutor<'e>,
{
    let record = sqlx::query_as::<_, SigningKeyRecord>(
        r#"
        SELECT id, kid, algorithm, public_jwk, private_key_encrypted, kek_version, status, activated_at, retires_at, revoked_at, created_at, updated_at
        FROM signing_keys
        WHERE status = 'active'
        ORDER BY activated_at DESC NULLS LAST, created_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(executor)
    .await?;

    Ok(record)
}

pub async fn find_verifiable_signing_key_by_kid<'e, E>(
    executor: E,
    kid: &str,
) -> Result<Option<SigningKeyRecord>, AppError>
where
    E: PgExecutor<'e>,
{
    let record = sqlx::query_as::<_, SigningKeyRecord>(
        r#"
        SELECT id, kid, algorithm, public_jwk, private_key_encrypted, kek_version, status, activated_at, retires_at, revoked_at, created_at, updated_at
        FROM signing_keys
        WHERE kid = $1 AND status IN ('active', 'retiring')
        LIMIT 1
        "#,
    )
    .bind(kid)
    .fetch_optional(executor)
    .await?;

    Ok(record)
}

pub async fn list_jwks_signing_keys<'e, E>(executor: E) -> Result<Vec<SigningKeyRecord>, AppError>
where
    E: PgExecutor<'e>,
{
    let records = sqlx::query_as::<_, SigningKeyRecord>(
        r#"
        SELECT id, kid, algorithm, public_jwk, private_key_encrypted, kek_version, status, activated_at, retires_at, revoked_at, created_at, updated_at
        FROM signing_keys
        WHERE status IN ('active', 'retiring')
        ORDER BY activated_at DESC NULLS LAST, created_at DESC
        "#,
    )
    .fetch_all(executor)
    .await?;

    Ok(records)
}

pub async fn mark_active_keys_retiring<'e, E>(executor: E) -> Result<u64, AppError>
where
    E: PgExecutor<'e>,
{
    let res = sqlx::query(
        r#"
        UPDATE signing_keys
        SET status = 'retiring', retires_at = NOW(), updated_at = NOW()
        WHERE status = 'active'
        "#,
    )
    .execute(executor)
    .await?;

    Ok(res.rows_affected())
}

pub async fn revoke_signing_key_by_kid<'e, E>(executor: E, kid: &str) -> Result<bool, AppError>
where
    E: PgExecutor<'e>,
{
    let res = sqlx::query(
        r#"
        UPDATE signing_keys
        SET status = 'revoked', revoked_at = NOW(), updated_at = NOW()
        WHERE kid = $1 AND status != 'revoked'
        "#,
    )
    .bind(kid)
    .execute(executor)
    .await?;

    Ok(res.rows_affected() > 0)
}
