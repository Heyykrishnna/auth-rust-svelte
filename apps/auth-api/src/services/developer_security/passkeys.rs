use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, Passkey, PasskeyAuthentication, PasskeyRegistration,
    PublicKeyCredential, RegisterPublicKeyCredential, RequestChallengeResponse, Webauthn,
    WebauthnBuilder,
};

use crate::errors::AppError;
use crate::middleware::AuthenticatedDeveloper;
use crate::models::DeveloperId;
use crate::AppState;

use super::{console_origin, record_activity, redis_error, validate_label, verification};

#[derive(Serialize, FromRow)]
pub struct PasskeyInfo {
    id: Uuid,
    label: String,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}

pub fn available(state: &AppState) -> bool {
    webauthn(state).is_ok()
}

fn webauthn(state: &AppState) -> Result<Webauthn, AppError> {
    let origin = console_origin(&state.config)?;
    let rp_id = origin.host_str().ok_or_else(unavailable)?;
    WebauthnBuilder::new(rp_id, &origin)
        .and_then(|builder| builder.rp_name("Signet Developer Console").build())
        .map_err(|_| unavailable())
}

fn unavailable() -> AppError {
    AppError::Validation("Configure a secure Developer Console origin to use passkeys".to_string())
}

pub async fn list(state: &AppState, id: DeveloperId) -> Result<Vec<PasskeyInfo>, AppError> {
    Ok(sqlx::query_as(
        "SELECT id, label, created_at, last_used_at FROM developer_passkeys WHERE developer_id = $1 ORDER BY created_at",
    ).bind(id).fetch_all(&state.db).await?)
}

async fn credentials(state: &AppState, id: DeveloperId) -> Result<Vec<Passkey>, AppError> {
    let rows: Vec<sqlx::types::Json<Passkey>> = sqlx::query_scalar(
        "SELECT credential FROM developer_passkeys WHERE developer_id = $1 ORDER BY created_at LIMIT 20",
    ).bind(id).fetch_all(&state.db).await?;
    Ok(rows.into_iter().map(|row| row.0).collect())
}

pub async fn begin_registration(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
) -> Result<CreationChallengeResponse, AppError> {
    if !verification::proof_is_current(&state.db, dev.developer_id, dev.session_id).await? {
        return Err(AppError::StepUpRequired);
    }
    let credentials = credentials(state, dev.developer_id).await?;
    if credentials.len() >= 20 {
        return Err(AppError::Validation(
            "Remove a passkey before adding another".to_string(),
        ));
    }
    let (options, challenge) = webauthn(state)?
        .start_passkey_registration(
            dev.developer_id.into_inner(),
            &dev.email,
            &dev.display_name,
            Some(
                credentials
                    .iter()
                    .map(|credential| credential.cred_id().clone())
                    .collect(),
            ),
        )
        .map_err(|_| AppError::Validation("Could not start passkey registration".to_string()))?;
    store_challenge(state, dev, "register", &challenge).await?;
    Ok(options)
}

