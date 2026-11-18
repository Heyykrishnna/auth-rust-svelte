use auth_api::models::EnvironmentKind;
use auth_api::services::credentials::{
    constant_time_hash_eq, generate_publishable_key, generate_secret_key,
    key_matches_environment_kind, validate_secret_scopes,
};
use auth_api::services::domains::{
    generate_dns_verification_token, validate_domain_name, validate_origin, validate_redirect_uri,
    validate_slug,
};

#[test]
fn test_credential_generation_prefixes_and_hashing() {
    let (dev_pk_prefix, dev_pk) = generate_publishable_key(EnvironmentKind::Development);
    assert_eq!(dev_pk_prefix, "pk_test_");
    assert!(dev_pk.starts_with("pk_test_"));
    assert!(key_matches_environment_kind(
        &dev_pk,
        EnvironmentKind::Development
    ));
    assert!(!key_matches_environment_kind(
        &dev_pk,
        EnvironmentKind::Production
    ));

    let (prod_pk_prefix, prod_pk) = generate_publishable_key(EnvironmentKind::Production);
    assert_eq!(prod_pk_prefix, "pk_live_");
    assert!(prod_pk.starts_with("pk_live_"));
    assert!(key_matches_environment_kind(
        &prod_pk,
        EnvironmentKind::Production
    ));
    assert!(!key_matches_environment_kind(
        &prod_pk,
        EnvironmentKind::Development
    ));

    let sk_dev = generate_secret_key(EnvironmentKind::Development);
    assert_eq!(sk_dev.prefix, "sk_test_");
    assert!(sk_dev.plaintext.starts_with("sk_test_"));
    assert_eq!(sk_dev.plaintext.len(), "sk_test_".len() + 64);
    assert_eq!(sk_dev.hash.len(), 64);
    assert!(constant_time_hash_eq(&sk_dev.hash, &sk_dev.hash));
    assert!(!constant_time_hash_eq(&sk_dev.hash, &"0".repeat(64)));
    assert!(key_matches_environment_kind(
        &sk_dev.plaintext,
        EnvironmentKind::Development
    ));
    assert!(!key_matches_environment_kind(
        &sk_dev.plaintext,
        EnvironmentKind::Production
    ));
}

#[test]
fn test_secret_scope_validation() {
    let valid = validate_secret_scopes(&[
        "users:read".to_string(),
        "sync:read".to_string(),
        "users:read".to_string(),
    ])
    .expect("valid scopes should normalize");
    assert_eq!(
        valid,
        vec!["users:read".to_string(), "sync:read".to_string()]
    );

    assert!(validate_secret_scopes(&[]).is_err());
    assert!(validate_secret_scopes(&["admin:superpower".to_string()]).is_err());
}

#[test]
fn test_domain_origin_and_redirect_uri_policies() {
    assert_eq!(validate_slug("Ferrum-App").unwrap(), "ferrum-app");
    assert!(validate_slug("-bad").is_err());
    assert!(validate_slug("bad_slug").is_err());

    assert_eq!(
        validate_domain_name(EnvironmentKind::Development, "localhost:5173").unwrap(),
        "localhost:5173"
    );
    assert!(validate_domain_name(EnvironmentKind::Production, "localhost:5173").is_err());
    assert!(validate_domain_name(EnvironmentKind::Production, "*.ferrum.app").is_err());
    assert_eq!(
        validate_domain_name(EnvironmentKind::Production, "Ferrum.App").unwrap(),
        "ferrum.app"
    );

    assert_eq!(
        validate_origin(EnvironmentKind::Development, "http://localhost:3000/").unwrap(),
        "http://localhost:3000"
    );
    assert!(validate_origin(EnvironmentKind::Development, "http://evil.com").is_err());
    assert!(validate_origin(EnvironmentKind::Production, "http://ferrum.app").is_err());
    assert!(validate_origin(EnvironmentKind::Production, "https://localhost:3000").is_err());
    assert_eq!(
        validate_origin(EnvironmentKind::Production, "https://ferrum.app").unwrap(),
        "https://ferrum.app"
    );

    assert_eq!(
        validate_redirect_uri(
            EnvironmentKind::Development,
            "http://localhost:5173/auth/callback"
        )
        .unwrap(),
        "http://localhost:5173/auth/callback"
    );
    assert_eq!(
        validate_redirect_uri(
            EnvironmentKind::Production,
            "https://ferrum.app/auth/callback"
        )
        .unwrap(),
        "https://ferrum.app/auth/callback"
    );
    assert!(validate_redirect_uri(
        EnvironmentKind::Production,
        "https://ferrum.app/auth/callback#frag"
    )
    .is_err());
    assert!(validate_redirect_uri(
        EnvironmentKind::Production,
        "https://ferrum.app@evil.com/callback"
    )
    .is_err());
    assert!(validate_redirect_uri(
        EnvironmentKind::Production,
        "https://ferrum.app/auth/../evil"
    )
    .is_err());
    assert!(
        validate_redirect_uri(EnvironmentKind::Production, "https://*.ferrum.app/callback")
            .is_err()
    );

    let dns_token = generate_dns_verification_token();
    assert!(dns_token.starts_with("auth-platform-verify="));
}

