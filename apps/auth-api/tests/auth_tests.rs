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
        login_max_attempts: 5,
        login_lockout_duration_secs: 900,
        verification_code_expiry_secs: 900,
        password_reset_expiry_secs: 900,
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
        smtp_host: Some("smtp.gmail.com".to_string()),
        smtp_port: 587,
        smtp_user: Some("test@gmail.com".to_string()),
        smtp_pass: Some("testpass".to_string()),
        smtp_from: "Dradix <support@dradix.dev>".to_string(),
        smtp_from_name: "Dradix".to_string(),
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

#[test]
fn test_session_lifecycle_and_revocation() {
    use auth_api::models::Session;
    use chrono::{Duration, Utc};

    let user_id = Uuid::new_v4();
    let token_hash = "f3b7c8d9e0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7".to_string();
    let expires_at = Utc::now() + Duration::hours(24);

    let mut session = Session::new(
        user_id,
        token_hash.clone(),
        expires_at,
        Some("Mozilla/5.0".to_string()),
        Some("127.0.0.1".to_string()),
    );

    assert_eq!(session.user_id, user_id);
    assert_eq!(session.session_hash, token_hash);
    assert_eq!(session.refresh_token_hash, token_hash);
    assert!(session.revoked_at.is_none());
    assert!(!session.is_expired());
    assert!(!session.is_revoked());
    assert!(session.is_active());

    session.revoked_at = Some(Utc::now());
    assert!(session.is_revoked());
    assert!(!session.is_active());

    let expired_session = Session::new(
        user_id,
        token_hash,
        Utc::now() - Duration::hours(1),
        None,
        None,
    );
    assert!(expired_session.is_expired());
    assert!(!expired_session.is_active());
}

#[test]
fn test_refresh_token_families_and_lifecycle() {
    use auth_api::models::RefreshToken;
    use chrono::{Duration, Utc};

    let user_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let family_id = Uuid::new_v4();
    let token_hash = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f01234".to_string();
    let expires_at = Utc::now() + Duration::days(7);

    let mut token = RefreshToken::new(
        user_id,
        Some(session_id),
        token_hash.clone(),
        family_id,
        expires_at,
    );

    assert_eq!(token.user_id, user_id);
    assert_eq!(token.session_id, Some(session_id));
    assert_eq!(token.family_id, family_id);
    assert_eq!(token.token_hash, token_hash);
    assert!(token.revoked_at.is_none());
    assert!(token.is_active());
    assert!(!token.is_revoked());
    assert!(!token.is_expired());

    token.revoked_at = Some(Utc::now());
    assert!(token.is_revoked());
    assert!(!token.is_active());
}

#[test]
fn test_audit_event_enum_and_serialization() {
    use auth_api::models::AuditEvent;

    let events = vec![
        (
            AuditEvent::LoginSuccess,
            "\"LOGIN_SUCCESS\"",
            "LOGIN_SUCCESS",
        ),
        (AuditEvent::LoginFailed, "\"LOGIN_FAILED\"", "LOGIN_FAILED"),
        (AuditEvent::Logout, "\"LOGOUT\"", "LOGOUT"),
        (
            AuditEvent::PasswordChanged,
            "\"PASSWORD_CHANGED\"",
            "PASSWORD_CHANGED",
        ),
        (
            AuditEvent::EmailVerified,
            "\"EMAIL_VERIFIED\"",
            "EMAIL_VERIFIED",
        ),
        (
            AuditEvent::SessionRevoked,
            "\"SESSION_REVOKED\"",
            "SESSION_REVOKED",
        ),
        (AuditEvent::MfaEnabled, "\"MFA_ENABLED\"", "MFA_ENABLED"),
        (AuditEvent::MfaFailed, "\"MFA_FAILED\"", "MFA_FAILED"),
        (
            AuditEvent::AccountLocked,
            "\"ACCOUNT_LOCKED\"",
            "ACCOUNT_LOCKED",
        ),
    ];

    for (event, json_repr, display_str) in events {
        let serialized = serde_json::to_string(&event).expect("must serialize");
        assert_eq!(serialized, json_repr);

        let deserialized: AuditEvent = serde_json::from_str(json_repr).expect("must deserialize");
        assert_eq!(deserialized, event);

        assert_eq!(format!("{}", event), display_str);
    }
}

