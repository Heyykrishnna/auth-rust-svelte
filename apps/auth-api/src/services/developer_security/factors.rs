use chrono::Utc;
use rand_core::{OsRng, RngCore};
use serde::Serialize;
use sqlx::{Postgres, Transaction};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::middleware::AuthenticatedDeveloper;
use crate::models::DeveloperId;
use crate::services::{
    account_security::{base32_encode, build_totp_uri, compute_totp_code},
    signing_keys::{decrypt_envelope, encrypt_envelope},
    tokens::hash_token,
};
use crate::AppState;

use super::{record_activity, redis_error, revoke_other_sessions, verification};

#[derive(Serialize)]
pub struct Enrollment {
    secret: String,
    uri: String,
}

#[derive(Clone, Copy)]
pub enum VerifiedFactor {
    None,
    Authenticator,
    BackupCode,
}

pub async fn begin_enrollment(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
) -> Result<Enrollment, AppError> {
    if !verification::proof_is_current(&state.db, dev.developer_id, dev.session_id).await? {
        return Err(AppError::StepUpRequired);
    }
    if configured(&state.db, dev.developer_id).await? {
        return Err(AppError::Validation(
            "An authenticator is already enrolled".to_string(),
        ));
    }
    let mut secret = Zeroizing::new([0_u8; 20]);
    OsRng.fill_bytes(secret.as_mut());
    let encrypted = encrypt_envelope(
        secret.as_ref(),
        &state.config.signing_key_kek,
        &state.config.signing_key_kek_version,
        &purpose(dev.developer_id),
    )?;
    let mut conn = state.redis.get().await.map_err(redis_error)?;
    redis::cmd("SETEX")
        .arg(enrollment_key(dev))
        .arg(300)
        .arg(encrypted)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(redis_error)?;
    let secret = base32_encode(secret.as_ref());
    let uri = build_totp_uri(&secret, &dev.email, "Signet Developer");
    Ok(Enrollment { secret, uri })
}

pub async fn confirm_enrollment(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    code: &str,
) -> Result<Vec<String>, AppError> {
    let mut tx = verification::lock_verified(&state.db, dev).await?;
    let id = dev.developer_id;
    let key = enrollment_key(dev);
    let mut conn = state.redis.get().await.map_err(redis_error)?;
    let encrypted: Vec<u8> = redis::cmd("GET")
        .arg(&key)
        .query_async::<_, Option<Vec<u8>>>(&mut conn)
        .await
        .map_err(redis_error)?
        .ok_or_else(|| {
            AppError::Validation("Setup expired. Start authenticator setup again.".to_string())
        })?;
    let version = &state.config.signing_key_kek_version;
    let secret = decrypt_envelope(
        &encrypted,
        &state.config.signing_key_kek,
        version,
        &purpose(id),
    )?;
    let step = matching_step(&secret, code, Utc::now().timestamp(), -1)
        .ok_or(AppError::InvalidSecondFactor)?;
    let claimed: bool = redis::Script::new(
        "if redis.call('GET', KEYS[1]) == ARGV[1] then return redis.call('DEL', KEYS[1]) end return 0",
    ).key(&key).arg(&encrypted).invoke_async(&mut conn).await.map_err(redis_error)?;
    if !claimed {
        return Err(AppError::Validation(
            "Setup expired. Start authenticator setup again.".to_string(),
        ));
    }
    let added = sqlx::query(
        "INSERT INTO developer_security_settings (developer_id, totp_secret, totp_key_version, totp_last_step)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (developer_id) DO UPDATE SET totp_secret = EXCLUDED.totp_secret,
             totp_key_version = EXCLUDED.totp_key_version, totp_last_step = EXCLUDED.totp_last_step
         WHERE developer_security_settings.totp_secret IS NULL",
    ).bind(id).bind(&encrypted).bind(version).bind(step).execute(&mut *tx).await?;
    if added.rows_affected() != 1 {
        return Err(AppError::Validation(
            "An authenticator is already enrolled".to_string(),
        ));
    }
    let codes = replace_codes(&mut tx, id).await?;
    revoke_other_sessions(&mut tx, id, dev.session_id).await?;
    record_activity(&mut *tx, id, "DEVELOPER_MFA_ENABLED", None).await?;
    tx.commit().await?;
    Ok(codes)
}

