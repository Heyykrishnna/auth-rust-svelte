use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};

use auth_api::config::AppConfig;
use auth_api::middleware::{csrf_protection_middleware, security_headers_middleware};
use auth_api::models::{
    AppSessionId, AppUserId, EnvironmentId, EnvironmentKind, OrganizationId, UserId,
};
use auth_api::services::cookies::{build_csrf_cookie, build_refresh_cookie, build_session_cookie};
use auth_api::services::domains::{validate_origin, validate_redirect_uri};
use auth_api::services::oidc_provider::{
    build_discovery_document, build_sso_session_cookie, verify_pkce_s256, PROD_SSO_COOKIE_NAME,
};
use auth_api::services::password_hasher::{
    argon2_max_concurrency, hash_password_async, verify_dummy_password_async, verify_password_async,
};
use auth_api::services::signing_keys::{
    decrypt_envelope, encrypt_envelope, generate_es256_keypair, SIGNING_KEY_AAD_PURPOSE,
};
use auth_api::services::tokens::{
    issue_es256_access_token_with_key, issue_es256_id_token_with_key,
    verify_es256_access_token_with_jwk, verify_es256_id_token_with_jwk, IdTokenClaims,
    IssueAccessTokenInput,
};
use auth_api::services::webhooks::{
    build_signature_header, build_ssrf_safe_http_client, is_blocked_ip, validate_webhook_url,
    verify_signature_header, DEFAULT_SIGNATURE_TOLERANCE_SECS,
};
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use axum_extra::extract::cookie::SameSite;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

