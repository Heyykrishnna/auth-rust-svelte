use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::repositories::db::{begin_tx, commit_tx};
use crate::repositories::signing_keys::{self as key_repo, SigningKeyRecord};

pub const SIGNING_KEY_AAD_PURPOSE: &str = "signing_key";
pub const OAUTH_SECRET_AAD_PURPOSE: &str = "oauth_client_secret";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EcPublicJwk {
    pub kty: String,
    pub crv: String,
    #[serde(rename = "use")]
    pub key_use: String,
    pub alg: String,
    pub kid: String,
    pub x: String,
    pub y: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksDocument {
    pub keys: Vec<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct GeneratedEs256Key {
    pub kid: String,
    pub pkcs8_der: Vec<u8>,
    pub public_jwk: EcPublicJwk,
}

fn derive_kek_bytes(kek_secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(kek_secret.as_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

pub fn encrypt_envelope(
    plaintext: &[u8],
    kek_secret: &str,
    kek_version: &str,
    purpose: &str,
) -> Result<Vec<u8>, AppError> {
    let key_bytes = derive_kek_bytes(kek_secret);
    let unbound = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| AppError::Internal("Failed to initialize AES-256-GCM key".to_string()))?;
    let sealing_key = LessSafeKey::new(unbound);

    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| AppError::Internal("Failed to generate encryption nonce".to_string()))?;

    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let aad_str = format!("{purpose}:{kek_version}");
    let mut in_out = plaintext.to_vec();
    sealing_key
        .seal_in_place_append_tag(nonce, Aad::from(aad_str.as_bytes()), &mut in_out)
        .map_err(|_| AppError::Internal("Envelope encryption failed".to_string()))?;

    let mut combined = Vec::with_capacity(NONCE_LEN + in_out.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&in_out);
    Ok(combined)
}

pub fn decrypt_envelope(
    ciphertext: &[u8],
    kek_secret: &str,
    kek_version: &str,
    purpose: &str,
) -> Result<Vec<u8>, AppError> {
    if ciphertext.len() <= NONCE_LEN {
        return Err(AppError::Internal(
            "Encrypted key payload is truncated".to_string(),
        ));
    }

    let key_bytes = derive_kek_bytes(kek_secret);
    let unbound = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| AppError::Internal("Failed to initialize AES-256-GCM key".to_string()))?;
    let opening_key = LessSafeKey::new(unbound);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    nonce_bytes.copy_from_slice(&ciphertext[..NONCE_LEN]);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let aad_str = format!("{purpose}:{kek_version}");
    let mut encrypted_data = ciphertext[NONCE_LEN..].to_vec();
    let plaintext = opening_key
        .open_in_place(nonce, Aad::from(aad_str.as_bytes()), &mut encrypted_data)
        .map_err(|_| {
            AppError::Internal(
                "Envelope decryption failed (invalid KEK or corrupted payload)".to_string(),
            )
        })?;

    Ok(plaintext.to_vec())
}

pub fn generate_es256_keypair() -> Result<GeneratedEs256Key, AppError> {
    let rng = SystemRandom::new();
    let pkcs8_doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
        .map_err(|_| AppError::Internal("Failed to generate ES256 keypair".to_string()))?;
    let pkcs8_der = pkcs8_doc.as_ref().to_vec();

    let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &pkcs8_der, &rng)
        .map_err(|_| AppError::Internal("Failed to parse generated ES256 keypair".to_string()))?;

    let pub_bytes = key_pair.public_key().as_ref();
    if pub_bytes.len() != 65 || pub_bytes[0] != 0x04 {
        return Err(AppError::Internal(
            "Unexpected ECDSA P-256 public key encoding".to_string(),
        ));
    }

    let x = URL_SAFE_NO_PAD.encode(&pub_bytes[1..33]);
    let y = URL_SAFE_NO_PAD.encode(&pub_bytes[33..65]);
    let kid = format!("es256-{}", &Uuid::new_v4().simple().to_string()[..16]);

    let public_jwk = EcPublicJwk {
        kty: "EC".to_string(),
        crv: "P-256".to_string(),
        key_use: "sig".to_string(),
        alg: "ES256".to_string(),
        kid: kid.clone(),
        x,
        y,
    };

    Ok(GeneratedEs256Key {
        kid,
        pkcs8_der,
        public_jwk,
    })
}

pub async fn ensure_active_signing_key(
    db: &PgPool,
    config: &AppConfig,
) -> Result<SigningKeyRecord, AppError> {
    if let Some(existing) = key_repo::find_active_signing_key(db).await? {
        return Ok(existing);
    }

    let generated = generate_es256_keypair()?;
    let encrypted = encrypt_envelope(
        &generated.pkcs8_der,
        &config.signing_key_kek,
        &config.signing_key_kek_version,
        SIGNING_KEY_AAD_PURPOSE,
    )?;
    let jwk_val = serde_json::to_value(&generated.public_jwk)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let record = key_repo::insert_signing_key(
        db,
        &generated.kid,
        "ES256",
        jwk_val,
        &encrypted,
        &config.signing_key_kek_version,
        "active",
        Some(Utc::now()),
    )
    .await?;

    Ok(record)
}

pub async fn rotate_signing_key(
    db: &PgPool,
    config: &AppConfig,
) -> Result<SigningKeyRecord, AppError> {
    let generated = generate_es256_keypair()?;
    let encrypted = encrypt_envelope(
        &generated.pkcs8_der,
        &config.signing_key_kek,
        &config.signing_key_kek_version,
        SIGNING_KEY_AAD_PURPOSE,
    )?;
    let jwk_val = serde_json::to_value(&generated.public_jwk)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut tx = begin_tx(db).await?;
    key_repo::mark_active_keys_retiring(&mut *tx).await?;
    let new_active = key_repo::insert_signing_key(
        &mut *tx,
        &generated.kid,
        "ES256",
        jwk_val,
        &encrypted,
        &config.signing_key_kek_version,
        "active",
        Some(Utc::now()),
    )
    .await?;
    commit_tx(tx).await?;

    Ok(new_active)
}

pub async fn revoke_signing_key(db: &PgPool, kid: &str) -> Result<bool, AppError> {
    key_repo::revoke_signing_key_by_kid(db, kid).await
}

pub fn decrypt_signing_key_pkcs8(
    record: &SigningKeyRecord,
    config: &AppConfig,
) -> Result<Vec<u8>, AppError> {
    decrypt_envelope(
        &record.private_key_encrypted,
        &config.signing_key_kek,
        &record.kek_version,
        SIGNING_KEY_AAD_PURPOSE,
    )
}

pub async fn build_jwks_with_etag(
    db: &PgPool,
    config: &AppConfig,
) -> Result<(JwksDocument, String), AppError> {
    let mut keys = key_repo::list_jwks_signing_keys(db).await?;
    if keys.is_empty() {
        let active = ensure_active_signing_key(db, config).await?;
        keys.push(active);
    }

    let jwks = JwksDocument {
        keys: keys.into_iter().map(|k| k.public_jwk).collect(),
    };

    let serialized = serde_json::to_vec(&jwks).map_err(|e| AppError::Internal(e.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(&serialized);
    let etag = format!("\"{}\"", hex::encode(hasher.finalize()));

    Ok((jwks, etag))
}
