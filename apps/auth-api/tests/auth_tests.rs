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
    use argon2::{
        Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    };
    use rand_core::OsRng;

    let password = "SuperSecretPassword123!";
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default());

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("hashing should succeed")
        .to_string();

    assert!(
        password_hash.starts_with("$argon2id$"),
        "Hash must use the Argon2id algorithm: {}",
        password_hash
    );

    let parsed = PasswordHash::new(&password_hash).expect("parsing should succeed");
    assert_eq!(parsed.algorithm, Algorithm::Argon2id.ident());
    assert!(argon2.verify_password(password.as_bytes(), &parsed).is_ok());
    assert!(argon2.verify_password(b"WrongPassword", &parsed).is_err());

    let sha256_fake = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    assert!(PasswordHash::new(sha256_fake).is_err());

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

#[test]
fn test_generate_and_validate_token_pair() {
    use auth_api::services::tokens::{
        generate_token_pair, validate_access_token, validate_refresh_token,
    };

    let user_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let secret = "super_secure_test_secret_key_that_is_at_least_32_chars!";

    let tokens = generate_token_pair(
        user_id,
        "user@example.com",
        "Test User",
        session_id,
        secret,
        900,
        604800,
    )
    .expect("token generation should succeed");

    assert_eq!(tokens.token_type, "Bearer");
    assert_eq!(tokens.expires_in, 900);

    let access_claims =
        validate_access_token(&tokens.access_token, secret).expect("access token should validate");
    assert_eq!(access_claims.sub, user_id.to_string());
    assert_eq!(access_claims.email, "user@example.com");
    assert_eq!(access_claims.display_name, "Test User");

    let refresh_claims = validate_refresh_token(&tokens.refresh_token, secret)
        .expect("refresh token should validate");
    assert_eq!(refresh_claims.sub, user_id.to_string());
    assert_eq!(refresh_claims.session_id, session_id.to_string());

    let wrong_secret = "completely_different_secret_key_at_least_32_chars!";
    assert!(validate_access_token(&tokens.access_token, wrong_secret).is_err());
    assert!(validate_refresh_token(&tokens.refresh_token, wrong_secret).is_err());
}

#[test]
fn test_email_verification_token() {
    use auth_api::services::tokens::{
        generate_email_verification_token, validate_email_verification_token,
    };

    let user_id = Uuid::new_v4();
    let secret = "super_secure_test_secret_key_that_is_at_least_32_chars!";

    let token = generate_email_verification_token(user_id, "verify@example.com", secret, 3600)
        .expect("verification token generation should succeed");

    let claims = validate_email_verification_token(&token, secret).expect("claims should validate");
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.email, "verify@example.com");
}

#[test]
fn test_cookie_helpers() {
    use auth_api::config::AppConfig;
    use auth_api::services::cookies::{
        build_refresh_cookie, build_session_cookie, clear_auth_cookies,
    };
    use axum_extra::extract::cookie::CookieJar;

    let config = AppConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
        database_url: "postgres://localhost/db".to_string(),
        db_max_connections: 10,
        redis_url: "redis://localhost:6379".to_string(),
        jwt_secret: "secret_that_is_long_enough_for_32_characters_validation".to_string(),
        jwt_access_expiry_secs: 900,
        jwt_refresh_expiry_secs: 604800,
        google_client_id: None,
        google_client_secret: None,
        google_redirect_uri: None,
        github_client_id: None,
        github_client_secret: None,
        github_redirect_uri: None,
        cors_origins: vec!["http://localhost:3000".to_string()],
        cookie_secure: true,
        cookie_domain: Some("example.com".to_string()),
        otel_exporter_otlp_endpoint: "http://localhost:4317".to_string(),
        otel_service_name: "auth-api".to_string(),
        otel_service_version: "0.1.0".to_string(),
    };

    let session_cookie = build_session_cookie(&config, "access_token_val".to_string());
    assert_eq!(session_cookie.name(), "session_token");
    assert_eq!(session_cookie.value(), "access_token_val");
    assert_eq!(session_cookie.secure(), Some(true));
    assert_eq!(session_cookie.domain(), Some("example.com"));

    let refresh_cookie = build_refresh_cookie(&config, "refresh_token_val".to_string());
    assert_eq!(refresh_cookie.name(), "refresh_token");
    assert_eq!(refresh_cookie.value(), "refresh_token_val");

    let jar = CookieJar::new();
    let cleared_jar = clear_auth_cookies(jar, &config);
    assert_eq!(cleared_jar.get("session_token").unwrap().value(), "");
    assert_eq!(cleared_jar.get("refresh_token").unwrap().value(), "");
}