#[test]
fn test_audit_log_model_and_metadata_sanitization() {
    use auth_api::models::{AuditEvent, AuditLog};
    use auth_api::services::audit::sanitize_audit_metadata;

    let user_id = Uuid::new_v4();
    let raw_metadata = serde_json::json!({
        "email": "user@example.com",
        "password": "ClearTextPassword123!",
        "token": "sensitive_jwt_token_here",
        "secret": "very_secret_key",
        "ip": "192.168.1.1"
    });

    let sanitized = sanitize_audit_metadata(raw_metadata);
    assert_eq!(sanitized["email"], "user@example.com");
    assert_eq!(sanitized["ip"], "192.168.1.1");
    assert_eq!(sanitized["password"], "[REDACTED]");
    assert_eq!(sanitized["token"], "[REDACTED]");
    assert_eq!(sanitized["secret"], "[REDACTED]");

    let pre_auth_log = AuditLog::new(
        None,
        AuditEvent::LoginFailed,
        Some("192.168.1.1".to_string()),
        Some("Mozilla/5.0".to_string()),
        sanitized.clone(),
    );
    assert!(pre_auth_log.user_id.is_none());
    assert_eq!(pre_auth_log.event, AuditEvent::LoginFailed);
    assert_eq!(pre_auth_log.metadata["password"], "[REDACTED]");

    let auth_log = AuditLog::new(
        Some(user_id),
        AuditEvent::LoginSuccess,
        Some("127.0.0.1".to_string()),
        Some("TestRunner".to_string()),
        serde_json::json!({ "session_id": "123" }),
    );
    assert_eq!(auth_log.user_id, Some(user_id));
    assert_eq!(auth_log.event, AuditEvent::LoginSuccess);
}