fn test_config(cookie_secure: bool) -> AppConfig {
    AppConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: "postgres://localhost:5432/unused".to_string(),
        db_max_connections: 5,
        redis_url: "redis://127.0.0.1:6379".to_string(),
        jwt_secret: "asvs_test_jwt_secret_material_32_bytes_minimum_01".to_string(),
        issuer_url: "https://auth.dradix.test".to_string(),
        signing_key_kek: "asvs_test_kek_secret_material_32_bytes_minimum_01".to_string(),
        signing_key_kek_version: "v1".to_string(),
        jwt_access_expiry_secs: 900,
        jwt_refresh_expiry_secs: 604_800,
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
        cors_origins: vec!["https://app.dradix.test".to_string()],
        cookie_secure,
        cookie_domain: None,
        otel_exporter_otlp_endpoint: "http://127.0.0.1:4317".to_string(),
        otel_service_name: "auth-api-asvs-test".to_string(),
        otel_service_version: "0.1.0".to_string(),
        smtp_host: None,
        smtp_port: 587,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: "Dradix <noreply@dradix.test>".to_string(),
        smtp_from_name: "Dradix".to_string(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_bounded_argon2id_pool_prevents_async_reactor_starvation() {
    let bound = argon2_max_concurrency();
    assert!((1..=32).contains(&bound));

    let password = "Phase10StrongPassword!2026".to_string();
    let hash = hash_password_async(password.clone()).await.unwrap();
    assert!(hash.starts_with("$argon2id$"));
    assert!(verify_password_async(password.clone(), hash.clone())
        .await
        .unwrap());
    assert!(
        !verify_password_async("WrongPassword!2026".to_string(), hash.clone())
            .await
            .unwrap()
    );

    verify_dummy_password_async("nonexistent-user-attempt".to_string()).await;

    let mut hash_handles = Vec::new();
    for i in 0..8 {
        let p = if i % 2 == 0 {
            password.clone()
        } else {
            format!("wrong-{i}")
        };
        let h = hash.clone();
        hash_handles.push(tokio::spawn(async move {
            verify_password_async(p, h).await.unwrap()
        }));
    }

    let probe_handle = tokio::spawn(async {
        let mut max_tick_ms = 0u128;
        for _ in 0..6 {
            let started = Instant::now();
            tokio::time::sleep(Duration::from_millis(5)).await;
            let elapsed_ms = started.elapsed().as_millis();
            if elapsed_ms > max_tick_ms {
                max_tick_ms = elapsed_ms;
            }
        }
        max_tick_ms
    });

    let max_tick_ms = probe_handle.await.unwrap();
    assert!(
        max_tick_ms < 500,
        "Async Tokio reactor stalled for {max_tick_ms}ms during concurrent Argon2id load"
    );

    for (idx, handle) in hash_handles.into_iter().enumerate() {
        let ok = handle.await.unwrap();
        assert_eq!(ok, idx % 2 == 0);
    }
}

#[test]
fn test_jwt_and_oidc_attack_matrix() {
    let key_a = generate_es256_keypair().unwrap();
    let key_b = generate_es256_keypair().unwrap();
    let issuer = "https://auth.dradix.test";
    let env_ferrum = EnvironmentId::new();
    let env_elysiar = EnvironmentId::new();
    let user_id = UserId::new();
    let app_user_id = AppUserId::new();
    let session_id = AppSessionId::new();

    let (valid_token, valid_claims) = issue_es256_access_token_with_key(
        IssueAccessTokenInput {
            issuer_url: issuer,
            user_id,
            environment_id: env_ferrum,
            authorized_party: "pk_live_ferrum_client",
            app_user_id,
            session_id,
            active_organization_id: Some(OrganizationId::new()),
            roles: vec!["member".to_string()],
            permissions: vec!["profile.read".to_string()],
            amr: vec!["pwd".to_string()],
            acr: "aal1".to_string(),
            version: 1,
            ttl_secs: 300,
        },
        &key_a.kid,
        &key_a.pkcs8_der,
    )
    .unwrap();

    assert!(verify_es256_access_token_with_jwk(
        &valid_token,
        &key_a.public_jwk,
        issuer,
        &env_ferrum.to_string(),
    )
    .is_ok());

    assert!(verify_es256_access_token_with_jwk(
        &valid_token,
        &key_a.public_jwk,
        issuer,
        &env_elysiar.to_string(),
    )
    .is_err());

    assert!(verify_es256_access_token_with_jwk(
        &valid_token,
        &key_a.public_jwk,
        "https://evil-issuer.test",
        &env_ferrum.to_string(),
    )
    .is_err());

    assert!(verify_es256_access_token_with_jwk(
        &valid_token,
        &key_b.public_jwk,
        issuer,
        &env_ferrum.to_string(),
    )
    .is_err());

    let none_header = URL_SAFE_NO_PAD
        .encode(serde_json::json!({"alg": "none", "typ": "at+jwt", "kid": key_a.kid}).to_string());
    let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&valid_claims).unwrap());
    let alg_none_token = format!("{none_header}.{payload_b64}.");
    assert!(verify_es256_access_token_with_jwk(
        &alg_none_token,
        &key_a.public_jwk,
        issuer,
        &env_ferrum.to_string(),
    )
    .is_err());

    let mut hs_header = Header::new(Algorithm::HS256);
    hs_header.kid = Some(key_a.kid.clone());
    hs_header.typ = Some("at+jwt".to_string());
    let jwk_public_bytes = serde_json::to_vec(&key_a.public_jwk).unwrap();
    let hs_confused = encode(
        &hs_header,
        &valid_claims,
        &EncodingKey::from_secret(&jwk_public_bytes),
    )
    .unwrap();
    assert!(verify_es256_access_token_with_jwk(
        &hs_confused,
        &key_a.public_jwk,
        issuer,
        &env_ferrum.to_string(),
    )
    .is_err());

    let (expired_token, _) = issue_es256_access_token_with_key(
        IssueAccessTokenInput {
            issuer_url: issuer,
            user_id,
            environment_id: env_ferrum,
            authorized_party: "pk_live_ferrum_client",
            app_user_id,
            session_id,
            active_organization_id: None,
            roles: vec![],
            permissions: vec![],
            amr: vec!["pwd".to_string()],
            acr: "aal1".to_string(),
            version: 1,
            ttl_secs: -120,
        },
        &key_a.kid,
        &key_a.pkcs8_der,
    )
    .unwrap();
    assert!(verify_es256_access_token_with_jwk(
        &expired_token,
        &key_a.public_jwk,
        issuer,
        &env_ferrum.to_string(),
    )
    .is_err());

    let now = Utc::now().timestamp();
    let id_claims = IdTokenClaims {
        iss: issuer.to_string(),
        sub: user_id.to_string(),
        aud: "pk_live_ferrum_client".to_string(),
        nonce: Some("nonce-123".to_string()),
        auth_time: now,
        amr: vec!["pwd".to_string()],
        acr: "aal1".to_string(),
        email: "alice@dradix.test".to_string(),
        email_verified: true,
        name: "Alice".to_string(),
        picture: None,
        app_user_id: app_user_id.to_string(),
        ver: 1,
        iat: now,
        exp: now + 300,
    };
    let id_token = issue_es256_id_token_with_key(&id_claims, &key_a.kid, &key_a.pkcs8_der).unwrap();
    assert!(verify_es256_access_token_with_jwk(
        &id_token,
        &key_a.public_jwk,
        issuer,
        "pk_live_ferrum_client",
    )
    .is_err());
    assert!(verify_es256_id_token_with_jwk(
        &valid_token,
        &key_a.public_jwk,
        issuer,
        &env_ferrum.to_string(),
    )
    .is_err());
    assert!(verify_es256_id_token_with_jwk(
        &id_token,
        &key_a.public_jwk,
        issuer,
        "pk_live_elysiar_client",
    )
    .is_err());

    let discovery = build_discovery_document(&test_config(true));
    assert_eq!(discovery.code_challenge_methods_supported, vec!["S256"]);
    assert_eq!(
        discovery.id_token_signing_alg_values_supported,
        vec!["ES256"]
    );

    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    assert!(verify_pkce_s256(verifier, &challenge));
    assert!(!verify_pkce_s256(verifier, verifier));
    assert!(!verify_pkce_s256("too-short-verifier", &challenge));
    assert!(!verify_pkce_s256(
        "invalid space characters in pkce verifier string 1234567890",
        &challenge
    ));
}

