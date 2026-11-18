use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use auth_api::models::EnvironmentKind;
use auth_api::services::backend_sync::{decode_sync_cursor, encode_sync_cursor};
use auth_api::services::webhooks::{
    build_signature_header, compute_next_retry_delay_secs, is_blocked_ip, is_permanent_4xx,
    validate_webhook_url, verify_signature_header, DEFAULT_SIGNATURE_TOLERANCE_SECS,
};
use chrono::Utc;
use uuid::Uuid;

#[test]
fn webhook_hmac_signature_and_dual_secret_rotation_verification() {
    let old_secret = "whsec_old_rotation_secret_00000000000000000000000000000001";
    let new_secret = "whsec_new_rotation_secret_00000000000000000000000000000002";
    let wrong_secret = "whsec_wrong_secret_0000000000000000000000000000000000000003";

    let now = Utc::now().timestamp();
    let body = br#"{"spec_version":"1.0","event_type":"user.updated","aggregate_version":4}"#;

    let single_header = build_signature_header(&[new_secret], now, body);
    assert!(verify_signature_header(
        &single_header,
        &[new_secret],
        now,
        body,
        now,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_ok());

    assert!(verify_signature_header(
        &single_header,
        &[new_secret],
        now,
        br#"{"spec_version":"1.0","event_type":"user.updated","aggregate_version":999}"#,
        now,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_err());

    assert!(verify_signature_header(
        &single_header,
        &[new_secret],
        now + 1,
        body,
        now,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_err());

    assert!(verify_signature_header(
        &single_header,
        &[new_secret],
        now - 301,
        body,
        now,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_err());

    let dual_header = build_signature_header(&[new_secret, old_secret], now, body);
    assert!(verify_signature_header(
        &dual_header,
        &[old_secret],
        now,
        body,
        now,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_ok());
    assert!(verify_signature_header(
        &dual_header,
        &[new_secret],
        now,
        body,
        now,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_ok());
    assert!(verify_signature_header(
        &dual_header,
        &[wrong_secret],
        now,
        body,
        now,
        DEFAULT_SIGNATURE_TOLERANCE_SECS,
    )
    .is_err());
}

#[test]
fn webhook_ssrf_url_and_ip_defense_policy() {
    assert!(is_blocked_ip(
        IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
        false
    ));
    assert!(is_blocked_ip(
        IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
        true
    ));
    assert!(is_blocked_ip(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        false
    ));
    assert!(!is_blocked_ip(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        true
    ));
    assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), false));
    assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), true));
    assert!(is_blocked_ip(
        IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1)),
        false
    ));
    assert!(is_blocked_ip(
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        false
    ));
    assert!(is_blocked_ip(
        IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)),
        false
    ));
    assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST), false));
    assert!(!is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST), true));
    assert!(is_blocked_ip(
        IpAddr::V6("fe80::1".parse::<Ipv6Addr>().unwrap()),
        false
    ));
    assert!(is_blocked_ip(
        IpAddr::V6("fc00::1".parse::<Ipv6Addr>().unwrap()),
        false
    ));
    assert!(is_blocked_ip(
        IpAddr::V6("::ffff:169.254.169.254".parse::<Ipv6Addr>().unwrap()),
        true
    ));
    assert!(!is_blocked_ip(
        IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
        false
    ));

    assert!(
        validate_webhook_url("https://hooks.ferrum.app/auth", EnvironmentKind::Production).is_ok()
    );
    assert!(
        validate_webhook_url("http://hooks.ferrum.app/auth", EnvironmentKind::Production).is_err()
    );
    assert!(
        validate_webhook_url("http://localhost:4000/hooks", EnvironmentKind::Production).is_err()
    );
    assert!(validate_webhook_url("https://127.0.0.1/hooks", EnvironmentKind::Production).is_err());
    assert!(validate_webhook_url(
        "https://169.254.169.254/latest",
        EnvironmentKind::Production
    )
    .is_err());
    assert!(validate_webhook_url("https://10.0.0.1/hooks", EnvironmentKind::Production).is_err());
    assert!(validate_webhook_url("https://[::1]/hooks", EnvironmentKind::Production).is_err());

    assert!(
        validate_webhook_url("http://localhost:4000/hooks", EnvironmentKind::Development).is_ok()
    );
    assert!(
        validate_webhook_url("http://127.0.0.1:4000/hooks", EnvironmentKind::Development).is_ok()
    );
    assert!(validate_webhook_url(
        "https://169.254.169.254/latest",
        EnvironmentKind::Development
    )
    .is_err());
    assert!(validate_webhook_url("https://10.0.0.1/hooks", EnvironmentKind::Development).is_err());
}