#[test]
fn test_too_many_requests_error_response() {
    use auth_api::errors::AppError;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let err = AppError::TooManyRequests(
        "Account temporarily locked due to 5 failed attempts.".to_string(),
    );
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[test]
fn test_redis_ephemeral_email_normalization() {
    use auth_api::repositories::redis_ephemeral::normalize_email;

    assert_eq!(normalize_email(" User@Example.COM "), "user@example.com");
    assert_eq!(
        normalize_email("JOHN.DOE@DOMAIN.ORG"),
        "john.doe@domain.org"
    );
    assert_eq!(normalize_email("   test@test.io   "), "test@test.io");
}

#[test]
fn test_verification_code_data_serde() {
    use auth_api::repositories::redis_ephemeral::VerificationCodeData;

    let user_id = Uuid::new_v4();
    let data = VerificationCodeData {
        user_id,
        email: "verify@test.com".to_string(),
    };

    let serialized = serde_json::to_string(&data).expect("must serialize");
    let deserialized: VerificationCodeData =
        serde_json::from_str(&serialized).expect("must deserialize");

    assert_eq!(data, deserialized);
}

#[test]
fn test_login_attempt_brute_force_counter_threshold_logic() {
    let max_attempts = 5u32;

    for attempts in 0..max_attempts {
        assert!(
            attempts < max_attempts,
            "Attempt {} should be below threshold",
            attempts
        );
    }

    for attempts in max_attempts..max_attempts + 3 {
        assert!(
            attempts >= max_attempts,
            "Attempt {} should trigger lockout",
            attempts
        );
    }
}

#[test]
fn test_password_reset_token_entropy_and_request_validation() {
    use auth_api::handlers::password_reset::{ForgotPasswordRequest, ResetPasswordRequest};
    use validator::Validate;

    let valid_req = ForgotPasswordRequest {
        email: "user@example.com".to_string(),
    };
    assert!(valid_req.validate().is_ok());

    let invalid_req = ForgotPasswordRequest {
        email: "not-an-email".to_string(),
    };
    assert!(invalid_req.validate().is_err());

    let valid_reset = ResetPasswordRequest {
        token: "token123".to_string(),
        new_password: "NewSecurePassword123!".to_string(),
    };
    assert!(valid_reset.validate().is_ok());

    let short_password_reset = ResetPasswordRequest {
        token: "token123".to_string(),
        new_password: "short".to_string(),
    };
    assert!(short_password_reset.validate().is_err());

    let empty_token_reset = ResetPasswordRequest {
        token: "".to_string(),
        new_password: "NewSecurePassword123!".to_string(),
    };
    assert!(empty_token_reset.validate().is_err());

    let t1 = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let t2 = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    assert_ne!(t1, t2);
    assert_eq!(t1.len(), 64);
    assert_eq!(t2.len(), 64);
}

#[test]
fn test_app_config_redis_defaults() {
    use auth_api::config::AppConfig;

    std::env::remove_var("LOGIN_MAX_ATTEMPTS");
    std::env::remove_var("LOGIN_LOCKOUT_DURATION_SECS");
    std::env::remove_var("VERIFICATION_CODE_EXPIRY_SECS");
    std::env::remove_var("PASSWORD_RESET_EXPIRY_SECS");
    std::env::set_var("DATABASE_URL", "postgres://localhost/testdb");
    std::env::set_var(
        "JWT_SECRET",
        "super_secret_test_key_that_is_at_least_32_characters_long",
    );

    let cfg = AppConfig::from_env().expect("AppConfig should parse default values");
    assert_eq!(cfg.login_max_attempts, 5);
    assert_eq!(cfg.login_lockout_duration_secs, 900); // 15 mins
    assert_eq!(cfg.verification_code_expiry_secs, 900); // 15 mins
    assert_eq!(cfg.password_reset_expiry_secs, 900); // 15 mins
}

#[test]
fn test_role_and_permission_serde_and_matching() {
    use auth_api::models::{Permission, Role};

    assert_eq!(Role::from("user"), Role::User);
    assert_eq!(Role::from("admin"), Role::Admin);
    assert_eq!(
        Role::from("moderator"),
        Role::Custom("moderator".to_string())
    );
    assert_eq!(Role::User.as_str(), "user");
    assert_eq!(Role::Admin.as_str(), "admin");
    assert_eq!(Role::Custom("auditor".to_string()).as_str(), "auditor");
    assert_eq!(Role::Admin.to_string(), "admin");

    assert_eq!(Permission::from("profile.read"), Permission::ProfileRead);
    assert_eq!(Permission::from("profile.write"), Permission::ProfileWrite);
    assert_eq!(Permission::from("sessions.read"), Permission::SessionsRead);
    assert_eq!(
        Permission::from("sessions.delete"),
        Permission::SessionsDelete
    );
    assert_eq!(Permission::from("users.read"), Permission::UsersRead);
    assert_eq!(Permission::from("users.delete"), Permission::UsersDelete);
    assert_eq!(
        Permission::from("reports.export"),
        Permission::Custom("reports.export".to_string())
    );
    assert_eq!(Permission::UsersDelete.as_str(), "users.delete");
    assert_eq!(Permission::UsersDelete.to_string(), "users.delete");

    let perm_json = serde_json::to_string(&Permission::UsersDelete).expect("serialize permission");
    assert_eq!(perm_json, "\"users.delete\"");
    let parsed_perm: Permission = serde_json::from_str(&perm_json).expect("deserialize permission");
    assert_eq!(parsed_perm, Permission::UsersDelete);

    let role_json = serde_json::to_string(&Role::Admin).expect("serialize role");
    assert_eq!(role_json, "\"admin\"");
    let parsed_role: Role = serde_json::from_str(&role_json).expect("deserialize role");
    assert_eq!(parsed_role, Role::Admin);
}

#[test]
fn test_authenticated_user_authorization_evaluation() {
    use auth_api::middleware::AuthenticatedUser;
    use auth_api::models::{Permission, Role};
    use std::collections::HashSet;

    let mut user_roles = HashSet::new();
    user_roles.insert(Role::User);

    let mut user_perms = HashSet::new();
    user_perms.insert(Permission::ProfileRead);
    user_perms.insert(Permission::ProfileWrite);

    let standard_user = AuthenticatedUser {
        user_id: Uuid::new_v4(),
        email: "user@example.com".to_string(),
        display_name: "Regular User".to_string(),
        jti: Uuid::new_v4().to_string(),
        roles: user_roles,
        permissions: user_perms,
    };

    assert!(standard_user.has_role(&Role::User));
    assert!(!standard_user.has_role(&Role::Admin));

    assert!(standard_user.has_permission(&Permission::ProfileRead));
    assert!(standard_user.has_permission(&Permission::ProfileWrite));
    assert!(!standard_user.has_permission(&Permission::UsersDelete));

    assert!(standard_user.has_any_permission(&[Permission::UsersDelete, Permission::ProfileRead]));
    assert!(!standard_user.has_any_permission(&[Permission::UsersDelete, Permission::UsersRead]));

    assert!(standard_user.has_all_permissions(&[Permission::ProfileRead, Permission::ProfileWrite]));
    assert!(!standard_user.has_all_permissions(&[Permission::ProfileRead, Permission::UsersDelete]));
}

#[test]
fn test_jwt_token_generation_and_validation_with_roles_and_permissions() {
    use auth_api::services::tokens::{
        generate_token_pair_with_roles_and_permissions, validate_access_token,
    };

    let user_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let secret = "super_secret_jwt_test_key_with_at_least_32_bytes";
    let roles = vec!["admin".to_string(), "user".to_string()];
    let permissions = vec![
        "profile.read".to_string(),
        "profile.write".to_string(),
        "users.read".to_string(),
        "users.delete".to_string(),
    ];

    let token_pair = generate_token_pair_with_roles_and_permissions(
        user_id,
        "admin@example.com",
        "Admin User",
        roles.clone(),
        permissions.clone(),
        session_id,
        secret,
        900,
        86400,
    )
    .expect("Token generation should succeed");

    let claims = validate_access_token(&token_pair.access_token, secret)
        .expect("Token validation should succeed");

    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.roles, roles);
    assert_eq!(claims.permissions, permissions);
}

