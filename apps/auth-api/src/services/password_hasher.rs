use std::sync::OnceLock;
use std::time::Instant;

use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use rand_core::OsRng;
use tokio::sync::Semaphore;

use crate::errors::AppError;
use crate::services::metrics as platform_metrics;

static ARGON2_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();
static DUMMY_HASH: OnceLock<String> = OnceLock::new();

pub fn argon2_max_concurrency() -> usize {
    std::env::var("ARGON2_MAX_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|n| n.clamp(1, 32))
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get().clamp(2, 8))
                .unwrap_or(4)
        })
}

fn argon2_semaphore() -> &'static Semaphore {
    ARGON2_SEMAPHORE.get_or_init(|| Semaphore::new(argon2_max_concurrency()))
}

fn create_argon2id() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
}

fn dummy_password_hash() -> &'static str {
    DUMMY_HASH.get_or_init(|| {
        let salt = SaltString::generate(&mut OsRng);
        create_argon2id()
            .hash_password(b"dummy-timing-parity-secret", &salt)
            .map(|h| h.to_string())
            .unwrap_or_else(|_| {
                "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string()
            })
    })
}

pub async fn hash_password_async(password: String) -> Result<String, AppError> {
    let _permit = argon2_semaphore()
        .acquire()
        .await
        .map_err(|_| AppError::Internal("Password hashing pool unavailable".to_string()))?;

    let started = Instant::now();
    let result = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        create_argon2id()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AppError::PasswordHash(e.to_string()))
    })
    .await
    .map_err(|e| AppError::Internal(format!("Password hashing task failed: {e}")))?;

    platform_metrics::record_argon2_duration(started.elapsed().as_secs_f64());
    result
}

pub async fn verify_password_async(password: String, hash: String) -> Result<bool, AppError> {
    let _permit = argon2_semaphore()
        .acquire()
        .await
        .map_err(|_| AppError::Internal("Password verification pool unavailable".to_string()))?;

    let started = Instant::now();
    let result = tokio::task::spawn_blocking(move || {
        let Ok(parsed_hash) = PasswordHash::new(&hash) else {
            return false;
        };
        create_argon2id()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    })
    .await
    .map_err(|e| AppError::Internal(format!("Password verification task failed: {e}")))?;

    platform_metrics::record_argon2_duration(started.elapsed().as_secs_f64());
    Ok(result)
}

pub async fn verify_dummy_password_async(password: String) {
    let dummy = dummy_password_hash().to_string();
    let _ = verify_password_async(password, dummy).await;
}