#[test]
fn test_redirect_uri_and_origin_parser_differential_attacks() {
    let malicious_redirect_uris = [
        "https://good.com@evil.com/callback",
        "https://good.com/auth/../../evil",
        "https://good.com/auth/..",
        "https://good.com/auth/./callback",
        "https://good.com\\evil.com/callback",
        "https://good.com/callback#fragment",
        "https://good.com/callback*",
        "https://good.com/callback with space",
        "//evil.com/callback",
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "http://app.dradix.test/callback",
        "https://localhost:3000/callback",
        "https://127.0.0.1:3000/callback",
        "https://g\u{043e}\u{043e}d.com/callback",
        "https://good.com:0/callback",
    ];

    for bad_uri in malicious_redirect_uris {
        assert!(
            validate_redirect_uri(EnvironmentKind::Production, bad_uri).is_err(),
            "Expected production redirect URI rejection for '{bad_uri}'"
        );
    }

    assert_eq!(
        validate_redirect_uri(
            EnvironmentKind::Production,
            "https://App.Dradix.Test/auth/callback"
        )
        .unwrap(),
        "https://app.dradix.test/auth/callback"
    );
    assert!(validate_origin(EnvironmentKind::Production, "http://app.dradix.test").is_err());
    assert!(validate_origin(EnvironmentKind::Development, "http://evil.example.com").is_err());
    assert!(validate_origin(EnvironmentKind::Development, "http://localhost:5173").is_ok());
}