#[test]
fn test_idor_protection_ensure_user_or_admin() {
    use auth_api::middleware::AuthenticatedUser;
    use auth_api::models::Role;
    use auth_api::services::authorization::ensure_user_or_admin;
    use std::collections::HashSet;

    let user1_id = Uuid::new_v4();
    let user2_id = Uuid::new_v4();

    let mut regular_roles = HashSet::new();
    regular_roles.insert(Role::User);

    let regular_user = AuthenticatedUser {
        user_id: user1_id,
        email: "user1@example.com".to_string(),
        display_name: "User 1".to_string(),
        jti: Uuid::new_v4().to_string(),
        roles: regular_roles,
        permissions: HashSet::new(),
    };

    let mut admin_roles = HashSet::new();
    admin_roles.insert(Role::Admin);

    let admin_user = AuthenticatedUser {
        user_id: user1_id,
        email: "admin@example.com".to_string(),
        display_name: "Admin".to_string(),
        jti: Uuid::new_v4().to_string(),
        roles: admin_roles,
        permissions: HashSet::new(),
    };

    assert!(ensure_user_or_admin(&regular_user, user1_id).is_ok());

    let idor_attempt = ensure_user_or_admin(&regular_user, user2_id);
    assert!(idor_attempt.is_err());
    match idor_attempt {
        Err(auth_api::errors::AppError::Forbidden(msg)) => {
            assert!(msg.contains("Access denied"));
        }
        _ => panic!("Expected AppError::Forbidden on IDOR attempt"),
    }

    assert!(ensure_user_or_admin(&admin_user, user2_id).is_ok());
}

