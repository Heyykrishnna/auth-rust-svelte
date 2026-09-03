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
    use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
    use rand_core::OsRng;

    let password = "SuperSecretPassword123!";
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("hashing should succeed")
        .to_string();

    let parsed = PasswordHash::new(&password_hash).expect("parsing should succeed");
    assert!(argon2.verify_password(password.as_bytes(), &parsed).is_ok());
    assert!(argon2.verify_password(b"WrongPassword", &parsed).is_err());
}

#[test]
fn test_uuid_generation() {
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    assert_ne!(id1, id2);
}
