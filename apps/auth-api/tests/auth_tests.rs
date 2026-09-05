use uuid::Uuid;

#[test]
fn test_token_hashing_consistency() {
    use sha2::{Digest, Sha256};
    let token = "sample_test_refresh_token_string";
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let hash = hex::encode(hasher.finalize());

    assert_eq!(hash.len(), 64);

    let mut hasher2 = Sha256::new();
    hasher2.update(token.as_bytes());
    let hash2 = hex::encode(hasher2.finalize());

    assert_eq!(hash, hash2);
}

#[test]
fn test_argon2_password_hashing() {
    use argon2::password_hash::SaltString;
    use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
    use rand_core::OsRng;

    let password = "SuperSecretPassword123!";
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default());

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("hashing should succeed")
        .to_string();

    // Verify format adheres to Argon2id PHC string specification
    assert!(
        password_hash.starts_with("$argon2id$"),
        "Hash must use the Argon2id algorithm: {}",
        password_hash
    );

    let parsed = PasswordHash::new(&password_hash).expect("parsing should succeed");
    assert_eq!(parsed.algorithm, Algorithm::Argon2id.ident());
    assert!(argon2.verify_password(password.as_bytes(), &parsed).is_ok());
    assert!(argon2.verify_password(b"WrongPassword", &parsed).is_err());

    // Never accept plain SHA-256 as valid Argon2 hash
    let sha256_fake = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    assert!(PasswordHash::new(sha256_fake).is_err());

    // Unique salts ensure distinct hashes for identical passwords
    let salt2 = SaltString::generate(&mut OsRng);
    let password_hash2 = argon2
        .hash_password(password.as_bytes(), &salt2)
        .expect("hashing should succeed")
        .to_string();
    assert_ne!(password_hash, password_hash2);
}

#[test]
fn test_account_status_check() {
    use chrono::Utc;
    use uuid::Uuid;

    let active_user = auth_api::models::User {
        id: Uuid::new_v4(),
        email: "active@example.com".to_string(),
        display_name: "Active User".to_string(),
        password_hash: None,
        avatar_url: None,
        email_verified: true,
        status: "active".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    assert!(active_user.is_active());

    let suspended_user = auth_api::models::User {
        id: Uuid::new_v4(),
        email: "suspended@example.com".to_string(),
        display_name: "Suspended User".to_string(),
        password_hash: None,
        avatar_url: None,
        email_verified: true,
        status: "suspended".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    assert!(!suspended_user.is_active());
}

#[test]
fn test_secure_cookie_attributes() {
    use axum_extra::extract::cookie::{Cookie, SameSite};

    let session_token = "test_jwt_access_token_payload";
    let cookie = Cookie::build(("session_token", session_token))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(900))
        .build();

    assert_eq!(cookie.name(), "session_token");
    assert_eq!(cookie.value(), session_token);
    assert_eq!(cookie.http_only(), Some(true));
    assert_eq!(cookie.secure(), Some(true));
    assert_eq!(cookie.same_site(), Some(SameSite::Lax));
    assert_eq!(cookie.path(), Some("/"));
}

#[test]
fn test_uuid_generation() {
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    assert_ne!(id1, id2);
}