pub async fn finish_registration(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    label: &str,
    response: &RegisterPublicKeyCredential,
) -> Result<(), AppError> {
    let label = validate_label(label)?;
    let challenge: PasskeyRegistration = take_challenge(state, dev, "register").await?;
    let credential = webauthn(state)?
        .finish_passkey_registration(response, &challenge)
        .map_err(|_| {
            AppError::Validation(
                "Passkey confirmation failed. Start registration again.".to_string(),
            )
        })?;
    let credential_id = URL_SAFE_NO_PAD.encode(credential.cred_id());
    let mut tx = verification::lock_verified(&state.db, dev).await?;
    let inserted = sqlx::query(
        "INSERT INTO developer_passkeys (developer_id, credential_id, credential, label)
         SELECT $1, $2, $3, $4 WHERE (SELECT COUNT(*) FROM developer_passkeys WHERE developer_id = $1) < 20
         ON CONFLICT (credential_id) DO NOTHING",
    ).bind(dev.developer_id).bind(credential_id).bind(sqlx::types::Json(credential)).bind(label)
        .execute(&mut *tx).await?;
    if inserted.rows_affected() != 1 {
        return Err(AppError::Validation(
            "This passkey is already registered or the account has reached its limit".to_string(),
        ));
    }
    record_activity(&mut *tx, dev.developer_id, "DEVELOPER_PASSKEY_ADDED", None).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn remove(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    id: Uuid,
) -> Result<(), AppError> {
    let mut tx = verification::lock_verified(&state.db, dev).await?;
    let deleted = sqlx::query("DELETE FROM developer_passkeys WHERE developer_id = $1 AND id = $2")
        .bind(dev.developer_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if deleted.rows_affected() != 1 {
        return Err(AppError::NotFound("Passkey not found".to_string()));
    }
    record_activity(
        &mut *tx,
        dev.developer_id,
        "DEVELOPER_PASSKEY_REMOVED",
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn begin_verification(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
) -> Result<RequestChallengeResponse, AppError> {
    let credentials = credentials(state, dev.developer_id).await?;
    if credentials.is_empty() {
        return Err(AppError::NotFound(
            "Register a passkey before using it to verify your identity".to_string(),
        ));
    }
    let (options, challenge) = webauthn(state)?
        .start_passkey_authentication(&credentials)
        .map_err(|_| AppError::Validation("Could not start passkey verification".to_string()))?;
    store_challenge(state, dev, "verify", &challenge).await?;
    Ok(options)
}

pub async fn finish_verification(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    response: &PublicKeyCredential,
) -> Result<DateTime<Utc>, AppError> {
    let challenge: PasskeyAuthentication = take_challenge(state, dev, "verify").await?;
    let result = webauthn(state)?
        .finish_passkey_authentication(response, &challenge)
        .map_err(|_| {
            AppError::Validation(
                "Passkey verification failed. Start verification again.".to_string(),
            )
        })?;
    let id = URL_SAFE_NO_PAD.encode(result.cred_id());
    let mut tx = state.db.begin().await?;
    verification::lock_account(&mut tx, dev.developer_id).await?;
    let (mut credential, counter) = sqlx::query_as::<_, (sqlx::types::Json<Passkey>, i64)>(
        "SELECT credential, sign_count FROM developer_passkeys WHERE developer_id = $1 AND credential_id = $2 FOR UPDATE",
    ).bind(dev.developer_id).bind(&id).fetch_optional(&mut *tx).await?
        .ok_or_else(|| AppError::NotFound("Passkey not found".to_string()))?;
    let reported = i64::from(result.counter());
    if (reported != 0 || counter != 0) && reported <= counter {
        return Err(AppError::Validation(
            "This authenticator could not be verified".to_string(),
        ));
    }
    credential.0.update_credential(&result).ok_or_else(|| {
        AppError::Validation("This authenticator could not be verified".to_string())
    })?;
    sqlx::query(
        "UPDATE developer_passkeys SET credential = $3, sign_count = $4, last_used_at = NOW()
         WHERE developer_id = $1 AND credential_id = $2",
    )
    .bind(dev.developer_id)
    .bind(id)
    .bind(credential)
    .bind(reported)
    .execute(&mut *tx)
    .await?;
    let expiry = verification::mark_verified(&mut tx, dev).await?;
    record_activity(
        &mut *tx,
        dev.developer_id,
        "DEVELOPER_IDENTITY_VERIFIED",
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(expiry)
}

async fn store_challenge<T: Serialize>(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    flow: &str,
    challenge: &T,
) -> Result<(), AppError> {
    let data =
        serde_json::to_vec(challenge).map_err(|error| AppError::Internal(error.to_string()))?;
    let mut conn = state.redis.get().await.map_err(redis_error)?;
    redis::cmd("SETEX")
        .arg(challenge_key(dev, flow))
        .arg(300)
        .arg(data)
        .query_async(&mut conn)
        .await
        .map_err(redis_error)
}

async fn take_challenge<T: DeserializeOwned>(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    flow: &str,
) -> Result<T, AppError> {
    let mut conn = state.redis.get().await.map_err(redis_error)?;
    let data: Vec<u8> = redis::cmd("GETDEL")
        .arg(challenge_key(dev, flow))
        .query_async::<_, Option<Vec<u8>>>(&mut conn)
        .await
        .map_err(redis_error)?
        .ok_or_else(|| AppError::Validation("Passkey challenge expired. Try again.".to_string()))?;
    serde_json::from_slice(&data).map_err(|error| AppError::Internal(error.to_string()))
}

fn challenge_key(dev: &AuthenticatedDeveloper, flow: &str) -> String {
    format!(
        "developer:passkey:v1:{flow}:{}:{}",
        dev.developer_id, dev.session_id
    )
}
