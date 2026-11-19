use serde::Serialize;
use validator::ValidateEmail;

use crate::errors::AppError;
use crate::models::DeveloperId;
use crate::repositories::redis_ephemeral;
use crate::services::{
    email,
    password_hasher::hash_password_async,
    tokens::{generate_opaque_refresh_token, hash_token},
};
use crate::AppState;

use super::{console_origin, record_activity, verification, RevokeSessions};

#[derive(Serialize)]
pub struct DeliveryStatus {
    pub email_configured: bool,
    pub recovery_configured: bool,
}

pub fn delivery_status(state: &AppState) -> DeliveryStatus {
    let email_configured = email::smtp_configured(&state.config);
    DeliveryStatus {
        email_configured,
        recovery_configured: email_configured && console_origin(&state.config).is_ok(),
    }
}

pub async fn request(state: &AppState, address: &str) -> Result<(), AppError> {
    let address = redis_ephemeral::normalize_email(address);
    if address.len() > 254 || !address.validate_email() {
        return Err(AppError::Validation(
            "Enter a valid email address".to_string(),
        ));
    }
    let origin = console_origin(&state.config)?;
    if !email::smtp_configured(&state.config) {
        return Err(AppError::Validation(
            "Recovery email delivery is unavailable".to_string(),
        ));
    }
    let token = generate_opaque_refresh_token();
    let ttl = state.config.password_reset_expiry_secs.clamp(300, 1800);
    let changed = sqlx::query(
        "INSERT INTO developer_password_resets (developer_id, token_hash, expires_at)
         SELECT id, $2, NOW() + make_interval(secs => $3) FROM developers
         WHERE LOWER(email) = $1 AND status = 'active' AND password_hash IS NOT NULL
         ON CONFLICT (developer_id) DO UPDATE SET token_hash = EXCLUDED.token_hash, expires_at = EXCLUDED.expires_at
         WHERE developer_password_resets.expires_at <= EXCLUDED.expires_at - INTERVAL '1 minute'",
    ).bind(&address).bind(hash_token(&token)).bind(ttl as f64).execute(&state.db).await?;
    if changed.rows_affected() == 1 {
        let state = state.clone();
        tokio::spawn(async move {
            let mut recovery_url = origin;
            recovery_url.set_path("/control/login/recovery");
            recovery_url.set_fragment(Some(&format!("token={token}")));
            if email::send_developer_recovery_email(
                &state.config,
                &address,
                recovery_url.as_str(),
                ttl,
            )
            .await
            .is_err()
            {
                tracing::warn!("Developer password recovery email could not be delivered");
            }
        });
    }
    Ok(())
}

pub async fn finish(state: &AppState, token: &str, password: String) -> Result<(), AppError> {
    if token.is_empty() || token.len() > 128 {
        return Err(invalid_link());
    }
    if !(12..=128).contains(&password.len()) {
        return Err(AppError::Validation(
            "New password must be between 12 and 128 characters".to_string(),
        ));
    }
    let digest = hash_token(token);
    let id = sqlx::query_scalar::<_, DeveloperId>(
        "SELECT developer_id FROM developer_password_resets WHERE token_hash = $1 AND expires_at > NOW()",
    ).bind(&digest).fetch_optional(&state.db).await?.ok_or_else(invalid_link)?;
    let password_hash = hash_password_async(password).await?;
    let mut tx = state.db.begin().await?;
    verification::lock_account(&mut tx, id).await?;
    let deleted = sqlx::query(
        "DELETE FROM developer_password_resets WHERE developer_id = $1 AND token_hash = $2 AND expires_at > NOW()",
    ).bind(id).bind(digest).execute(&mut *tx).await?;
    if deleted.rows_affected() != 1 {
        return Err(invalid_link());
    }
    sqlx::query("UPDATE developers SET password_hash = $2, email_verified = TRUE WHERE id = $1")
        .bind(id)
        .bind(password_hash)
        .execute(&mut *tx)
        .await?;
    super::revoke_account_sessions(&mut tx, id, RevokeSessions::All).await?;
    sqlx::query("DELETE FROM developer_trusted_devices WHERE developer_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    record_activity(&mut *tx, id, "DEVELOPER_PASSWORD_RESET", None).await?;
    tx.commit().await?;
    Ok(())
}

fn invalid_link() -> AppError {
    AppError::Validation("The recovery link is invalid or expired".to_string())
}
