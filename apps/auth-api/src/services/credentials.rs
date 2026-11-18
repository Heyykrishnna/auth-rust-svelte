use deadpool_redis::Pool as RedisPool;
use rand_core::{OsRng, RngCore};
use subtle::ConstantTimeEq;

use crate::errors::AppError;
use crate::models::{EnvironmentId, EnvironmentKind};
use crate::repositories::redis_keys;
use crate::services::tokens::hash_token;

pub const ALLOWED_SECRET_SCOPES: &[&str] = &[
    "users:read",
    "users:write",
    "sessions:read",
    "sessions:revoke",
    "orgs:read",
    "orgs:manage",
    "sync:read",
    "*",
];

pub struct GeneratedSecretKey {
    pub prefix: String,
    pub plaintext: String,
    pub hash: String,
}

pub fn publishable_prefix(kind: EnvironmentKind) -> &'static str {
    match kind {
        EnvironmentKind::Development => "pk_test_",
        EnvironmentKind::Production => "pk_live_",
    }
}

pub fn secret_prefix(kind: EnvironmentKind) -> &'static str {
    match kind {
        EnvironmentKind::Development => "sk_test_",
        EnvironmentKind::Production => "sk_live_",
    }
}

pub fn generate_publishable_key(kind: EnvironmentKind) -> (String, String) {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    let prefix = publishable_prefix(kind).to_string();
    let public_key = format!("{prefix}{}", hex::encode(bytes));
    (prefix, public_key)
}

pub fn generate_secret_key(kind: EnvironmentKind) -> GeneratedSecretKey {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let prefix = secret_prefix(kind).to_string();
    let plaintext = format!("{prefix}{}", hex::encode(bytes));
    let hash = hash_token(&plaintext);
    GeneratedSecretKey {
        prefix,
        plaintext,
        hash,
    }
}

pub fn constant_time_hash_eq(expected_hex: &str, candidate_hex: &str) -> bool {
    let a = expected_hex.as_bytes();
    let b = candidate_hex.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

pub fn validate_secret_scopes(scopes: &[String]) -> Result<Vec<String>, AppError> {
    if scopes.is_empty() {
        return Err(AppError::Validation(
            "Secret keys must specify at least one permission scope".to_string(),
        ));
    }

    let mut normalized = Vec::with_capacity(scopes.len());
    for scope in scopes {
        let trimmed = scope.trim();
        if !ALLOWED_SECRET_SCOPES.contains(&trimmed) {
            return Err(AppError::Validation(format!(
                "Unsupported secret key scope: {trimmed}"
            )));
        }
        if !normalized
            .iter()
            .any(|existing: &String| existing == trimmed)
        {
            normalized.push(trimmed.to_string());
        }
    }

    Ok(normalized)
}

pub fn key_matches_environment_kind(key: &str, kind: EnvironmentKind) -> bool {
    match kind {
        EnvironmentKind::Development => key.starts_with("pk_test_") || key.starts_with("sk_test_"),
        EnvironmentKind::Production => key.starts_with("pk_live_") || key.starts_with("sk_live_"),
    }
}

pub async fn invalidate_credential_cache(
    redis: &RedisPool,
    environment_id: EnvironmentId,
    key_digest: &str,
) {
    if let Ok(mut conn) = redis.get().await {
        let key = redis_keys::env_credential_cache_key(environment_id, key_digest);
        let _: Result<(), _> = redis::cmd("DEL").arg(&key).query_async(&mut conn).await;
    }
}

pub async fn should_record_last_used(
    redis: &RedisPool,
    environment_id: EnvironmentId,
    credential_id: uuid::Uuid,
) -> bool {
    let Ok(mut conn) = redis.get().await else {
        return true;
    };
    let key = redis_keys::env_key(environment_id, "cred_last_used", &credential_id.to_string());
    let set_result: Result<Option<String>, _> = redis::cmd("SET")
        .arg(&key)
        .arg("1")
        .arg("EX")
        .arg(60)
        .arg("NX")
        .query_async(&mut conn)
        .await;
    matches!(set_result, Ok(Some(_)))
}
