use auth_api::models::{AppSessionId, AppUserId, EnvironmentId, OrganizationId, UserId};
use auth_api::services::signing_keys::{
    decrypt_envelope, encrypt_envelope, generate_es256_keypair, EcPublicJwk,
    SIGNING_KEY_AAD_PURPOSE,
};
use auth_api::services::social_oauth::{generate_pkce_s256_pair, validate_google_id_token_nonce};
use auth_api::services::tokens::{
    issue_es256_access_token_with_key, issue_es256_id_token_with_key,
    verify_es256_access_token_with_jwk, verify_es256_id_token_with_jwk, IdTokenClaims,
    IssueAccessTokenInput,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use sha2::{Digest, Sha256};

#[test]
fn envelope_encryption_roundtrip_and_aad_tamper_rejection() {
    let kek = "phase03_master_kek_secret_material_32bytes_min_01";
    let wrong_kek = "phase03_wrong_kek_secret_material_32bytes_min_02";
    let plaintext = b"pkcs8-es256-private-key-material-bytes";

    let ciphertext =
        encrypt_envelope(plaintext, kek, "v1", SIGNING_KEY_AAD_PURPOSE).expect("encrypt");
    assert_ne!(ciphertext.as_slice(), plaintext);

    let decrypted = decrypt_envelope(&ciphertext, kek, "v1", SIGNING_KEY_AAD_PURPOSE)
        .expect("decrypt with matching kek and aad");
    assert_eq!(decrypted.as_slice(), plaintext);

    assert!(decrypt_envelope(&ciphertext, wrong_kek, "v1", SIGNING_KEY_AAD_PURPOSE).is_err());
    assert!(decrypt_envelope(&ciphertext, kek, "v2", SIGNING_KEY_AAD_PURPOSE).is_err());
    assert!(decrypt_envelope(&ciphertext, kek, "v1", "different_purpose").is_err());

    let mut tampered = ciphertext.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0xFF;
    assert!(decrypt_envelope(&tampered, kek, "v1", SIGNING_KEY_AAD_PURPOSE).is_err());
}

#[test]
fn es256_access_and_id_token_jwk_verification_and_confusion_rejection() {
    let generated = generate_es256_keypair().expect("generate es256 keypair");
    assert_eq!(generated.public_jwk.kty, "EC");
    assert_eq!(generated.public_jwk.crv, "P-256");
    assert_eq!(generated.public_jwk.alg, "ES256");
    assert_eq!(generated.public_jwk.key_use, "sig");
    assert_eq!(generated.public_jwk.kid, generated.kid);

    let issuer = "https://auth.platform.example";
    let env_id = EnvironmentId::new();
    let other_env_id = EnvironmentId::new();
    let user_id = UserId::new();
    let app_user_id = AppUserId::new();
    let session_id = AppSessionId::new();
    let org_id = OrganizationId::new();

    let (access_token, issued_claims) = issue_es256_access_token_with_key(
        IssueAccessTokenInput {
            issuer_url: issuer,
            user_id,
            environment_id: env_id,
            authorized_party: "pk_test_ferrum_1234",
            app_user_id,
            session_id,
            active_organization_id: Some(org_id),
            roles: vec!["user".to_string(), "editor".to_string()],
            permissions: vec!["profile.read".to_string(), "profile.write".to_string()],
            amr: vec!["pwd".to_string()],
            acr: "aal1".to_string(),
            version: 3,
            ttl_secs: 300,
        },
        &generated.kid,
        &generated.pkcs8_der,
    )
    .expect("issue access token");

    let verified = verify_es256_access_token_with_jwk(
        &access_token,
        &generated.public_jwk,
        issuer,
        &env_id.to_string(),
    )
    .expect("verify access token with public jwk");

    assert_eq!(verified.sub, user_id.to_string());
    assert_eq!(verified.aud, env_id.to_string());
    assert_eq!(verified.azp, "pk_test_ferrum_1234");
    assert_eq!(verified.app_user_id, app_user_id.to_string());
    assert_eq!(verified.sid, session_id.to_string());
    assert_eq!(verified.org_id, Some(org_id.to_string()));
    assert_eq!(verified.ver, 3);
    assert_eq!(verified.jti, issued_claims.jti);

    assert!(verify_es256_access_token_with_jwk(
        &access_token,
        &generated.public_jwk,
        issuer,
        &other_env_id.to_string(),
    )
    .is_err());

    assert!(verify_es256_access_token_with_jwk(
        &access_token,
        &generated.public_jwk,
        "https://evil.issuer.example",
        &env_id.to_string(),
    )
    .is_err());

    let now = Utc::now().timestamp();
    let id_claims = IdTokenClaims {
        iss: issuer.to_string(),
        sub: user_id.to_string(),
        aud: env_id.to_string(),
        nonce: Some("nonce-abc-123".to_string()),
        auth_time: now,
        amr: vec!["pwd".to_string()],
        acr: "aal1".to_string(),
        email: "alice@example.com".to_string(),
        email_verified: true,
        name: "Alice".to_string(),
        picture: None,
        app_user_id: app_user_id.to_string(),
        ver: 3,
        iat: now,
        exp: now + 300,
    };

    let id_token = issue_es256_id_token_with_key(&id_claims, &generated.kid, &generated.pkcs8_der)
        .expect("issue id token");

    let verified_id = verify_es256_id_token_with_jwk(
        &id_token,
        &generated.public_jwk,
        issuer,
        &env_id.to_string(),
    )
    .expect("verify id token");
    assert_eq!(verified_id.nonce.as_deref(), Some("nonce-abc-123"));

    assert!(verify_es256_access_token_with_jwk(
        &id_token,
        &generated.public_jwk,
        issuer,
        &env_id.to_string(),
    )
    .is_err());

    assert!(verify_es256_id_token_with_jwk(
        &access_token,
        &generated.public_jwk,
        issuer,
        &env_id.to_string(),
    )
    .is_err());

    let mut hs_header = Header::new(Algorithm::HS256);
    hs_header.typ = Some("at+jwt".to_string());
    hs_header.kid = Some(generated.kid.clone());
    let forged_hs256 = encode(
        &hs_header,
        &issued_claims,
        &EncodingKey::from_secret(b"attacker_symmetric_key"),
    )
    .expect("encode forged hs256");

    assert!(verify_es256_access_token_with_jwk(
        &forged_hs256,
        &generated.public_jwk,
        issuer,
        &env_id.to_string(),
    )
    .is_err());

    let other_key = generate_es256_keypair().expect("generate second keypair");
    let wrong_jwk: EcPublicJwk = other_key.public_jwk;
    assert!(verify_es256_access_token_with_jwk(
        &access_token,
        &wrong_jwk,
        issuer,
        &env_id.to_string(),
    )
    .is_err());
}

#[test]
fn social_oauth_pkce_s256_and_google_nonce_validation() {
    let (verifier, challenge) = generate_pkce_s256_pair();
    assert!(verifier.len() >= 43);

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let recomputed = URL_SAFE_NO_PAD.encode(hasher.finalize());
    assert_eq!(challenge, recomputed);

    let header_b64 = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
    let valid_payload_b64 = URL_SAFE_NO_PAD.encode(r#"{"sub":"123","nonce":"expected_nonce_42"}"#);
    let missing_nonce_b64 = URL_SAFE_NO_PAD.encode(r#"{"sub":"123"}"#);
    let fake_sig = URL_SAFE_NO_PAD.encode(b"sig");

    let valid_jwt = format!("{header_b64}.{valid_payload_b64}.{fake_sig}");
    let missing_jwt = format!("{header_b64}.{missing_nonce_b64}.{fake_sig}");

    assert!(validate_google_id_token_nonce(&valid_jwt, "expected_nonce_42").is_ok());
    assert!(validate_google_id_token_nonce(&valid_jwt, "wrong_nonce").is_err());
    assert!(validate_google_id_token_nonce(&missing_jwt, "expected_nonce_42").is_err());
    assert!(validate_google_id_token_nonce("not-a-jwt", "expected_nonce_42").is_err());
}

#[cfg(test)]
mod live_postgres_phase03_tests {
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::Arc;

    use auth_api::config::AppConfig;
    use auth_api::models::{EnvironmentId, UserId};
    use auth_api::repositories::global_identity;
    use auth_api::routes::build_router;
    use auth_api::services::client_auth;
    use auth_api::services::signing_keys::{self, EcPublicJwk};
    use auth_api::services::social_oauth::{self, VerifiedSocialIdentity};
    use auth_api::services::tokens::{
        issue_tenant_access_token, verify_es256_access_token_with_jwk, verify_tenant_access_token,
        IssueAccessTokenInput,
    };
    use auth_api::AppState;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::header::{AUTHORIZATION, CACHE_CONTROL, COOKIE, ETAG, IF_NONE_MATCH};
    use axum::http::{Method, Request, StatusCode};
    use chrono::{Duration, Utc};
    use deadpool_redis::{Config as RedisConfig, Runtime as RedisRuntime};
    use serde_json::{json, Value};
    use sqlx::postgres::PgPoolOptions;
    use sqlx::PgPool;
    use tower::ServiceExt;
    use uuid::Uuid;

    static IP_COUNTER: AtomicU16 = AtomicU16::new(1);

    async fn build_live_test_env() -> (axum::Router, AppState) {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for live Phase 03 tests");
        let db = PgPoolOptions::new()
            .max_connections(8)
            .connect(&db_url)
            .await
            .expect("connect to PostgreSQL");

        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .expect("run migrations");

        sqlx::query("DELETE FROM signing_keys")
            .execute(&db)
            .await
            .expect("reset signing_keys for test KEK");

        let redis_cfg = RedisConfig::from_url("redis://127.0.0.1:6399");
        let redis = redis_cfg
            .create_pool(Some(RedisRuntime::Tokio1))
            .expect("create redis pool");

        let config = Arc::new(AppConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            database_url: db_url,
            db_max_connections: 8,
            redis_url: "redis://127.0.0.1:6399".to_string(),
            jwt_secret: "phase03_integration_test_jwt_secret_at_least_32_chars".to_string(),
            issuer_url: "https://auth.phase03.test".to_string(),
            signing_key_kek: "phase03_live_test_kek_material_at_least_32_characters_01".to_string(),
            signing_key_kek_version: "v1".to_string(),
            jwt_access_expiry_secs: 300,
            jwt_refresh_expiry_secs: 86400,
            login_max_attempts: 5,
            login_lockout_duration_secs: 900,
            verification_code_expiry_secs: 900,
            password_reset_expiry_secs: 900,
            google_client_id: Some("google-test-client-id".to_string()),
            google_client_secret: Some("google-test-client-secret".to_string()),
            google_redirect_uri: Some("http://localhost:3000/auth/google/callback".to_string()),
            github_client_id: Some("github-test-client-id".to_string()),
            github_client_secret: Some("github-test-client-secret".to_string()),
            github_redirect_uri: Some("http://localhost:3000/auth/github/callback".to_string()),
            cors_origins: vec!["http://localhost:3000".to_string()],
            cookie_secure: false,
            cookie_domain: None,
            otel_exporter_otlp_endpoint: "http://localhost:4317".to_string(),
            otel_service_name: "auth-api-test".to_string(),
            otel_service_version: "0.1.0".to_string(),
            smtp_host: None,
            smtp_port: 587,
            smtp_user: None,
            smtp_pass: None,
            smtp_from: "Auth <no-reply@example.com>".to_string(),
            smtp_from_name: "Auth".to_string(),
        });

        let state = AppState { config, db, redis };
        let router = build_router(state.clone());
        (router, state)
    }

    async fn send_request(app: &axum::Router, mut req: Request<Body>) -> axum::response::Response {
        let octet = (IP_COUNTER.fetch_add(1, Ordering::Relaxed) % 250) as u8 + 1;
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 1, octet], 41000))));
        app.clone().oneshot(req).await.expect("send request")
    }

    async fn read_json(res: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&bytes).expect("parse json")
    }

    struct ProvisionedApp {
        env_id: EnvironmentId,
        publishable_key: String,
    }

    async fn provision_two_apps(app: &axum::Router) -> (ProvisionedApp, ProvisionedApp) {
        let dev_email = format!("dev-{}@phase03.test", Uuid::new_v4().simple());
        let sign_up = Request::builder()
            .method(Method::POST)
            .uri("/v1/control/auth/sign-up")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "email": dev_email,
                    "password": "StrongDevPassword123!",
                    "display_name": "Phase 03 Architect",
                    "workspace_name": "Phase 03 Workspace"
                })
                .to_string(),
            ))
            .unwrap();
        let res = send_request(app, sign_up).await;
        assert_eq!(res.status(), StatusCode::CREATED);

        let mut dev_cookie = String::new();
        let mut csrf_token = String::new();
        for raw in res
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
        {
            if let Some(first) = raw.split(';').next().map(str::trim) {
                if first.starts_with("dev_session_token=") {
                    dev_cookie = first.to_string();
                } else if let Some(val) = first.strip_prefix("csrf_token=") {
                    csrf_token = val.to_string();
                }
            }
        }
        assert!(!dev_cookie.is_empty());
        assert!(!csrf_token.is_empty());
        let cookie_header = format!("{dev_cookie}; csrf_token={csrf_token}");

        let ws_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/control/workspaces")
            .header(COOKIE, &cookie_header)
            .body(Body::empty())
            .unwrap();
        let ws_res = send_request(app, ws_req).await;
        assert_eq!(ws_res.status(), StatusCode::OK);
        let ws_list = read_json(ws_res).await;
        let workspace_id = ws_list[0]["id"].as_str().unwrap().to_string();

        let create_app = |name: &str, slug_prefix: &str| {
            let ws_id = workspace_id.clone();
            let cookie = cookie_header.clone();
            let csrf = csrf_token.clone();
            let app_clone = app.clone();
            let app_name = name.to_string();
            let slug = format!(
                "{}-{}",
                slug_prefix,
                &Uuid::new_v4().simple().to_string()[..8]
            );
            async move {
                let req = Request::builder()
                    .method(Method::POST)
                    .uri("/v1/control/applications")
                    .header(COOKIE, &cookie)
                    .header("x-csrf-token", &csrf)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "workspace_id": ws_id,
                            "name": app_name,
                            "slug": slug
                        })
                        .to_string(),
                    ))
                    .unwrap();
                let res = send_request(&app_clone, req).await;
                assert_eq!(res.status(), StatusCode::CREATED);
                let body = read_json(res).await;
                let dev_env = body["environments"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|e| e["kind"] == "development")
                    .expect("development environment");
                let env_id: EnvironmentId = dev_env["id"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .expect("parse env_id");
                let publishable_key = body["development_publishable_key"]
                    .as_str()
                    .unwrap()
                    .to_string();
                ProvisionedApp {
                    env_id,
                    publishable_key,
                }
            }
        };

        let ferrum = create_app("Ferrum", "ferrum").await;
        let elysiar = create_app("Elysiar", "elysiar").await;
        (ferrum, elysiar)
    }

    async fn register_and_verify_user(
        app: &axum::Router,
        state: &AppState,
        target: &ProvisionedApp,
        email: &str,
        password: &str,
        display_name: &str,
    ) -> Value {
        let (_, otp) = client_auth::initiate_sign_up(
            &state.db,
            &state.config,
            target.env_id,
            email,
            password,
            display_name,
        )
        .await
        .expect("initiate sign up");

        let verify_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/sign-up/verify")
            .header("x-publishable-key", &target.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "email": email,
                    "code": otp
                })
                .to_string(),
            ))
            .unwrap();
        let verify_res = send_request(app, verify_req).await;
        assert_eq!(verify_res.status(), StatusCode::CREATED);
        read_json(verify_res).await
    }

    async fn count_outbox_events(db: &PgPool, env_id: EnvironmentId, event_type: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM outbox_events WHERE environment_id = $1 AND event_type = $2",
        )
        .bind(env_id)
        .bind(event_type)
        .fetch_one(db)
        .await
        .expect("count outbox events")
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn jwks_endpoint_and_zero_downtime_key_rotation_and_revocation() {
        let (app, state) = build_live_test_env().await;

        let req1 = Request::builder()
            .method(Method::GET)
            .uri("/.well-known/jwks.json")
            .body(Body::empty())
            .unwrap();
        let res1 = send_request(&app, req1).await;
        assert_eq!(res1.status(), StatusCode::OK);
        assert_eq!(
            res1.headers()
                .get(CACHE_CONTROL)
                .and_then(|v| v.to_str().ok()),
            Some("public, max-age=300")
        );
        let etag1 = res1
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .expect("etag header")
            .to_string();
        let jwks1 = read_json(res1).await;
        let keys1 = jwks1["keys"].as_array().expect("jwks keys array");
        assert!(!keys1.is_empty());

        let req_304 = Request::builder()
            .method(Method::GET)
            .uri("/.well-known/jwks.json")
            .header(IF_NONE_MATCH, &etag1)
            .body(Body::empty())
            .unwrap();
        let res_304 = send_request(&app, req_304).await;
        assert_eq!(res_304.status(), StatusCode::NOT_MODIFIED);

        let active_key_1 = signing_keys::ensure_active_signing_key(&state.db, &state.config)
            .await
            .expect("active key 1");
        let env_id = EnvironmentId::new();
        let (token_k1, _) = issue_tenant_access_token(
            &state.db,
            &state.config,
            IssueAccessTokenInput {
                issuer_url: &state.config.issuer_url,
                user_id: UserId::new(),
                environment_id: env_id,
                authorized_party: "pk_test_rot",
                app_user_id: auth_api::models::AppUserId::new(),
                session_id: auth_api::models::AppSessionId::new(),
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
        .expect("issue token with key 1");

        let jwk_1: EcPublicJwk =
            serde_json::from_value(active_key_1.public_jwk.clone()).expect("parse jwk1");
        verify_es256_access_token_with_jwk(
            &token_k1,
            &jwk_1,
            &state.config.issuer_url,
            &env_id.to_string(),
        )
        .expect("token k1 verifies with public jwk from jwks");

        let active_key_2 = signing_keys::rotate_signing_key(&state.db, &state.config)
            .await
            .expect("rotate signing key");
        assert_ne!(active_key_1.kid, active_key_2.kid);

        let (token_k2, _) = issue_tenant_access_token(
            &state.db,
            &state.config,
            IssueAccessTokenInput {
                issuer_url: &state.config.issuer_url,
                user_id: UserId::new(),
                environment_id: env_id,
                authorized_party: "pk_test_rot",
                app_user_id: auth_api::models::AppUserId::new(),
                session_id: auth_api::models::AppSessionId::new(),
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
        .expect("issue token with key 2");

        let req2 = Request::builder()
            .method(Method::GET)
            .uri("/.well-known/jwks.json")
            .body(Body::empty())
            .unwrap();
        let res2 = send_request(&app, req2).await;
        assert_eq!(res2.status(), StatusCode::OK);
        let etag2 = res2
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .unwrap()
            .to_string();
        assert_ne!(etag1, etag2);
        let jwks2 = read_json(res2).await;
        let kids2: Vec<&str> = jwks2["keys"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|k| k["kid"].as_str())
            .collect();
        assert!(kids2.contains(&active_key_1.kid.as_str()));
        assert!(kids2.contains(&active_key_2.kid.as_str()));

        verify_tenant_access_token(&state.db, &state.config, &token_k1, env_id)
            .await
            .expect("retiring key token still validates");
        verify_tenant_access_token(&state.db, &state.config, &token_k2, env_id)
            .await
            .expect("active key token validates");

        signing_keys::revoke_signing_key(&state.db, &active_key_1.kid)
            .await
            .expect("revoke key 1");

        let req3 = Request::builder()
            .method(Method::GET)
            .uri("/.well-known/jwks.json")
            .body(Body::empty())
            .unwrap();
        let res3 = send_request(&app, req3).await;
        let jwks3 = read_json(res3).await;
        let kids3: Vec<&str> = jwks3["keys"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|k| k["kid"].as_str())
            .collect();
        assert!(!kids3.contains(&active_key_1.kid.as_str()));
        assert!(kids3.contains(&active_key_2.kid.as_str()));

        assert!(
            verify_tenant_access_token(&state.db, &state.config, &token_k1, env_id)
                .await
                .is_err()
        );
        verify_tenant_access_token(&state.db, &state.config, &token_k2, env_id)
            .await
            .expect("active key 2 remains valid after key 1 revocation");
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn cross_app_token_session_and_suspension_isolation_ferrum_vs_elysiar() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, elysiar) = provision_two_apps(&app).await;

        let alice_email = format!("alice-{}@example.com", Uuid::new_v4().simple());
        let alice_pass = "AliceSecurePass123!";

        let ferrum_auth =
            register_and_verify_user(&app, &state, &ferrum, &alice_email, alice_pass, "Alice")
                .await;
        let ferrum_access = ferrum_auth["access_token"].as_str().unwrap().to_string();
        let ferrum_refresh = ferrum_auth["refresh_token"].as_str().unwrap().to_string();
        let ferrum_app_user_id = ferrum_auth["user"]["app_user_id"]
            .as_str()
            .unwrap()
            .to_string();
        let ferrum_session_id = ferrum_auth["session_id"].as_str().unwrap().to_string();
        let alice_sub = ferrum_auth["user"]["sub"].as_str().unwrap().to_string();

        let collision_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/sign-up")
            .header("x-publishable-key", &elysiar.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "email": alice_email,
                    "password": "AttackerNewPassword999!",
                    "display_name": "Evil Alice"
                })
                .to_string(),
            ))
            .unwrap();
        let collision_res = send_request(&app, collision_req).await;
        assert_eq!(collision_res.status(), StatusCode::CONFLICT);

        let elysiar_signin_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/sign-in")
            .header("x-publishable-key", &elysiar.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "email": alice_email,
                    "password": alice_pass
                })
                .to_string(),
            ))
            .unwrap();
        let elysiar_signin_res = send_request(&app, elysiar_signin_req).await;
        assert_eq!(elysiar_signin_res.status(), StatusCode::OK);
        let elysiar_auth = read_json(elysiar_signin_res).await;
        let elysiar_access = elysiar_auth["access_token"].as_str().unwrap().to_string();
        let elysiar_refresh = elysiar_auth["refresh_token"].as_str().unwrap().to_string();
        let elysiar_app_user_id = elysiar_auth["user"]["app_user_id"]
            .as_str()
            .unwrap()
            .to_string();
        let elysiar_session_id = elysiar_auth["session_id"].as_str().unwrap().to_string();

        assert_eq!(elysiar_auth["user"]["sub"].as_str().unwrap(), alice_sub);
        assert_ne!(ferrum_app_user_id, elysiar_app_user_id);
        assert_ne!(ferrum_session_id, elysiar_session_id);

        let cross_me_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/client/users/me")
            .header("x-publishable-key", &elysiar.publishable_key)
            .header(AUTHORIZATION, format!("Bearer {ferrum_access}"))
            .body(Body::empty())
            .unwrap();
        let cross_me_res = send_request(&app, cross_me_req).await;
        assert_eq!(cross_me_res.status(), StatusCode::UNAUTHORIZED);

        let cross_refresh_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/refresh")
            .header("x-publishable-key", &elysiar.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "refresh_token": ferrum_refresh }).to_string(),
            ))
            .unwrap();
        let cross_refresh_res = send_request(&app, cross_refresh_req).await;
        assert_eq!(cross_refresh_res.status(), StatusCode::UNAUTHORIZED);

        let ferrum_sessions_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/client/sessions")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header(AUTHORIZATION, format!("Bearer {ferrum_access}"))
            .body(Body::empty())
            .unwrap();
        let ferrum_sessions_res = send_request(&app, ferrum_sessions_req).await;
        assert_eq!(ferrum_sessions_res.status(), StatusCode::OK);
        let ferrum_sessions = read_json(ferrum_sessions_res).await;
        let session_ids: Vec<&str> = ferrum_sessions
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| s["id"].as_str())
            .collect();
        assert!(session_ids.contains(&ferrum_session_id.as_str()));
        assert!(!session_ids.contains(&elysiar_session_id.as_str()));

        sqlx::query("UPDATE application_users SET status = 'suspended' WHERE id = $1")
            .bind(Uuid::parse_str(&ferrum_app_user_id).unwrap())
            .execute(&state.db)
            .await
            .expect("suspend alice in ferrum");

        let suspended_refresh_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/refresh")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "refresh_token": ferrum_refresh }).to_string(),
            ))
            .unwrap();
        let suspended_refresh_res = send_request(&app, suspended_refresh_req).await;
        assert_eq!(suspended_refresh_res.status(), StatusCode::FORBIDDEN);

        let elysiar_me_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/client/users/me")
            .header("x-publishable-key", &elysiar.publishable_key)
            .header(AUTHORIZATION, format!("Bearer {elysiar_access}"))
            .body(Body::empty())
            .unwrap();
        let elysiar_me_res = send_request(&app, elysiar_me_req).await;
        assert_eq!(elysiar_me_res.status(), StatusCode::OK);

        let elysiar_refresh_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/refresh")
            .header("x-publishable-key", &elysiar.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "refresh_token": elysiar_refresh }).to_string(),
            ))
            .unwrap();
        let elysiar_refresh_res = send_request(&app, elysiar_refresh_req).await;
        assert_eq!(elysiar_refresh_res.status(), StatusCode::OK);

        let secondary_email = format!("alice-alt-{}@example.com", Uuid::new_v4().simple());
        let user_id: UserId = alice_sub.parse().unwrap();
        let (_, sec_otp) = client_auth::request_secondary_email_verification(
            &state.db,
            &state.config,
            elysiar.env_id,
            user_id,
            &secondary_email,
        )
        .await
        .expect("request secondary email otp");

        let confirm_sec_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/verify-email")
            .header("x-publishable-key", &elysiar.publishable_key)
            .header(AUTHORIZATION, format!("Bearer {elysiar_access}"))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "email": secondary_email,
                    "code": sec_otp
                })
                .to_string(),
            ))
            .unwrap();
        let confirm_sec_res = send_request(&app, confirm_sec_req).await;
        assert_eq!(confirm_sec_res.status(), StatusCode::OK);
        let sec_body = read_json(confirm_sec_res).await;
        assert_eq!(sec_body["status"], "verified");

        let verified_row = global_identity::find_verified_email(&state.db, &secondary_email)
            .await
            .expect("query verified email");
        assert!(verified_row.is_some());
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn refresh_token_rotation_reuse_detection_and_concurrency() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, _) = provision_two_apps(&app).await;

        let email = format!("rot-{}@example.com", Uuid::new_v4().simple());
        let pass = "RotationTestPass123!";
        let auth = register_and_verify_user(&app, &state, &ferrum, &email, pass, "Rotator").await;
        let rt_1 = auth["refresh_token"].as_str().unwrap().to_string();

        let rot1_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/refresh")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(json!({ "refresh_token": rt_1 }).to_string()))
            .unwrap();
        let rot1_res = send_request(&app, rot1_req).await;
        assert_eq!(rot1_res.status(), StatusCode::OK);
        let rot1_body = read_json(rot1_res).await;
        let rt_2 = rot1_body["refresh_token"].as_str().unwrap().to_string();
        assert_ne!(rt_1, rt_2);

        let replay_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/refresh")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(json!({ "refresh_token": rt_1 }).to_string()))
            .unwrap();
        let replay_res = send_request(&app, replay_req).await;
        assert_eq!(replay_res.status(), StatusCode::UNAUTHORIZED);

        let family_revoked_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/refresh")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(json!({ "refresh_token": rt_2 }).to_string()))
            .unwrap();
        let family_revoked_res = send_request(&app, family_revoked_req).await;
        assert_eq!(family_revoked_res.status(), StatusCode::UNAUTHORIZED);

        let reuse_audit_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM audit_logs WHERE environment_id = $1 AND event = 'REFRESH_TOKEN_REUSE_DETECTED'",
        )
        .bind(ferrum.env_id)
        .fetch_one(&state.db)
        .await
        .expect("count reuse audit logs");
        assert!(reuse_audit_count >= 1);
        assert!(count_outbox_events(&state.db, ferrum.env_id, "session.revoked").await >= 1);

        let fresh_signin = client_auth::sign_in(
            &state.db,
            &state.redis,
            &state.config,
            ferrum.env_id,
            &ferrum.publishable_key,
            &email,
            pass,
            None,
            None,
        )
        .await
        .expect("fresh sign in");

        let db_a = state.db.clone();
        let db_b = state.db.clone();
        let cfg_a = state.config.clone();
        let cfg_b = state.config.clone();
        let pk_a = ferrum.publishable_key.clone();
        let pk_b = ferrum.publishable_key.clone();
        let rt_a = fresh_signin.refresh_token.clone();
        let rt_b = fresh_signin.refresh_token.clone();

        let (res_a, res_b) = tokio::join!(
            client_auth::refresh_tokens(&db_a, &cfg_a, ferrum.env_id, &pk_a, &rt_a, None, None),
            client_auth::refresh_tokens(&db_b, &cfg_b, ferrum.env_id, &pk_b, &rt_b, None, None),
        );

        let ok_count = usize::from(res_a.is_ok()) + usize::from(res_b.is_ok());
        let err_count = usize::from(res_a.is_err()) + usize::from(res_b.is_err());
        assert_eq!(ok_count, 1);
        assert_eq!(err_count, 1);
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn upstream_social_oauth_security_state_replay_and_unverified_email_blocking() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, elysiar) = provision_two_apps(&app).await;

        let start_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/client/auth/oauth/google/start?redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fauth%2Fgoogle%2Fcallback")
            .header("x-publishable-key", &ferrum.publishable_key)
            .body(Body::empty())
            .unwrap();
        let start_res = send_request(&app, start_req).await;
        assert_eq!(start_res.status(), StatusCode::OK);
        let start_body = read_json(start_res).await;
        let auth_url = start_body["authorization_url"].as_str().unwrap();
        let state_token = start_body["state"].as_str().unwrap().to_string();

        assert!(auth_url.contains("code_challenge_method=S256"));
        assert!(auth_url.contains("code_challenge="));
        assert!(auth_url.contains("nonce="));
        assert!(auth_url.contains(&format!("state={state_token}")));

        let wrong_env_cb = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/oauth/google/callback")
            .header("x-publishable-key", &elysiar.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "code": "fake-code",
                    "state": state_token
                })
                .to_string(),
            ))
            .unwrap();
        let wrong_env_res = send_request(&app, wrong_env_cb).await;
        assert_eq!(wrong_env_res.status(), StatusCode::UNAUTHORIZED);

        let wrong_provider_cb = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/oauth/github/callback")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "code": "fake-code",
                    "state": state_token
                })
                .to_string(),
            ))
            .unwrap();
        let wrong_provider_res = send_request(&app, wrong_provider_cb).await;
        assert_eq!(wrong_provider_res.status(), StatusCode::UNAUTHORIZED);

        let replay_state_cb = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/oauth/google/callback")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "code": "fake-code",
                    "state": state_token
                })
                .to_string(),
            ))
            .unwrap();
        let replay_state_res = send_request(&app, replay_state_cb).await;
        assert_eq!(replay_state_res.status(), StatusCode::UNAUTHORIZED);

        let bob_email = format!("bob-{}@example.com", Uuid::new_v4().simple());
        let bob_auth = register_and_verify_user(
            &app,
            &state,
            &ferrum,
            &bob_email,
            "BobStrongPass123!",
            "Bob",
        )
        .await;
        let bob_sub = bob_auth["user"]["sub"].as_str().unwrap().to_string();

        let unverified_link = social_oauth::complete_social_login_with_identity(
            &state.db,
            &state.config,
            ferrum.env_id,
            &ferrum.publishable_key,
            VerifiedSocialIdentity {
                provider: "github".to_string(),
                provider_user_id: format!("gh-unverified-{}", Uuid::new_v4().simple()),
                email: bob_email.clone(),
                email_verified: false,
                display_name: "Bob".to_string(),
                avatar_url: None,
            },
            None,
            None,
        )
        .await;
        assert!(matches!(
            unverified_link,
            Err(auth_api::errors::AppError::Forbidden(_))
        ));

        let verified_link = social_oauth::complete_social_login_with_identity(
            &state.db,
            &state.config,
            ferrum.env_id,
            &ferrum.publishable_key,
            VerifiedSocialIdentity {
                provider: "github".to_string(),
                provider_user_id: format!("gh-verified-{}", Uuid::new_v4().simple()),
                email: bob_email,
                email_verified: true,
                display_name: "Bob".to_string(),
                avatar_url: Some("https://avatars.example/bob.png".to_string()),
            },
            None,
            None,
        )
        .await
        .expect("verified social account links cleanly");

        assert_eq!(verified_link.user.sub.to_string(), bob_sub);
        let verified_claims = verify_tenant_access_token(
            &state.db,
            &state.config,
            &verified_link.access_token,
            ferrum.env_id,
        )
        .await
        .expect("social token validates as ES256");
        assert_eq!(verified_claims.amr, vec!["oauth".to_string()]);
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn global_password_reset_revokes_all_environments_and_sso_sessions() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, elysiar) = provision_two_apps(&app).await;

        let carol_email = format!("carol-{}@example.com", Uuid::new_v4().simple());
        let old_pass = "CarolOldPass123!";
        let new_pass = "CarolBrandNewPass456!";

        let ferrum_auth =
            register_and_verify_user(&app, &state, &ferrum, &carol_email, old_pass, "Carol").await;
        let ferrum_rt = ferrum_auth["refresh_token"].as_str().unwrap().to_string();
        let carol_id: UserId = ferrum_auth["user"]["sub"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();

        let elysiar_signin = client_auth::sign_in(
            &state.db,
            &state.redis,
            &state.config,
            elysiar.env_id,
            &elysiar.publishable_key,
            &carol_email,
            old_pass,
            None,
            None,
        )
        .await
        .expect("carol signs into elysiar");
        let elysiar_rt = elysiar_signin.refresh_token.clone();

        let sso_session = global_identity::create_sso_session(
            &state.db,
            carol_id,
            &format!("sso_hash_{}", Uuid::new_v4().simple()),
            &["pwd".to_string()],
            "aal1",
            None,
            None,
            Utc::now() + Duration::hours(24),
        )
        .await
        .expect("create central sso session");

        let raw_reset_token = client_auth::request_password_reset(
            &state.db,
            &state.config,
            ferrum.env_id,
            &carol_email,
        )
        .await
        .expect("request password reset")
        .expect("reset token returned for existing user");

        let reset_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/password/reset")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "token": raw_reset_token,
                    "new_password": new_pass
                })
                .to_string(),
            ))
            .unwrap();
        let reset_res = send_request(&app, reset_req).await;
        assert_eq!(reset_res.status(), StatusCode::OK);

        let ferrum_refresh_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/refresh")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "refresh_token": ferrum_rt }).to_string(),
            ))
            .unwrap();
        assert_eq!(
            send_request(&app, ferrum_refresh_req).await.status(),
            StatusCode::UNAUTHORIZED
        );

        let elysiar_refresh_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/client/auth/refresh")
            .header("x-publishable-key", &elysiar.publishable_key)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "refresh_token": elysiar_rt }).to_string(),
            ))
            .unwrap();
        assert_eq!(
            send_request(&app, elysiar_refresh_req).await.status(),
            StatusCode::UNAUTHORIZED
        );

        let sso_revoked = sqlx::query_scalar::<_, bool>(
            "SELECT revoked_at IS NOT NULL FROM sso_sessions WHERE id = $1",
        )
        .bind(sso_session.id)
        .fetch_one(&state.db)
        .await
        .expect("check sso session revoked");
        assert!(sso_revoked);

        assert!(count_outbox_events(&state.db, ferrum.env_id, "session.revoked").await >= 1);
        assert!(count_outbox_events(&state.db, elysiar.env_id, "session.revoked").await >= 1);

        let old_login = client_auth::sign_in(
            &state.db,
            &state.redis,
            &state.config,
            elysiar.env_id,
            &elysiar.publishable_key,
            &carol_email,
            old_pass,
            None,
            None,
        )
        .await;
        assert!(old_login.is_err());

        let new_login = client_auth::sign_in(
            &state.db,
            &state.redis,
            &state.config,
            elysiar.env_id,
            &elysiar.publishable_key,
            &carol_email,
            new_pass,
            None,
            None,
        )
        .await;
        assert!(new_login.is_ok());
    }
}