#[test]
fn test_webhook_ssrf_and_hmac_replay_protection() {
    let always_blocked_ips: Vec<IpAddr> = vec![
        IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(172, 16, 0, 5)),
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
        IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        IpAddr::V4(Ipv4Addr::new(198, 18, 0, 1)),
        IpAddr::V6("::ffff:169.254.169.254".parse::<Ipv6Addr>().unwrap()),
        IpAddr::V6("::ffff:10.0.0.1".parse::<Ipv6Addr>().unwrap()),
        IpAddr::V6("::ffff:192.168.1.1".parse::<Ipv6Addr>().unwrap()),
        IpAddr::V6("fc00::1".parse::<Ipv6Addr>().unwrap()),
        IpAddr::V6("fe80::1".parse::<Ipv6Addr>().unwrap()),
    ];

    for ip in always_blocked_ips {
        assert!(is_blocked_ip(ip, false), "IP {ip} must be blocked in prod");
        assert!(is_blocked_ip(ip, true), "IP {ip} must be blocked in dev");
    }

    let prod_only_blocked_ips: Vec<IpAddr> = vec![
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        IpAddr::V6("::ffff:127.0.0.1".parse::<Ipv6Addr>().unwrap()),
    ];

    for ip in prod_only_blocked_ips {
        assert!(
            is_blocked_ip(ip, false),
            "Loopback {ip} must be blocked in prod"
        );
        assert!(!is_blocked_ip(ip, true), "Loopback {ip} allowed in dev");
    }

    assert!(!is_blocked_ip(
        IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
        false
    ));

    let blocked_prod_urls = [
        "https://169.254.169.254/latest/meta-data/",
        "https://metadata.google.internal/computeMetadata/v1/",
        "https://127.0.0.1/hook",
        "https://[::1]/hook",
        "https://[::ffff:127.0.0.1]/hook",
        "https://[::ffff:169.254.169.254]/hook",
        "https://10.0.0.5/hook",
        "https://service.local/hook",
        "https://service.internal/hook",
        "https://user:pass@hooks.dradix.test/events",
        "http://hooks.dradix.test/events",
        "file:///etc/passwd",
        "gopher://127.0.0.1:6379/_INFO",
    ];
    for url in blocked_prod_urls {
        assert!(
            validate_webhook_url(url, EnvironmentKind::Production).is_err(),
            "Expected SSRF block for production webhook URL '{url}'"
        );
    }

    assert!(validate_webhook_url(
        "https://hooks.dradix.test/events",
        EnvironmentKind::Production
    )
    .is_ok());
    assert!(validate_webhook_url(
        "http://localhost:4000/webhooks",
        EnvironmentKind::Development
    )
    .is_ok());
    assert!(build_ssrf_safe_http_client(false).is_ok());

    let signing_secret = Uuid::new_v4().to_string();
    let body = br#"{"event_id":"evt_1","type":"user.created"}"#;
    let now = 1_760_000_000i64;
    let header = build_signature_header(&[signing_secret.as_str()], now, body);

    assert!(verify_signature_header(
        &header,
        &[signing_secret.as_str()],
        now,
        body,
        now + 120,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_ok());

    assert!(verify_signature_header(
        &header,
        &[signing_secret.as_str()],
        now,
        body,
        now + DEFAULT_SIGNATURE_TOLERANCE_SECS + 1,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_err());

    assert!(verify_signature_header(
        &header,
        &[signing_secret.as_str()],
        now,
        body,
        now - DEFAULT_SIGNATURE_TOLERANCE_SECS - 1,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_err());

    assert!(verify_signature_header(
        &header,
        &[signing_secret.as_str()],
        now,
        br#"{"event_id":"evt_1","type":"user.deleted"}"#,
        now,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_err());
}

#[tokio::test]
async fn test_security_headers_cookie_flags_and_csrf_enforcement() {
    let app = Router::new()
        .route("/ping", get(|| async { StatusCode::OK }))
        .route("/protected/mutate", post(|| async { StatusCode::OK }))
        .layer(axum::middleware::from_fn(csrf_protection_middleware))
        .layer(axum::middleware::from_fn(security_headers_middleware));

    let resp = app
        .clone()
        .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let headers = resp.headers();
    assert_eq!(
        headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
        "nosniff"
    );
    assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
    assert_eq!(
        headers.get(header::REFERRER_POLICY).unwrap(),
        "strict-origin-when-cross-origin"
    );
    assert!(headers
        .get("content-security-policy")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("frame-ancestors 'none'"));
    assert!(headers
        .get("strict-transport-security")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("max-age=63072000"));
    assert!(headers.get("server").is_none());

    let csrf_missing = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/protected/mutate")
                .header(
                    header::COOKIE,
                    "session_token=abc; csrf_token=expected_token",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(csrf_missing.status(), StatusCode::FORBIDDEN);

    let csrf_mismatch = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/protected/mutate")
                .header(
                    header::COOKIE,
                    "session_token=abc; csrf_token=expected_token",
                )
                .header("x-csrf-token", "wrong_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(csrf_mismatch.status(), StatusCode::FORBIDDEN);

    let csrf_valid = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/protected/mutate")
                .header(
                    header::COOKIE,
                    "session_token=abc; csrf_token=expected_token",
                )
                .header("x-csrf-token", "expected_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(csrf_valid.status(), StatusCode::OK);

    let prod_cfg = test_config(true);
    let session_cookie = build_session_cookie(&prod_cfg, "tok".to_string());
    assert_eq!(session_cookie.http_only(), Some(true));
    assert_eq!(session_cookie.secure(), Some(true));
    assert_eq!(session_cookie.same_site(), Some(SameSite::Lax));

    let refresh_cookie = build_refresh_cookie(&prod_cfg, "rtok".to_string());
    assert_eq!(refresh_cookie.http_only(), Some(true));
    assert_eq!(refresh_cookie.secure(), Some(true));
    assert_eq!(refresh_cookie.same_site(), Some(SameSite::Lax));

    let csrf_cookie = build_csrf_cookie(&prod_cfg, "ctok".to_string());
    assert_eq!(csrf_cookie.http_only(), Some(false));
    assert_eq!(csrf_cookie.secure(), Some(true));
    assert_eq!(csrf_cookie.same_site(), Some(SameSite::Lax));

    let sso_cookie = build_sso_session_cookie(&prod_cfg, "ssotok".to_string());
    assert_eq!(sso_cookie.name(), PROD_SSO_COOKIE_NAME);
    assert_eq!(sso_cookie.http_only(), Some(true));
    assert_eq!(sso_cookie.secure(), Some(true));
    assert_eq!(sso_cookie.same_site(), Some(SameSite::Lax));

    let ct = encrypt_envelope(
        b"signing-key-material",
        &prod_cfg.signing_key_kek,
        &prod_cfg.signing_key_kek_version,
        SIGNING_KEY_AAD_PURPOSE,
    )
    .unwrap();
    assert!(decrypt_envelope(
        &ct,
        &prod_cfg.signing_key_kek,
        &prod_cfg.signing_key_kek_version,
        "wrong_purpose",
    )
    .is_err());
}

#[cfg(test)]
mod live_phase10_key_compromise_drill {
    use super::test_config;
    use auth_api::models::{AppSessionId, AppUserId, EnvironmentId, UserId};
    use auth_api::services::signing_keys::{
        build_jwks_with_etag, ensure_active_signing_key, revoke_signing_key, rotate_signing_key,
    };
    use auth_api::services::tokens::{
        issue_tenant_access_token, verify_tenant_access_token, IssueAccessTokenInput,
    };
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn test_live_emergency_signing_key_revocation_and_rotation_drill() {
        let db_url = match std::env::var("TEST_DATABASE_URL") {
            Ok(v) if !v.trim().is_empty() => v,
            _ => return,
        };

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let mut cfg = test_config(false);
        cfg.database_url = db_url;

        let compromised_key = ensure_active_signing_key(&pool, &cfg).await.unwrap();
        let env_id = EnvironmentId::new();
        let user_id = UserId::new();
        let app_user_id = AppUserId::new();
        let session_id = AppSessionId::new();

        let (pre_revoke_token, _) = issue_tenant_access_token(
            &pool,
            &cfg,
            IssueAccessTokenInput {
                issuer_url: &cfg.issuer_url,
                user_id,
                environment_id: env_id,
                authorized_party: "pk_test_drill",
                app_user_id,
                session_id,
                active_organization_id: None,
                roles: vec!["user".to_string()],
                permissions: vec!["profile.read".to_string()],
                amr: vec!["pwd".to_string()],
                acr: "aal1".to_string(),
                version: 1,
                ttl_secs: 300,
            },
        )
        .await
        .unwrap();

        assert!(
            verify_tenant_access_token(&pool, &cfg, &pre_revoke_token, env_id)
                .await
                .is_ok()
        );

        let (jwks_before, _) = build_jwks_with_etag(&pool, &cfg).await.unwrap();
        assert!(jwks_before
            .keys
            .iter()
            .any(|k| k["kid"].as_str() == Some(compromised_key.kid.as_str())));

        let rotated = rotate_signing_key(&pool, &cfg).await.unwrap();
        assert_ne!(rotated.kid, compromised_key.kid);

        let revoked = revoke_signing_key(&pool, &compromised_key.kid)
            .await
            .unwrap();
        assert!(revoked);

        let (jwks_after, _) = build_jwks_with_etag(&pool, &cfg).await.unwrap();
        assert!(!jwks_after
            .keys
            .iter()
            .any(|k| k["kid"].as_str() == Some(compromised_key.kid.as_str())));
        assert!(jwks_after
            .keys
            .iter()
            .any(|k| k["kid"].as_str() == Some(rotated.kid.as_str())));

        assert!(
            verify_tenant_access_token(&pool, &cfg, &pre_revoke_token, env_id)
                .await
                .is_err()
        );

        let (post_rotate_token, _) = issue_tenant_access_token(
            &pool,
            &cfg,
            IssueAccessTokenInput {
                issuer_url: &cfg.issuer_url,
                user_id,
                environment_id: env_id,
                authorized_party: "pk_test_drill",
                app_user_id,
                session_id,
                active_organization_id: None,
                roles: vec!["user".to_string()],
                permissions: vec!["profile.read".to_string()],
                amr: vec!["pwd".to_string()],
                acr: "aal1".to_string(),
                version: 1,
                ttl_secs: 300,
            },
        )
        .await
        .unwrap();

        assert!(
            verify_tenant_access_token(&pool, &cfg, &post_rotate_token, env_id)
                .await
                .is_ok()
        );
    }
}