#[test]
fn webhook_retry_backoff_and_sync_cursor_roundtrip() {
    assert_eq!(compute_next_retry_delay_secs(1, false), Some(10));
    assert_eq!(compute_next_retry_delay_secs(2, false), Some(30));
    assert_eq!(compute_next_retry_delay_secs(8, false), Some(43200));
    assert_eq!(compute_next_retry_delay_secs(9, false), None);

    assert_eq!(compute_next_retry_delay_secs(1, true), Some(30));
    assert_eq!(compute_next_retry_delay_secs(3, true), Some(600));
    assert_eq!(compute_next_retry_delay_secs(4, true), None);

    assert!(is_permanent_4xx(Some(400)));
    assert!(is_permanent_4xx(Some(404)));
    assert!(!is_permanent_4xx(Some(408)));
    assert!(!is_permanent_4xx(Some(429)));
    assert!(!is_permanent_4xx(Some(500)));
    assert!(!is_permanent_4xx(None));

    let now = Utc::now();
    let id = Uuid::new_v4();
    let encoded = encode_sync_cursor(now, id).expect("encode cursor");
    let (decoded_ts, decoded_id) = decode_sync_cursor(&encoded).expect("decode cursor");
    assert_eq!(decoded_ts, now);
    assert_eq!(decoded_id, id);
    assert!(decode_sync_cursor("not-valid-base64!!!").is_err());
}

#[cfg(test)]
mod live_postgres_phase05_tests {
    use std::collections::HashSet;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::Arc;

    use auth_api::config::AppConfig;
    use auth_api::models::{
        AggregateType, DomainEventEnvelope, DomainEventType, EnvironmentId, UserId,
    };
    use auth_api::repositories::db::begin_tx;
    use auth_api::repositories::outbox;
    use auth_api::routes::build_router;
    use auth_api::services::client_auth;
    use auth_api::services::webhooks::{
        run_worker_tick, verify_signature_header, DEFAULT_SIGNATURE_TOLERANCE_SECS,
        WEBHOOK_EVENT_ID_HEADER, WEBHOOK_SIGNATURE_HEADER, WEBHOOK_TIMESTAMP_HEADER,
    };
    use auth_api::AppState;
    use axum::body::{Body, Bytes};
    use axum::extract::ConnectInfo;
    use axum::http::header::{AUTHORIZATION, COOKIE, LOCATION};
    use axum::http::{HeaderMap, Method, Request, StatusCode};
    use axum::routing::post;
    use axum::Router;
    use chrono::{Duration, Utc};
    use deadpool_redis::{Config as RedisConfig, Runtime as RedisRuntime};
    use serde_json::{json, Value};
    use sqlx::postgres::PgPoolOptions;
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
    use tower::ServiceExt;
    use uuid::Uuid;

    static IP_COUNTER: AtomicU16 = AtomicU16::new(1);

    #[derive(Debug, Clone)]
    struct ReceivedWebhook {
        event_id: String,
        timestamp: i64,
        signature: String,
        raw_body: Vec<u8>,
        payload: Value,
    }

