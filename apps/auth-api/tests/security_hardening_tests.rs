mod support;

use auth_api::middleware::rate_limit::{auth_rate_limit_layer, resolve_client_ip};
use auth_api::services::{identity_advanced, otp, password_hasher, retention};
use auth_api::AppState;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::routing::get;
use axum::{Extension, Router};
use deadpool_redis::{Config as RedisConfig, Runtime};
use ipnet::IpNet;
use sqlx::postgres::PgPoolOptions;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::process::Command;
use tower::ServiceExt;
use uuid::Uuid;

#[test]
fn startup_rejects_missing_placeholder_and_insecure_production_configuration() {
    let directory = std::env::temp_dir().join(format!("auth-config-{}", Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let secret = Uuid::new_v4().to_string();
    for (jwt, kek, production, secure, issuer, expected) in [
        (
            None,
            Some(secret.as_str()),
            false,
            "false",
            "http://localhost",
            "JWT_SECRET must be explicitly specified",
        ),
        (
            Some(secret.as_str()),
            None,
            false,
            "false",
            "http://localhost",
            "SIGNING_KEY_KEK must be explicitly specified",
        ),
        (
            Some("dev_jwt_secret_change_me_in_production_min_32_chars"),
            Some(secret.as_str()),
            false,
            "false",
            "http://localhost",
            "non-placeholder",
        ),
        (
            Some(secret.as_str()),
            Some(secret.as_str()),
            true,
            "false",
            "https://auth.example.test",
            "Production requires",
        ),
        (
            Some(secret.as_str()),
            Some(secret.as_str()),
            true,
            "true",
            "http://auth.example.test",
            "Production requires",
        ),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_auth-api"));
        command
            .current_dir(&directory)
            .env_clear()
            .env("DATABASE_URL", "postgres://127.0.0.1:1/unused")
            .env(
                "APP_ENV",
                if production {
                    "production"
                } else {
                    "development"
                },
            )
            .env("COOKIE_SECURE", secure)
            .env("ISSUER_URL", issuer);
        if let Some(value) = jwt {
            command.env("JWT_SECRET", value);
        }
        if let Some(value) = kek {
            command.env("SIGNING_KEY_KEK", value);
        }
        let result = command.output().unwrap();
        assert!(!result.status.success());
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains(expected), "Expected {expected}; got {error}");
        assert!(!error.contains(&secret));
    }
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn config_debug_omits_credentials_and_private_keys() {
    let config = support::test_config(false);
    let rendered = format!("{config:?}");
    for value in [
        &config.jwt_secret,
        &config.signing_key_kek,
        &config.database_url,
        &config.redis_url,
    ] {
        assert!(!rendered.contains(value));
    }
    let key = auth_api::services::signing_keys::generate_es256_keypair().unwrap();
    assert!(!format!("{key:?}").contains("pkcs8"));
}

#[test]
fn proxy_headers_are_accepted_only_from_trusted_hops() {
    let trusted: Vec<IpNet> = vec!["10.0.0.0/24".parse().unwrap()];
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-forwarded-for",
        "198.51.100.99, 203.0.113.8, 10.0.0.2".parse().unwrap(),
    );
    assert_eq!(
        resolve_client_ip(&headers, "10.0.0.1".parse().unwrap(), &trusted).unwrap(),
        "203.0.113.8".parse::<IpAddr>().unwrap()
    );
    assert_eq!(
        resolve_client_ip(&headers, "192.0.2.1".parse().unwrap(), &trusted).unwrap(),
        "192.0.2.1".parse::<IpAddr>().unwrap()
    );
    headers.insert("x-forwarded-for", "malformed".parse().unwrap());
    assert_eq!(
        resolve_client_ip(&headers, "10.0.0.1".parse().unwrap(), &trusted),
        Err(StatusCode::BAD_REQUEST)
    );
}

#[tokio::test]
async fn password_limits_are_applied_before_hashing_and_verification() {
    let password = "🔐".repeat(128);
    let hash = password_hasher::hash_password_async(password.clone())
        .await
        .unwrap();
    assert!(
        password_hasher::verify_password_async(password, hash.clone())
            .await
            .unwrap()
    );
    assert!(password_hasher::hash_password_async("a".repeat(129))
        .await
        .is_err());
    assert!(
        password_hasher::verify_password_async("a".repeat(129), hash)
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and TEST_REDIS_URL"]
async fn redis_limits_and_otp_attempts_are_atomic_across_instances() {
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL");
    let redis_url =
        std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6399".to_string());
    let database = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .unwrap();
    let redis = RedisConfig::from_url(&redis_url)
        .create_pool(Some(Runtime::Tokio1))
        .unwrap();
    let mut config = support::test_config(false);
    config.database_url = database_url;
    config.redis_url = redis_url;
    let state = AppState {
        db: database,
        redis: redis.clone(),
        config: std::sync::Arc::new(config),
    };
    let app = Router::new()
        .route("/v1/client/probe", get(|| async { StatusCode::NO_CONTENT }))
        .layer(auth_rate_limit_layer())
        .layer(Extension(state));
    let peer = IpAddr::V6(Ipv6Addr::from(Uuid::new_v4().as_u128()));
    for attempt in 0..6 {
        let mut request = Request::builder()
            .uri("/v1/client/probe")
            .body(Body::empty())
            .unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::new(peer, 40001)));
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            if attempt < 5 {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::TOO_MANY_REQUESTS
            }
        );
        if attempt == 5 {
            assert!(response.headers().contains_key("retry-after"));
        }
    }
    let key = format!("test:otp:{}", Uuid::new_v4());
    otp::store(&redis, &key, "123456", 60).await.unwrap();
    for _ in 0..5 {
        assert!(!otp::consume(&redis, &key, "000000").await.unwrap());
    }
    assert!(!otp::consume(&redis, &key, "123456").await.unwrap());
    otp::store(&redis, &key, "654321", 60).await.unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let redis = redis.clone();
        let key = key.clone();
        tasks.spawn(async move { otp::consume(&redis, &key, "654321").await.unwrap() });
    }
    let mut successes = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            successes += 1;
        }
    }
    assert_eq!(successes, 1);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn erasure_anonymizes_identity_and_retention_preserves_pending_events() {
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL");
    let database = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&database).await.unwrap();
    let id = Uuid::new_v4();
    let email = format!("erase-{id}@example.test");
    sqlx::query("INSERT INTO users (id, email, display_name, password_hash, avatar_url, email_verified) VALUES ($1, $2, 'Sensitive Name', 'password-hash', 'https://example.test/avatar', TRUE)")
        .bind(id).bind(&email).execute(&database).await.unwrap();
    sqlx::query("INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, TRUE, NOW())")
        .bind(id).bind(&email).execute(&database).await.unwrap();
    identity_advanced::delete_user_account(&database, id.into())
        .await
        .unwrap();
    let erased: (String, String, Option<String>, Option<String>, String) = sqlx::query_as(
        "SELECT email, display_name, password_hash, avatar_url, status FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&database)
    .await
    .unwrap();
    assert!(erased.0.ends_with("@erased.invalid"));
    assert_eq!(erased.1, "Deleted User");
    assert!(erased.2.is_none());
    assert!(erased.3.is_none());
    assert_eq!(erased.4, "deleted");
    let emails: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_emails WHERE user_id = $1")
        .bind(id)
        .fetch_one(&database)
        .await
        .unwrap();
    assert_eq!(emails, 0);
    retention::purge_expired_records(&database, 90)
        .await
        .unwrap();
}