#[cfg(test)]
mod live_control_plane_http_suite {
    use auth_api::config::AppConfig;
    use auth_api::routes::build_router;
    use auth_api::AppState;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{Method, Request, StatusCode};
    use deadpool_redis::{Config as RedisConfig, Runtime as RedisRuntime};
    use serde_json::{json, Value};
    use sqlx::postgres::PgPoolOptions;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn send_request(app: &axum::Router, mut req: Request<Body>) -> axum::response::Response {
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))));
        app.clone().oneshot(req).await.unwrap()
    }

    async fn build_test_app() -> axum::Router {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for live control plane HTTP tests");

        let db = PgPoolOptions::new()
            .max_connections(4)
            .connect(&db_url)
            .await
            .expect("connect to postgres");

        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .expect("run migrations");

        let redis_cfg = RedisConfig::from_url("redis://127.0.0.1:6399");
        let redis = redis_cfg
            .create_pool(Some(RedisRuntime::Tokio1))
            .expect("create redis pool");

        let config = Arc::new(AppConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            database_url: db_url,
            db_max_connections: 4,
            redis_url: "redis://127.0.0.1:6399".to_string(),
            jwt_secret: "phase02_integration_test_jwt_secret_at_least_32_chars".to_string(),
            jwt_access_expiry_secs: 900,
            jwt_refresh_expiry_secs: 86400,
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

        build_router(AppState { config, db, redis })
    }

    async fn read_json(res: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&bytes).expect("parse json")
    }

    async fn register_dev_and_get_workspace(app: &axum::Router, email: &str) -> (String, String) {
        let req = Request::builder()
            .method(Method::POST)
            .uri("/v1/control/auth/sign-up")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "email": email,
                    "password": "StrongPassword123!",
                    "display_name": "Dev Operator"
                })
                .to_string(),
            ))
            .unwrap();
        let res = send_request(app, req).await;
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = read_json(res).await;
        let token = body["token"].as_str().unwrap().to_string();

        let ws_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/control/workspaces")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let ws_res = send_request(app, ws_req).await;
        assert_eq!(ws_res.status(), StatusCode::OK);
        let ws_list = read_json(ws_res).await;
        let ws_id = ws_list[0]["id"].as_str().unwrap().to_string();

        (token, ws_id)
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn test_developer_isolation_credentials_and_dynamic_cors() {
        let app = build_test_app().await;
        let suffix = Uuid::new_v4().simple().to_string();

        let (dev_a_token, ws_a_id) =
            register_dev_and_get_workspace(&app, &format!("dev-a-{suffix}@example.com")).await;
        let (dev_b_token, _) =
            register_dev_and_get_workspace(&app, &format!("dev-b-{suffix}@example.com")).await;

        let create_app_req = Request::builder()
            .method(Method::POST)
            .uri("/v1/control/applications")
            .header("authorization", format!("Bearer {dev_a_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "workspace_id": ws_a_id,
                    "name": "Ferrum",
                    "slug": format!("ferrum-{suffix}")
                })
                .to_string(),
            ))
            .unwrap();
        let create_app_res = send_request(&app, create_app_req).await;
        assert_eq!(create_app_res.status(), StatusCode::CREATED);
        let created_app = read_json(create_app_res).await;
        let app_id = created_app["application"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let dev_pk = created_app["development_publishable_key"]
            .as_str()
            .unwrap()
            .to_string();
        let initial_sk = created_app["initial_development_secret_key"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(dev_pk.starts_with("pk_test_"));
        assert!(initial_sk.starts_with("sk_test_"));

        let dev_b_probe = Request::builder()
            .method(Method::GET)
            .uri(format!("/v1/control/applications/{app_id}"))
            .header("authorization", format!("Bearer {dev_b_token}"))
            .body(Body::empty())
            .unwrap();
        let dev_b_res = send_request(&app, dev_b_probe).await;
        assert_eq!(dev_b_res.status(), StatusCode::FORBIDDEN);

        let dev_b_cred_probe = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/v1/control/applications/{app_id}/environments/development/credentials"
            ))
            .header("authorization", format!("Bearer {dev_b_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "credential_type": "secret",
                    "name": "Stolen Key",
                    "scopes": ["users:read"]
                })
                .to_string(),
            ))
            .unwrap();
        assert_eq!(
            send_request(&app, dev_b_cred_probe).await.status(),
            StatusCode::FORBIDDEN
        );

        let list_creds_req = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "/v1/control/applications/{app_id}/environments/development/credentials"
            ))
            .header("authorization", format!("Bearer {dev_a_token}"))
            .body(Body::empty())
            .unwrap();
        let list_creds_res = send_request(&app, list_creds_req).await;
        assert_eq!(list_creds_res.status(), StatusCode::OK);
        let creds_json = read_json(list_creds_res).await;
        let creds_text = creds_json.to_string();
        assert!(
            !creds_text.contains(&initial_sk),
            "Secret key plaintext must never appear in list responses"
        );
        assert!(
            !creds_text.contains("secret_hash"),
            "Secret hash must not be serialized in API responses"
        );

        let ping_ok_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/backend/ping")
            .header("authorization", format!("Bearer {initial_sk}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            send_request(&app, ping_ok_req).await.status(),
            StatusCode::OK
        );

        let low_scope_req = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/v1/control/applications/{app_id}/environments/development/credentials"
            ))
            .header("authorization", format!("Bearer {dev_a_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "credential_type": "secret",
                    "name": "Sync Only Key",
                    "scopes": ["sync:read"]
                })
                .to_string(),
            ))
            .unwrap();
        let low_scope_res = send_request(&app, low_scope_req).await;
        assert_eq!(low_scope_res.status(), StatusCode::CREATED);
        let low_scope_body = read_json(low_scope_res).await;
        let low_scope_id = low_scope_body["credential"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let low_scope_sk = low_scope_body["secret_plaintext"]
            .as_str()
            .unwrap()
            .to_string();

        let ping_forbidden_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/backend/ping")
            .header("authorization", format!("Bearer {low_scope_sk}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            send_request(&app, ping_forbidden_req).await.status(),
            StatusCode::FORBIDDEN
        );

        let revoke_req = Request::builder()
            .method(Method::DELETE)
            .uri(format!(
                "/v1/control/applications/{app_id}/environments/development/credentials/{low_scope_id}"
            ))
            .header("authorization", format!("Bearer {dev_a_token}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            send_request(&app, revoke_req).await.status(),
            StatusCode::NO_CONTENT
        );

        let revoked_ping_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/backend/ping")
            .header("authorization", format!("Bearer {low_scope_sk}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            send_request(&app, revoked_ping_req).await.status(),
            StatusCode::UNAUTHORIZED
        );

        let cors_before_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/client/config")
            .header("x-publishable-key", &dev_pk)
            .header("origin", "http://localhost:5173")
            .body(Body::empty())
            .unwrap();
        let cors_before_res = send_request(&app, cors_before_req).await;
        assert_eq!(cors_before_res.status(), StatusCode::OK);
        assert!(
            cors_before_res
                .headers()
                .get("access-control-allow-origin")
                .is_none(),
            "Unregistered origin must not receive Access-Control-Allow-Origin"
        );

        let add_domain_req = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/v1/control/applications/{app_id}/environments/development/domains"
            ))
            .header("authorization", format!("Bearer {dev_a_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "domain": "localhost:5173",
                    "allowed_origins": ["http://localhost:5173"],
                    "redirect_uris": ["http://localhost:5173/auth/callback"]
                })
                .to_string(),
            ))
            .unwrap();
        let add_domain_res = send_request(&app, add_domain_req).await;
        assert_eq!(add_domain_res.status(), StatusCode::CREATED);
        let domain_body = read_json(add_domain_res).await;
        let domain_id = domain_body["id"].as_str().unwrap().to_string();

        let cors_after_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/client/config")
            .header("x-publishable-key", &dev_pk)
            .header("origin", "http://localhost:5173")
            .body(Body::empty())
            .unwrap();
        let cors_after_res = send_request(&app, cors_after_req).await;
        assert_eq!(cors_after_res.status(), StatusCode::OK);
        assert_eq!(
            cors_after_res
                .headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok()),
            Some("http://localhost:5173")
        );

        let preflight_ok_req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/v1/client/config")
            .header("origin", "http://localhost:5173")
            .body(Body::empty())
            .unwrap();
        let preflight_ok_res = send_request(&app, preflight_ok_req).await;
        assert_eq!(preflight_ok_res.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            preflight_ok_res
                .headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok()),
            Some("http://localhost:5173")
        );

        let bad_prod_domain_req = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/v1/control/applications/{app_id}/environments/production/domains"
            ))
            .header("authorization", format!("Bearer {dev_a_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "domain": "ferrum.app",
                    "allowed_origins": ["http://ferrum.app"],
                    "redirect_uris": ["https://ferrum.app/callback#bad"]
                })
                .to_string(),
            ))
            .unwrap();
        assert_eq!(
            send_request(&app, bad_prod_domain_req).await.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );

        let del_domain_req = Request::builder()
            .method(Method::DELETE)
            .uri(format!(
                "/v1/control/applications/{app_id}/environments/development/domains/{domain_id}"
            ))
            .header("authorization", format!("Bearer {dev_a_token}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            send_request(&app, del_domain_req).await.status(),
            StatusCode::NO_CONTENT
        );

        let cors_removed_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/client/config")
            .header("x-publishable-key", &dev_pk)
            .header("origin", "http://localhost:5173")
            .body(Body::empty())
            .unwrap();
        let cors_removed_res = send_request(&app, cors_removed_req).await;
        assert!(
            cors_removed_res
                .headers()
                .get("access-control-allow-origin")
                .is_none(),
            "Removed origin must immediately lose CORS allowance without server restart"
        );
    }
}