pub async fn remove_authenticator(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
) -> Result<(), AppError> {
    let mut tx = verification::lock_verified(&state.db, dev).await?;
    let id = dev.developer_id;
    let updated = sqlx::query(
        "UPDATE developer_security_settings SET totp_secret = NULL, totp_key_version = NULL, totp_last_step = NULL
         WHERE developer_id = $1 AND totp_secret IS NOT NULL",
    ).bind(id).execute(&mut *tx).await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Authenticator not configured".to_string(),
        ));
    }
    sqlx::query("DELETE FROM developer_backup_codes WHERE developer_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    revoke_other_sessions(&mut tx, id, dev.session_id).await?;
    record_activity(&mut *tx, id, "DEVELOPER_MFA_DISABLED", None).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn rotate_backup_codes(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
) -> Result<Vec<String>, AppError> {
    let mut tx = verification::lock_verified(&state.db, dev).await?;
    if !configured(&mut *tx, dev.developer_id).await? {
        return Err(AppError::Validation(
            "Set up an authenticator first".to_string(),
        ));
    }
    let codes = replace_codes(&mut tx, dev.developer_id).await?;
    record_activity(
        &mut *tx,
        dev.developer_id,
        "DEVELOPER_BACKUP_CODES_REPLACED",
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(codes)
}

pub async fn verify_second_factor(
    tx: &mut Transaction<'_, Postgres>,
    config: &AppConfig,
    id: DeveloperId,
    code: Option<&str>,
) -> Result<VerifiedFactor, AppError> {
    let values = sqlx::query_as::<_, (Vec<u8>, String, i64)>(
        "SELECT totp_secret, totp_key_version, totp_last_step FROM developer_security_settings
         WHERE developer_id = $1 AND totp_secret IS NOT NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((ciphertext, version, last_step)) = values else {
        return Ok(VerifiedFactor::None);
    };
    let code = code
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .ok_or(AppError::SecondFactorRequired)?;
    if let Some(hash) = backup_code_hash(code) {
        let removed = sqlx::query(
            "DELETE FROM developer_backup_codes WHERE developer_id = $1 AND code_hash = $2",
        )
        .bind(id)
        .bind(hash)
        .execute(&mut **tx)
        .await?;
        if removed.rows_affected() != 1 {
            return Err(AppError::InvalidSecondFactor);
        }
        record_activity(&mut **tx, id, "DEVELOPER_BACKUP_CODE_USED", None).await?;
        return Ok(VerifiedFactor::BackupCode);
    }
    if version != config.signing_key_kek_version {
        return Err(AppError::Internal(
            "Developer authenticator encryption version is unavailable".to_string(),
        ));
    }
    let secret = decrypt_envelope(&ciphertext, &config.signing_key_kek, &version, &purpose(id))?;
    let step = matching_step(&secret, code, Utc::now().timestamp(), last_step)
        .ok_or(AppError::InvalidSecondFactor)?;
    sqlx::query(
        "UPDATE developer_security_settings SET totp_last_step = $2 WHERE developer_id = $1",
    )
    .bind(id)
    .bind(step)
    .execute(&mut **tx)
    .await?;
    Ok(VerifiedFactor::Authenticator)
}

async fn configured<'e>(db: impl sqlx::PgExecutor<'e>, id: DeveloperId) -> Result<bool, AppError> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM developer_security_settings WHERE developer_id = $1 AND totp_secret IS NOT NULL)")
        .bind(id).fetch_one(db).await?)
}

async fn replace_codes(
    tx: &mut Transaction<'_, Postgres>,
    id: DeveloperId,
) -> Result<Vec<String>, AppError> {
    sqlx::query("DELETE FROM developer_backup_codes WHERE developer_id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    let mut codes = Vec::with_capacity(10);
    let mut hashes = Vec::with_capacity(10);
    for _ in 0..10 {
        let mut bytes = Zeroizing::new([0_u8; 15]);
        OsRng.fill_bytes(bytes.as_mut());
        let raw = base32_encode(bytes.as_ref());
        hashes.push(hash_token(&raw));
        codes.push(format!(
            "{}-{}-{}-{}",
            &raw[..6],
            &raw[6..12],
            &raw[12..18],
            &raw[18..]
        ));
    }
    sqlx::query(
        "INSERT INTO developer_backup_codes (developer_id, code_hash) SELECT $1, unnest($2::text[])",
    ).bind(id).bind(hashes).execute(&mut **tx).await?;
    Ok(codes)
}

fn backup_code_hash(code: &str) -> Option<String> {
    if !(24..=27).contains(&code.len()) {
        return None;
    }
    let raw = code.replace('-', "").to_ascii_uppercase();
    (raw.len() == 24
        && raw
            .bytes()
            .all(|b| b.is_ascii_uppercase() || (b'2'..=b'7').contains(&b)))
    .then(|| hash_token(&raw))
}

fn matching_step(secret: &[u8], code: &str, now: i64, last_step: i64) -> Option<i64> {
    let code = code.trim();
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let now_step = now / 30;
    [now_step, now_step - 1, now_step + 1]
        .into_iter()
        .find(|step| {
            *step >= 0
                && *step > last_step
                && bool::from(
                    compute_totp_code(secret, (*step as u64) * 30)
                        .as_bytes()
                        .ct_eq(code.as_bytes()),
                )
        })
}

fn purpose(id: DeveloperId) -> String {
    format!("developer-totp:{id}")
}

fn enrollment_key(dev: &AuthenticatedDeveloper) -> String {
    format!("developer:totp:v1:{}:{}", dev.developer_id, dev.session_id)
}

#[cfg(test)]
mod tests {
    #[test]
    fn accepts_valid_counter_and_rejects_replay() {
        let secret = b"12345678901234567890";
        assert_eq!(super::matching_step(secret, "287082", 59, -1), Some(1));
        assert_eq!(super::matching_step(secret, "287082", 59, 1), None);
        assert_eq!(super::matching_step(secret, "287082", 120, -1), None);
    }
}
