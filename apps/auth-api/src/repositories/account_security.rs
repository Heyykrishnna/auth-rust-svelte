use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    AuditLog, UpdateSecuritySettingsRequest, UserBackupCode, UserDevice, UserPasskey,
    UserSecurityAlert, UserSecuritySettings, UserTotpCredential,
};

pub async fn get_or_create_security_settings(
    db: &PgPool,
    user_id: Uuid,
) -> Result<UserSecuritySettings, AppError> {
    let existing = sqlx::query_as::<_, UserSecuritySettings>(
        r#"
        SELECT user_id, mfa_enabled, two_factor_method, recovery_email, recovery_phone,
               security_notifications_enabled, notify_new_login, notify_suspicious_activity,
               notify_security_changes, created_at, updated_at
        FROM user_security_settings
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?;

    if let Some(settings) = existing {
        return Ok(settings);
    }

    let created = sqlx::query_as::<_, UserSecuritySettings>(
        r#"
        INSERT INTO user_security_settings (user_id)
        VALUES ($1)
        ON CONFLICT (user_id) DO UPDATE SET updated_at = NOW()
        RETURNING user_id, mfa_enabled, two_factor_method, recovery_email, recovery_phone,
                  security_notifications_enabled, notify_new_login, notify_suspicious_activity,
                  notify_security_changes, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .fetch_one(db)
    .await?;

    Ok(created)
}

pub async fn update_security_settings(
    db: &PgPool,
    user_id: Uuid,
    req: &UpdateSecuritySettingsRequest,
) -> Result<UserSecuritySettings, AppError> {
    let current = get_or_create_security_settings(db, user_id).await?;

    let recovery_email = req
        .recovery_email
        .as_ref()
        .or(current.recovery_email.as_ref());
    let recovery_phone = req
        .recovery_phone
        .as_ref()
        .or(current.recovery_phone.as_ref());
    let security_notifications_enabled = req
        .security_notifications_enabled
        .unwrap_or(current.security_notifications_enabled);
    let notify_new_login = req.notify_new_login.unwrap_or(current.notify_new_login);
    let notify_suspicious_activity = req
        .notify_suspicious_activity
        .unwrap_or(current.notify_suspicious_activity);
    let notify_security_changes = req
        .notify_security_changes
        .unwrap_or(current.notify_security_changes);

    let updated = sqlx::query_as::<_, UserSecuritySettings>(
        r#"
        UPDATE user_security_settings
        SET recovery_email = $2,
            recovery_phone = $3,
            security_notifications_enabled = $4,
            notify_new_login = $5,
            notify_suspicious_activity = $6,
            notify_security_changes = $7,
            updated_at = NOW()
        WHERE user_id = $1
        RETURNING user_id, mfa_enabled, two_factor_method, recovery_email, recovery_phone,
                  security_notifications_enabled, notify_new_login, notify_suspicious_activity,
                  notify_security_changes, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(recovery_email)
    .bind(recovery_phone)
    .bind(security_notifications_enabled)
    .bind(notify_new_login)
    .bind(notify_suspicious_activity)
    .bind(notify_security_changes)
    .fetch_one(db)
    .await?;

    Ok(updated)
}

pub async fn set_mfa_enabled(
    db: &PgPool,
    user_id: Uuid,
    enabled: bool,
    method: &str,
) -> Result<(), AppError> {
    get_or_create_security_settings(db, user_id).await?;
    sqlx::query(
        r#"
        UPDATE user_security_settings
        SET mfa_enabled = $2,
            two_factor_method = $3,
            updated_at = NOW()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(enabled)
    .bind(method)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn upsert_totp_credential(
    db: &PgPool,
    user_id: Uuid,
    secret_base32: &str,
) -> Result<UserTotpCredential, AppError> {
    let credential = sqlx::query_as::<_, UserTotpCredential>(
        r#"
        INSERT INTO user_totp_credentials (user_id, secret_base32, verified)
        VALUES ($1, $2, FALSE)
        ON CONFLICT (user_id) DO UPDATE
        SET secret_base32 = EXCLUDED.secret_base32,
            verified = FALSE,
            updated_at = NOW()
        RETURNING id, user_id, secret_base32, verified, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(secret_base32)
    .fetch_one(db)
    .await?;

    Ok(credential)
}

pub async fn get_totp_credential(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Option<UserTotpCredential>, AppError> {
    let credential = sqlx::query_as::<_, UserTotpCredential>(
        r#"
        SELECT id, user_id, secret_base32, verified, created_at, updated_at
        FROM user_totp_credentials
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?;

    Ok(credential)
}

pub async fn verify_totp_credential(db: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE user_totp_credentials
        SET verified = TRUE, updated_at = NOW()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(db)
    .await?;

    set_mfa_enabled(db, user_id, true, "totp").await?;
    Ok(())
}

pub async fn delete_totp_credential(db: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM user_totp_credentials
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(db)
    .await?;

    set_mfa_enabled(db, user_id, false, "totp").await?;
    Ok(())
}

pub async fn create_passkey(
    db: &PgPool,
    user_id: Uuid,
    credential_id: &str,
    public_key_spki: &str,
    name: &str,
    aaguid: Option<&str>,
) -> Result<UserPasskey, AppError> {
    let passkey = sqlx::query_as::<_, UserPasskey>(
        r#"
        INSERT INTO user_passkeys (user_id, credential_id, public_key_spki, name, aaguid)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, credential_id, public_key_spki, name, aaguid, sign_count, created_at, last_used_at
        "#,
    )
    .bind(user_id)
    .bind(credential_id)
    .bind(public_key_spki)
    .bind(name)
    .bind(aaguid)
    .fetch_one(db)
    .await?;

    Ok(passkey)
}

pub async fn list_passkeys(db: &PgPool, user_id: Uuid) -> Result<Vec<UserPasskey>, AppError> {
    let passkeys = sqlx::query_as::<_, UserPasskey>(
        r#"
        SELECT id, user_id, credential_id, public_key_spki, name, aaguid, sign_count, created_at, last_used_at
        FROM user_passkeys
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(passkeys)
}

pub async fn delete_passkey(db: &PgPool, user_id: Uuid, passkey_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM user_passkeys
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(passkey_id)
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn store_backup_codes(
    db: &PgPool,
    user_id: Uuid,
    code_hashes: &[String],
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM user_backup_codes
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(db)
    .await?;

    for hash in code_hashes {
        sqlx::query(
            r#"
            INSERT INTO user_backup_codes (user_id, code_hash)
            VALUES ($1, $2)
            "#,
        )
        .bind(user_id)
        .bind(hash)
        .execute(db)
        .await?;
    }

    Ok(())
}

pub async fn list_unused_backup_codes(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserBackupCode>, AppError> {
    let codes = sqlx::query_as::<_, UserBackupCode>(
        r#"
        SELECT id, user_id, code_hash, used_at, created_at
        FROM user_backup_codes
        WHERE user_id = $1 AND used_at IS NULL
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(codes)
}

pub async fn count_unused_backup_codes(db: &PgPool, user_id: Uuid) -> Result<usize, AppError> {
    let row = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM user_backup_codes
        WHERE user_id = $1 AND used_at IS NULL
        "#,
    )
    .bind(user_id)
    .fetch_one(db)
    .await?;

    Ok(row as usize)
}

pub async fn mark_backup_code_used(
    db: &PgPool,
    user_id: Uuid,
    code_hash: &str,
) -> Result<bool, AppError> {
    let res = sqlx::query(
        r#"
        UPDATE user_backup_codes
        SET used_at = NOW()
        WHERE user_id = $1 AND code_hash = $2 AND used_at IS NULL
        "#,
    )
    .bind(user_id)
    .bind(code_hash)
    .execute(db)
    .await?;

    Ok(res.rows_affected() > 0)
}

pub async fn upsert_user_device(
    db: &PgPool,
    user_id: Uuid,
    fingerprint: &str,
    device_name: &str,
    device_type: &str,
    ip: Option<&str>,
    location: Option<&str>,
) -> Result<UserDevice, AppError> {
    let device = sqlx::query_as::<_, UserDevice>(
        r#"
        INSERT INTO user_devices (user_id, fingerprint, device_name, device_type, last_ip, last_location, last_active_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW())
        ON CONFLICT (user_id, fingerprint) DO UPDATE
        SET device_name = EXCLUDED.device_name,
            device_type = EXCLUDED.device_type,
            last_ip = COALESCE(EXCLUDED.last_ip, user_devices.last_ip),
            last_location = COALESCE(EXCLUDED.last_location, user_devices.last_location),
            last_active_at = NOW()
        RETURNING id, user_id, fingerprint, device_name, device_type, trusted, trusted_until,
                  last_ip, last_location, last_active_at, created_at
        "#,
    )
    .bind(user_id)
    .bind(fingerprint)
    .bind(device_name)
    .bind(device_type)
    .bind(ip)
    .bind(location)
    .fetch_one(db)
    .await?;

    Ok(device)
}

pub async fn list_user_devices(db: &PgPool, user_id: Uuid) -> Result<Vec<UserDevice>, AppError> {
    let devices = sqlx::query_as::<_, UserDevice>(
        r#"
        SELECT id, user_id, fingerprint, device_name, device_type, trusted, trusted_until,
               last_ip, last_location, last_active_at, created_at
        FROM user_devices
        WHERE user_id = $1
        ORDER BY last_active_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(devices)
}

pub async fn set_device_trusted(
    db: &PgPool,
    user_id: Uuid,
    device_id: Uuid,
    trusted: bool,
    duration_days: Option<i64>,
) -> Result<(), AppError> {
    let until = if trusted {
        Some(Utc::now() + Duration::days(duration_days.unwrap_or(30)))
    } else {
        None
    };

    sqlx::query(
        r#"
        UPDATE user_devices
        SET trusted = $3,
            trusted_until = $4
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(device_id)
    .bind(user_id)
    .bind(trusted)
    .bind(until)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn delete_device(db: &PgPool, user_id: Uuid, device_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM user_devices
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(device_id)
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn list_security_alerts(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserSecurityAlert>, AppError> {
    let alerts = sqlx::query_as::<_, UserSecurityAlert>(
        r#"
        SELECT id, user_id, alert_type, severity, title, description, ip_address, location, resolved, metadata, created_at
        FROM user_security_alerts
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(alerts)
}

pub async fn resolve_security_alert(
    db: &PgPool,
    user_id: Uuid,
    alert_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE user_security_alerts
        SET resolved = TRUE
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(alert_id)
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn get_recent_audit_logs(
    db: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<AuditLog>, AppError> {
    let logs = sqlx::query_as::<_, AuditLog>(
        r#"
        SELECT id, user_id, event, ip_address, user_agent, metadata, created_at
        FROM audit_logs
        WHERE user_id = $1 OR actor_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(logs)
}