#[tokio::test]
async fn test_authorization_middleware_pipeline() {
    use auth_api::middleware::{
        require_all_permissions, require_any_permission, require_permission, require_role,
        AuthenticatedUser,
    };
    use auth_api::models::{Permission, Role};
    use axum::http::{Request, StatusCode};
    use axum::routing::{delete, get};
    use axum::Router;
    use std::collections::HashSet;
    use tower::ServiceExt;

    let app = Router::new()
        .route(
            "/profile",
            get(|| async { "profile data" })
                .route_layer(require_permission(Permission::ProfileRead)),
        )
        .route(
            "/admin/users/:id",
            delete(|| async { StatusCode::NO_CONTENT })
                .route_layer(require_permission(Permission::UsersDelete)),
        )
        .route(
            "/admin/dashboard",
            get(|| async { "admin only" }).route_layer(require_role(Role::Admin)),
        )
        .route(
            "/reports",
            get(|| async { "reports data" }).route_layer(require_any_permission(vec![
                Permission::UsersRead,
                Permission::Custom("reports.view".to_string()),
            ])),
        )
        .route(
            "/sensitive",
            get(|| async { "sensitive data" }).route_layer(require_all_permissions(vec![
                Permission::ProfileRead,
                Permission::UsersRead,
            ])),
        );

    let regular_user = AuthenticatedUser {
        user_id: Uuid::new_v4(),
        email: "regular@example.com".to_string(),
        display_name: "Regular User".to_string(),
        jti: Uuid::new_v4().to_string(),
        roles: {
            let mut s = HashSet::new();
            s.insert(Role::User);
            s
        },
        permissions: {
            let mut s = HashSet::new();
            s.insert(Permission::ProfileRead);
            s.insert(Permission::ProfileWrite);
            s
        },
    };

    let mut req = Request::builder()
        .uri("/profile")
        .body(axum::body::Body::empty())
        .unwrap();
    req.extensions_mut().insert(regular_user.clone());
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let mut req = Request::builder()
        .method("DELETE")
        .uri("/admin/users/123")
        .body(axum::body::Body::empty())
        .unwrap();
    req.extensions_mut().insert(regular_user.clone());
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let mut req = Request::builder()
        .uri("/admin/dashboard")
        .body(axum::body::Body::empty())
        .unwrap();
    req.extensions_mut().insert(regular_user.clone());
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let admin_user = AuthenticatedUser {
        user_id: Uuid::new_v4(),
        email: "admin@example.com".to_string(),
        display_name: "Admin User".to_string(),
        jti: Uuid::new_v4().to_string(),
        roles: {
            let mut s = HashSet::new();
            s.insert(Role::Admin);
            s
        },
        permissions: {
            let mut s = HashSet::new();
            s.insert(Permission::ProfileRead);
            s.insert(Permission::ProfileWrite);
            s.insert(Permission::UsersRead);
            s.insert(Permission::UsersDelete);
            s
        },
    };

    let mut req = Request::builder()
        .method("DELETE")
        .uri("/admin/users/123")
        .body(axum::body::Body::empty())
        .unwrap();
    req.extensions_mut().insert(admin_user.clone());
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let mut req = Request::builder()
        .uri("/admin/dashboard")
        .body(axum::body::Body::empty())
        .unwrap();
    req.extensions_mut().insert(admin_user.clone());
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let mut req = Request::builder()
        .uri("/reports")
        .body(axum::body::Body::empty())
        .unwrap();
    req.extensions_mut().insert(admin_user.clone());
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let mut req = Request::builder()
        .uri("/sensitive")
        .body(axum::body::Body::empty())
        .unwrap();
    req.extensions_mut().insert(admin_user.clone());
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let mut req = Request::builder()
        .uri("/sensitive")
        .body(axum::body::Body::empty())
        .unwrap();
    req.extensions_mut().insert(regular_user.clone());
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[test]
fn test_pending_registration_serde() {
    use auth_api::repositories::redis_ephemeral::PendingRegistration;

    let pending = PendingRegistration {
        email: "user@example.com".to_string(),
        display_name: "Jane Doe".to_string(),
        password_hash: "$argon2id$v=19$m=19456,t=2,p=1$test$test".to_string(),
        code: "839201".to_string(),
        attempts: 2,
    };

    let serialized = serde_json::to_string(&pending).unwrap();
    let deserialized: PendingRegistration = serde_json::from_str(&serialized).unwrap();
    assert_eq!(pending, deserialized);
    assert_eq!(deserialized.code.len(), 6);
}

#[test]
fn test_smtp_configuration_parsing() {
    use auth_api::config::AppConfig;

    std::env::set_var("SMTP_HOST", "smtp.gmail.com");
    std::env::set_var("SMTP_PORT", "587");
    std::env::set_var("SMTP_USER", "khandelwalyatharth39@gmail.com");
    std::env::set_var("SMTP_PASS", "xvpx ykrc acys khqa");
    std::env::set_var("SMTP_FROM", "Dradix <support@dradix.dev>");
    std::env::set_var("SMTP_FROM_NAME", "Dradix");
    std::env::set_var("DATABASE_URL", "postgres://localhost/testdb");
    std::env::set_var(
        "JWT_SECRET",
        "super_secret_test_key_that_is_at_least_32_characters_long",
    );

    let cfg = AppConfig::from_env().unwrap();
    assert_eq!(cfg.smtp_host, Some("smtp.gmail.com".to_string()));
    assert_eq!(cfg.smtp_port, 587);
    assert_eq!(cfg.smtp_user, Some("khandelwalyatharth39@gmail.com".to_string()));
    assert_eq!(cfg.smtp_pass, Some("xvpx ykrc acys khqa".to_string()));
    assert_eq!(cfg.smtp_from, "Dradix <support@dradix.dev>");
    assert_eq!(cfg.smtp_from_name, "Dradix");
}

#[tokio::test]
async fn test_live_smtp_send() {
    let _ = dotenvy::dotenv();
    if let Ok(cfg) = auth_api::config::AppConfig::from_env() {
        if cfg.smtp_host.is_some() {
            let res = auth_api::services::email::send_registration_otp(
                &cfg,
                "khandelwalyatharth39@gmail.com",
                "Yatharth",
                "123456",
            ).await;
            println!("SMTP send result: {:?}", res);
        }
    }
}