    async fn spawn_mock_webhook_consumer(
        status_code: Arc<AtomicU16>,
        redirect_location: Option<String>,
    ) -> (String, Arc<Mutex<Vec<ReceivedWebhook>>>) {
        let received = Arc::new(Mutex::new(Vec::<ReceivedWebhook>::new()));
        let rec_clone = received.clone();
        let status_clone = status_code.clone();

        let app = Router::new().route(
            "/hook",
            post(move |headers: HeaderMap, body: Bytes| {
                let rec = rec_clone.clone();
                let st = status_clone.clone();
                let redir = redirect_location.clone();
                async move {
                    let event_id = headers
                        .get(WEBHOOK_EVENT_ID_HEADER)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string();
                    let timestamp = headers
                        .get(WEBHOOK_TIMESTAMP_HEADER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<i64>().ok())
                        .unwrap_or(0);
                    let signature = headers
                        .get(WEBHOOK_SIGNATURE_HEADER)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string();
                    let payload: Value =
                        serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));

                    rec.lock().await.push(ReceivedWebhook {
                        event_id,
                        timestamp,
                        signature,
                        raw_body: body.to_vec(),
                        payload,
                    });

                    let code = StatusCode::from_u16(st.load(Ordering::SeqCst))
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                    if let Some(loc) = redir {
                        let mut resp_headers = HeaderMap::new();
                        resp_headers.insert(LOCATION, loc.parse().unwrap());
                        return (code, resp_headers, "redirecting".to_string());
                    }
                    (code, HeaderMap::new(), "ok".to_string())
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock webhook server");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        (format!("http://127.0.0.1:{}/hook", addr.port()), received)
    }

    async fn build_live_test_env() -> (Router, AppState) {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for live Phase 05 tests");
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
            .expect("reset signing_keys");
        sqlx::query("DELETE FROM webhook_endpoints")
            .execute(&db)
            .await
            .expect("reset webhook fixtures");
        sqlx::query("DELETE FROM outbox_events")
            .execute(&db)
            .await
            .expect("reset outbox fixtures");

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
            jwt_secret: "phase05_integration_test_jwt_secret_at_least_32_chars".to_string(),
            issuer_url: "https://auth.phase05.test".to_string(),
            signing_key_kek: "phase05_live_test_kek_material_at_least_32_characters_01".to_string(),
            signing_key_kek_version: "v1".to_string(),
            jwt_access_expiry_secs: 300,
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

        let state = AppState { config, db, redis };
        let router = build_router(state.clone());
        (router, state)
    }

    async fn send_request(app: &Router, mut req: Request<Body>) -> axum::response::Response {
        let octet = (IP_COUNTER.fetch_add(1, Ordering::Relaxed) % 250) as u8 + 1;
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 5, octet], 42000))));
        app.clone().oneshot(req).await.expect("send request")
    }

    async fn read_json(res: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&bytes).expect("parse json")
    }

    struct ProvisionedApp {
        app_id: String,
        env_id: EnvironmentId,
        publishable_key: String,
        secret_key: String,
        cookie_header: String,
        csrf_token: String,
    }

    async fn provision_two_apps(app: &Router) -> (ProvisionedApp, ProvisionedApp) {
        let dev_email = format!("dev-{}@phase05.test", Uuid::new_v4().simple());
        let sign_up = Request::builder()
            .method(Method::POST)
            .uri("/v1/control/auth/sign-up")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "email": dev_email,
                    "password": "StrongDevPassword123!",
                    "display_name": "Phase 05 Architect",
                    "workspace_name": "Phase 05 Workspace"
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
                let app_id = body["application"]["id"].as_str().unwrap().to_string();
                let dev_env = body["environments"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|e| e["kind"] == "development")
                    .expect("development environment");
                let env_id: EnvironmentId = dev_env["id"].as_str().unwrap().parse().unwrap();
                let publishable_key = body["development_publishable_key"]
                    .as_str()
                    .unwrap()
                    .to_string();
                let secret_key = body["initial_development_secret_key"]
                    .as_str()
                    .unwrap()
                    .to_string();
                ProvisionedApp {
                    app_id,
                    env_id,
                    publishable_key,
                    secret_key,
                    cookie_header: cookie,
                    csrf_token: csrf,
                }
            }
        };

        let ferrum = create_app("Ferrum", "ferrum").await;
        let elysiar = create_app("Elysiar", "elysiar").await;
        (ferrum, elysiar)
    }

    async fn register_and_verify_user(
        app: &Router,
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
                json!({ "email": email, "code": otp }).to_string(),
            ))
            .unwrap();
        let verify_res = send_request(app, verify_req).await;
        assert_eq!(verify_res.status(), StatusCode::CREATED);
        read_json(verify_res).await
    }

    async fn register_dev_webhook(
        app: &Router,
        target: &ProvisionedApp,
        url: &str,
        subscribed_events: Vec<&str>,
    ) -> (Uuid, String) {
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/v1/control/applications/{}/environments/development/webhooks",
                target.app_id
            ))
            .header(COOKIE, &target.cookie_header)
            .header("x-csrf-token", &target.csrf_token)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "url": url,
                    "description": "Integration test consumer",
                    "subscribed_events": subscribed_events
                })
                .to_string(),
            ))
            .unwrap();
        let res = send_request(app, req).await;
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = read_json(res).await;
        let id = Uuid::parse_str(body["endpoint"]["id"].as_str().unwrap()).unwrap();
        let secret = body["signing_secret"].as_str().unwrap().to_string();
        assert!(secret.starts_with("whsec_"));
        (id, secret)
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn transactional_outbox_atomicity_rollback_vs_commit() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, _) = provision_two_apps(&app).await;

        let email = format!("atom-{}@example.com", Uuid::new_v4().simple());
        let auth =
            register_and_verify_user(&app, &state, &ferrum, &email, "AtomPass123!", "Atom").await;
        let access_token = auth["access_token"].as_str().unwrap().to_string();
        let app_user_id = Uuid::parse_str(auth["user"]["app_user_id"].as_str().unwrap()).unwrap();

        let rolled_back_evt_id = {
            let mut tx = begin_tx(&state.db).await.expect("begin tx");
            let evt = DomainEventEnvelope::new(
                DomainEventType::UserUpdated,
                Some(ferrum.env_id.into_inner()),
                AggregateType::ApplicationUser,
                app_user_id,
                99,
                json!({ "display_name": "Should Never Commit" }),
            );
            let id = outbox::insert_outbox_event(&mut *tx, ferrum.env_id, &evt)
                .await
                .expect("insert in tx");
            tx.rollback().await.expect("rollback tx");
            id
        };

        let rolled_back_count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM outbox_events WHERE id = $1")
                .bind(rolled_back_evt_id)
                .fetch_one(&state.db)
                .await
                .expect("query rolled back outbox");
        assert_eq!(rolled_back_count, 0);

        let patch_req = Request::builder()
            .method(Method::PATCH)
            .uri("/v1/client/users/me")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header(AUTHORIZATION, format!("Bearer {access_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "display_name": "Atom Committed Name", "avatar_url": null }).to_string(),
            ))
            .unwrap();
        let patch_res = send_request(&app, patch_req).await;
        assert_eq!(patch_res.status(), StatusCode::OK);
        let updated = read_json(patch_res).await;
        let version = updated["version"].as_i64().unwrap();
        assert!(version >= 2);

        let committed_version = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT aggregate_version
            FROM outbox_events
            WHERE environment_id = $1 AND event_type = 'user.updated' AND aggregate_id = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(ferrum.env_id)
        .bind(app_user_id)
        .fetch_one(&state.db)
        .await
        .expect("committed outbox row");
        assert_eq!(committed_version, version);
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn global_fanout_isolation_ferrum_receives_elysiar_never_receives() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, elysiar) = provision_two_apps(&app).await;

        let status_ok = Arc::new(AtomicU16::new(200));
        let (ferrum_url, ferrum_received) =
            spawn_mock_webhook_consumer(status_ok.clone(), None).await;
        let (elysiar_url, elysiar_received) =
            spawn_mock_webhook_consumer(status_ok.clone(), None).await;

        let (_, ferrum_secret) = register_dev_webhook(&app, &ferrum, &ferrum_url, vec!["*"]).await;
        let (_, _) = register_dev_webhook(&app, &elysiar, &elysiar_url, vec!["*"]).await;

        let alice_email = format!("fanout-{}@example.com", Uuid::new_v4().simple());
        let auth = register_and_verify_user(
            &app,
            &state,
            &ferrum,
            &alice_email,
            "FanoutPass123!",
            "Alice Fanout",
        )
        .await;
        let access_token = auth["access_token"].as_str().unwrap().to_string();

        let update_req = Request::builder()
            .method(Method::PATCH)
            .uri("/v1/client/users/me")
            .header("x-publishable-key", &ferrum.publishable_key)
            .header(AUTHORIZATION, format!("Bearer {access_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "display_name": "Alice Renamed", "avatar_url": null }).to_string(),
            ))
            .unwrap();
        assert_eq!(
            send_request(&app, update_req).await.status(),
            StatusCode::OK
        );

        let stats = run_worker_tick(&state.db, &state.config, 100)
            .await
            .expect("worker tick");
        assert!(stats.deliveries_succeeded >= 1);

        let ferrum_msgs = ferrum_received.lock().await;
        let elysiar_msgs = elysiar_received.lock().await;
        assert!(
            ferrum_msgs
                .iter()
                .any(|m| m.payload["event_type"] == "user.updated"),
            "Ferrum consumer must receive user.updated"
        );
        assert!(
            elysiar_msgs.is_empty(),
            "Elysiar consumer must receive zero events for non-member Alice"
        );

        let now = Utc::now().timestamp();
        for msg in ferrum_msgs.iter() {
            assert!(!msg.event_id.is_empty());
            assert!(verify_signature_header(
                &msg.signature,
                &[&ferrum_secret],
                msg.timestamp,
                &msg.raw_body,
                now,
                DEFAULT_SIGNATURE_TOLERANCE_SECS,
            )
            .is_ok());
        }
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn ssrf_blocks_production_internal_ips_and_worker_blocks_http_redirects() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, _) = provision_two_apps(&app).await;

        for blocked_url in [
            "https://169.254.169.254/latest/meta-data/",
            "https://127.0.0.1/hook",
            "https://10.0.0.1/hook",
            "https://[::1]/hook",
            "http://localhost:4000/hook",
        ] {
            let req = Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/v1/control/applications/{}/environments/production/webhooks",
                    ferrum.app_id
                ))
                .header(COOKIE, &ferrum.cookie_header)
                .header("x-csrf-token", &ferrum.csrf_token)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "url": blocked_url,
                        "subscribed_events": ["user.updated"]
                    })
                    .to_string(),
                ))
                .unwrap();
            let res = send_request(&app, req).await;
            assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        }

        let redir_status = Arc::new(AtomicU16::new(302));
        let (redir_url, _) = spawn_mock_webhook_consumer(
            redir_status,
            Some("http://169.254.169.254/latest/meta-data/".to_string()),
        )
        .await;

        let (endpoint_id, _) =
            register_dev_webhook(&app, &ferrum, &redir_url, vec!["webhook.test"]).await;

        let test_ping_req = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/v1/control/applications/{}/environments/development/webhooks/{}/test",
                ferrum.app_id, endpoint_id
            ))
            .header(COOKIE, &ferrum.cookie_header)
            .header("x-csrf-token", &ferrum.csrf_token)
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            send_request(&app, test_ping_req).await.status(),
            StatusCode::ACCEPTED
        );

        let stats = run_worker_tick(&state.db, &state.config, 50)
            .await
            .expect("worker tick");
        assert_eq!(stats.deliveries_succeeded, 0);
        assert!(stats.deliveries_failed >= 1);

        let list_deliv_req = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "/v1/control/applications/{}/environments/development/webhooks/{}/deliveries",
                ferrum.app_id, endpoint_id
            ))
            .header(COOKIE, &ferrum.cookie_header)
            .body(Body::empty())
            .unwrap();
        let list_deliv_res = send_request(&app, list_deliv_req).await;
        assert_eq!(list_deliv_res.status(), StatusCode::OK);
        let deliveries = read_json(list_deliv_res).await;
        let failed_entry = deliveries
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["status"] == "failed")
            .expect("failed redirect delivery");
        assert_eq!(failed_entry["response_status"], 302);
        assert!(failed_entry["response_body_excerpt"]
            .as_str()
            .unwrap()
            .contains("blocked by SSRF policy"));
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn retry_backoff_dlq_dual_secret_rotation_and_manual_replay() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, _) = provision_two_apps(&app).await;

        let consumer_status = Arc::new(AtomicU16::new(500));
        let (consumer_url, received) =
            spawn_mock_webhook_consumer(consumer_status.clone(), None).await;

        let (endpoint_id, old_secret) =
            register_dev_webhook(&app, &ferrum, &consumer_url, vec!["webhook.test"]).await;

        let ping_req = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/v1/control/applications/{}/environments/development/webhooks/{}/test",
                ferrum.app_id, endpoint_id
            ))
            .header(COOKIE, &ferrum.cookie_header)
            .header("x-csrf-token", &ferrum.csrf_token)
            .body(Body::empty())
            .unwrap();
        let ping_res = send_request(&app, ping_req).await;
        assert_eq!(ping_res.status(), StatusCode::ACCEPTED);
        let initial_delivery = read_json(ping_res).await;
        let initial_delivery_id = initial_delivery["id"].as_str().unwrap().to_string();

        let mut saw_dead_letter = false;
        for _ in 0..10 {
            sqlx::query(
                "UPDATE webhook_deliveries SET next_retry_at = NOW() - INTERVAL '1 second' WHERE webhook_endpoint_id = $1 AND status IN ('pending', 'retrying')",
            )
            .bind(endpoint_id)
            .execute(&state.db)
            .await
            .expect("fast forward retry time");

            let stats = run_worker_tick(&state.db, &state.config, 50)
                .await
                .expect("worker tick");
            if stats.deliveries_dead_lettered > 0 {
                saw_dead_letter = true;
                break;
            }
        }
        assert!(
            saw_dead_letter,
            "Exhausted retries must transition to dead_letter"
        );

        let rotate_req = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/v1/control/applications/{}/environments/development/webhooks/{}/rotate-secret",
                ferrum.app_id, endpoint_id
            ))
            .header(COOKIE, &ferrum.cookie_header)
            .header("x-csrf-token", &ferrum.csrf_token)
            .header("content-type", "application/json")
            .body(Body::from(json!({ "grace_hours": 24 }).to_string()))
            .unwrap();
        let rotate_res = send_request(&app, rotate_req).await;
        assert_eq!(rotate_res.status(), StatusCode::OK);
        let rotated = read_json(rotate_res).await;
        let new_secret = rotated["signing_secret"].as_str().unwrap().to_string();
        assert_ne!(old_secret, new_secret);
        assert_eq!(rotated["endpoint"]["has_retiring_secret"], true);

        let reactivate_req = Request::builder()
            .method(Method::PATCH)
            .uri(format!(
                "/v1/control/applications/{}/environments/development/webhooks/{}",
                ferrum.app_id, endpoint_id
            ))
            .header(COOKIE, &ferrum.cookie_header)
            .header("x-csrf-token", &ferrum.csrf_token)
            .header("content-type", "application/json")
            .body(Body::from(json!({ "status": "active" }).to_string()))
            .unwrap();
        assert_eq!(
            send_request(&app, reactivate_req).await.status(),
            StatusCode::OK
        );

        consumer_status.store(200, Ordering::SeqCst);
        received.lock().await.clear();

        let replay_req = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/v1/control/applications/{}/environments/development/webhooks/{}/deliveries/{}/replay",
                ferrum.app_id, endpoint_id, initial_delivery_id
            ))
            .header(COOKIE, &ferrum.cookie_header)
            .header("x-csrf-token", &ferrum.csrf_token)
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            send_request(&app, replay_req).await.status(),
            StatusCode::ACCEPTED
        );

        let replay_stats = run_worker_tick(&state.db, &state.config, 50)
            .await
            .expect("replay tick");
        assert_eq!(replay_stats.deliveries_succeeded, 1);

        let msgs = received.lock().await;
        assert_eq!(msgs.len(), 1);
        let msg = &msgs[0];
        let now = Utc::now().timestamp();
        assert!(verify_signature_header(
            &msg.signature,
            &[&old_secret],
            msg.timestamp,
            &msg.raw_body,
            now,
            DEFAULT_SIGNATURE_TOLERANCE_SECS,
        )
        .is_ok());
        assert!(verify_signature_header(
            &msg.signature,
            &[&new_secret],
            msg.timestamp,
            &msg.raw_body,
            now,
            DEFAULT_SIGNATURE_TOLERANCE_SECS,
        )
        .is_ok());
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn backend_cursor_reconciliation_250_users_metadata_suspend_sessions_and_events() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, elysiar) = provision_two_apps(&app).await;

        let sync_start = Utc::now() - Duration::minutes(1);

        for i in 0..250 {
            let uid = Uuid::new_v4();
            let email = format!("ferrum-sync-{i}-{}@example.com", uid.simple());
            sqlx::query(
                "INSERT INTO users (id, email, display_name, email_verified, status) VALUES ($1, $2, $3, TRUE, 'active')",
            )
            .bind(uid)
            .bind(&email)
            .bind(format!("Ferrum User {i}"))
            .execute(&state.db)
            .await
            .expect("insert ferrum user");

            sqlx::query(
                "INSERT INTO application_users (environment_id, user_id, status) VALUES ($1, $2, 'active')",
            )
            .bind(ferrum.env_id)
            .bind(uid)
            .execute(&state.db)
            .await
            .expect("insert ferrum membership");
        }

        for i in 0..30 {
            let uid = Uuid::new_v4();
            let email = format!("elysiar-sync-{i}-{}@example.com", uid.simple());
            sqlx::query(
                "INSERT INTO users (id, email, display_name, email_verified, status) VALUES ($1, $2, $3, TRUE, 'active')",
            )
            .bind(uid)
            .bind(&email)
            .bind(format!("Elysiar User {i}"))
            .execute(&state.db)
            .await
            .expect("insert elysiar user");

            sqlx::query(
                "INSERT INTO application_users (environment_id, user_id, status) VALUES ($1, $2, 'active')",
            )
            .bind(elysiar.env_id)
            .bind(uid)
            .execute(&state.db)
            .await
            .expect("insert elysiar membership");
        }

        let mut seen_ids = HashSet::new();
        let mut cursor: Option<String> = None;
        let mut pages = 0usize;

        loop {
            let mut uri = format!(
                "/v1/backend/users?updated_since={}&limit=60",
                urlencoding::encode(&sync_start.to_rfc3339())
            );
            if let Some(ref c) = cursor {
                uri.push_str(&format!("&cursor={}", urlencoding::encode(c)));
            }

            let req = Request::builder()
                .method(Method::GET)
                .uri(&uri)
                .header(AUTHORIZATION, format!("Bearer {}", ferrum.secret_key))
                .body(Body::empty())
                .unwrap();
            let res = send_request(&app, req).await;
            assert_eq!(res.status(), StatusCode::OK);
            let body = read_json(res).await;
            let items = body["data"].as_array().unwrap();
            pages += 1;

            for item in items {
                let email = item["email"].as_str().unwrap();
                assert!(email.starts_with("ferrum-sync-"));
                let app_user_id = item["app_user_id"].as_str().unwrap().to_string();
                assert!(
                    seen_ids.insert(app_user_id),
                    "Pagination must not return duplicate app_user_id"
                );
            }

            if body["has_more"].as_bool() == Some(true) {
                cursor = body["next_cursor"].as_str().map(ToString::to_string);
                assert!(cursor.is_some());
            } else {
                assert!(body["next_cursor"].is_null());
                break;
            }
        }

        assert_eq!(seen_ids.len(), 250);
        assert!(pages >= 5);

        let managed_email = format!("managed-{}@example.com", Uuid::new_v4().simple());
        let auth = register_and_verify_user(
            &app,
            &state,
            &ferrum,
            &managed_email,
            "ManagedPass123!",
            "Managed User",
        )
        .await;
        let managed_app_user_id = auth["user"]["app_user_id"].as_str().unwrap().to_string();
        let managed_sub: UserId = auth["user"]["sub"].as_str().unwrap().parse().unwrap();
        let managed_sid = auth["session_id"].as_str().unwrap().to_string();

        let cross_lookup = Request::builder()
            .method(Method::GET)
            .uri(format!("/v1/backend/users/{managed_app_user_id}"))
            .header(AUTHORIZATION, format!("Bearer {}", elysiar.secret_key))
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            send_request(&app, cross_lookup).await.status(),
            StatusCode::NOT_FOUND
        );

        let by_sub_req = Request::builder()
            .method(Method::GET)
            .uri(format!("/v1/backend/users/by-sub/{managed_sub}"))
            .header(AUTHORIZATION, format!("Bearer {}", ferrum.secret_key))
            .body(Body::empty())
            .unwrap();
        let by_sub_res = send_request(&app, by_sub_req).await;
        assert_eq!(by_sub_res.status(), StatusCode::OK);

        let meta_req = Request::builder()
            .method(Method::PATCH)
            .uri(format!("/v1/backend/users/{managed_app_user_id}/metadata"))
            .header(AUTHORIZATION, format!("Bearer {}", ferrum.secret_key))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "external_id": "crm_cust_99",
                    "public_metadata": { "plan": "pro" },
                    "private_metadata": { "stripe_id": "cus_123" }
                })
                .to_string(),
            ))
            .unwrap();
        let meta_res = send_request(&app, meta_req).await;
        assert_eq!(meta_res.status(), StatusCode::OK);
        let meta_body = read_json(meta_res).await;
        assert_eq!(meta_body["external_id"], "crm_cust_99");
        assert_eq!(meta_body["public_metadata"]["plan"], "pro");
        assert_eq!(meta_body["private_metadata"]["stripe_id"], "cus_123");

        let sess_active_req = Request::builder()
            .method(Method::GET)
            .uri(format!("/v1/backend/sessions/{managed_sid}"))
            .header(AUTHORIZATION, format!("Bearer {}", ferrum.secret_key))
            .body(Body::empty())
            .unwrap();
        let sess_active_res = send_request(&app, sess_active_req).await;
        assert_eq!(sess_active_res.status(), StatusCode::OK);
        assert_eq!(read_json(sess_active_res).await["status"], "active");

        let suspend_req = Request::builder()
            .method(Method::POST)
            .uri(format!("/v1/backend/users/{managed_app_user_id}/suspend"))
            .header(AUTHORIZATION, format!("Bearer {}", ferrum.secret_key))
            .header("content-type", "application/json")
            .body(Body::from(json!({ "reason": "billing_hold" }).to_string()))
            .unwrap();
        let suspend_res = send_request(&app, suspend_req).await;
        assert_eq!(suspend_res.status(), StatusCode::OK);
        assert_eq!(read_json(suspend_res).await["status"], "suspended");

        let sess_revoked_req = Request::builder()
            .method(Method::GET)
            .uri(format!("/v1/backend/sessions/{managed_sid}"))
            .header(AUTHORIZATION, format!("Bearer {}", ferrum.secret_key))
            .body(Body::empty())
            .unwrap();
        let sess_revoked_res = send_request(&app, sess_revoked_req).await;
        assert_eq!(sess_revoked_res.status(), StatusCode::OK);
        assert_eq!(read_json(sess_revoked_res).await["status"], "revoked");

        let restore_req = Request::builder()
            .method(Method::POST)
            .uri(format!("/v1/backend/users/{managed_app_user_id}/restore"))
            .header(AUTHORIZATION, format!("Bearer {}", ferrum.secret_key))
            .body(Body::empty())
            .unwrap();
        let restore_res = send_request(&app, restore_req).await;
        assert_eq!(restore_res.status(), StatusCode::OK);
        assert_eq!(read_json(restore_res).await["status"], "active");

        let events_req = Request::builder()
            .method(Method::GET)
            .uri("/v1/backend/events?types=user.suspended,membership.updated&limit=50")
            .header(AUTHORIZATION, format!("Bearer {}", ferrum.secret_key))
            .body(Body::empty())
            .unwrap();
        let events_res = send_request(&app, events_req).await;
        assert_eq!(events_res.status(), StatusCode::OK);
        let events_body = read_json(events_res).await;
        let evt_list = events_body["data"].as_array().unwrap();
        assert!(evt_list.iter().any(|e| e["event_type"] == "user.suspended"));
        assert!(evt_list
            .iter()
            .any(|e| e["event_type"] == "membership.updated"));

        let first_evt_id = evt_list[0]["id"].as_str().unwrap();
        let catchup_req = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "/v1/backend/events?after_event_id={first_evt_id}&types=user.suspended,membership.updated&limit=50"
            ))
            .header(AUTHORIZATION, format!("Bearer {}", ferrum.secret_key))
            .body(Body::empty())
            .unwrap();
        let catchup_res = send_request(&app, catchup_req).await;
        assert_eq!(catchup_res.status(), StatusCode::OK);
        let catchup_body = read_json(catchup_res).await;
        let catchup_list = catchup_body["data"].as_array().unwrap();
        assert_eq!(catchup_list.len(), evt_list.len() - 1);
    }
}
