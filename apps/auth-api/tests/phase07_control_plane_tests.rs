use auth_api::models::WorkspaceRole;
use auth_api::services::control_ops::{
    can_manage_target_role, clamp_pagination, normalize_oauth_provider, workspace_role_allows,
    workspace_role_rank,
};
use auth_api::services::signing_keys::{
    decrypt_envelope, encrypt_envelope, OAUTH_SECRET_AAD_PURPOSE,
};

#[test]
fn test_workspace_rbac_permission_matrix() {
    assert_eq!(workspace_role_rank("owner"), 4);
    assert_eq!(workspace_role_rank("admin"), 3);
    assert_eq!(workspace_role_rank("developer"), 2);
    assert_eq!(workspace_role_rank("viewer"), 1);
    assert_eq!(workspace_role_rank("unknown"), 0);

    assert!(workspace_role_allows("viewer", WorkspaceRole::Viewer));
    assert!(!workspace_role_allows("viewer", WorkspaceRole::Developer));
    assert!(!workspace_role_allows("viewer", WorkspaceRole::Admin));
    assert!(!workspace_role_allows("viewer", WorkspaceRole::Owner));

    assert!(workspace_role_allows("developer", WorkspaceRole::Viewer));
    assert!(workspace_role_allows("developer", WorkspaceRole::Developer));
    assert!(!workspace_role_allows("developer", WorkspaceRole::Admin));

    assert!(workspace_role_allows("admin", WorkspaceRole::Admin));
    assert!(!workspace_role_allows("admin", WorkspaceRole::Owner));
    assert!(workspace_role_allows("owner", WorkspaceRole::Owner));

    assert!(can_manage_target_role("owner", "owner"));
    assert!(can_manage_target_role("owner", "admin"));
    assert!(can_manage_target_role("admin", "admin"));
    assert!(can_manage_target_role("admin", "developer"));
    assert!(can_manage_target_role("admin", "viewer"));
    assert!(!can_manage_target_role("admin", "owner"));
    assert!(!can_manage_target_role("developer", "viewer"));
    assert!(!can_manage_target_role("viewer", "viewer"));
}

#[test]
fn test_oauth_provider_normalization_and_pagination_clamping() {
    assert_eq!(normalize_oauth_provider("Google").unwrap(), "google");
    assert_eq!(normalize_oauth_provider(" GITHUB ").unwrap(), "github");
    assert!(normalize_oauth_provider("facebook").is_err());

    assert_eq!(clamp_pagination(None, None, 100), (25, 0));
    assert_eq!(clamp_pagination(Some(0), Some(-5), 100), (1, 0));
    assert_eq!(clamp_pagination(Some(500), Some(10), 100), (100, 10));
}

#[test]
fn test_oauth_custom_secret_envelope_encryption_roundtrip() {
    let kek = "test-kek-material-for-phase07-oauth-encryption";
    let secret = b"custom-oauth-client-secret-value";
    let ciphertext = encrypt_envelope(secret, kek, "v1", OAUTH_SECRET_AAD_PURPOSE).unwrap();
    assert_ne!(ciphertext.as_slice(), secret);

    let decrypted = decrypt_envelope(&ciphertext, kek, "v1", OAUTH_SECRET_AAD_PURPOSE).unwrap();
    assert_eq!(decrypted.as_slice(), secret);

    assert!(decrypt_envelope(&ciphertext, kek, "v2", OAUTH_SECRET_AAD_PURPOSE).is_err());
}

#[cfg(test)]
mod live_phase07_control_plane_suite {
    use auth_api::config::AppConfig;
    use auth_api::routes::build_router;
    use auth_api::AppState;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{Method, Request, StatusCode};
    use base64::{engine::general_purpose::STANDARD, Engine};
    use deadpool_redis::{Config as RedisConfig, Runtime as RedisRuntime};
    use serde_json::{json, Value};
    use sqlx::postgres::PgPoolOptions;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::Arc;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn send_request(app: &axum::Router, req: Request<Body>) -> axum::response::Response {
        send_request_from(app, req, [127, 0, 0, 1]).await
    }

    async fn send_request_from(
        app: &axum::Router,
        mut req: Request<Body>,
        ip: [u8; 4],
    ) -> axum::response::Response {
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from((ip, 40107))));
        app.clone().oneshot(req).await.unwrap()
    }

    async fn read_json(resp: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    }

    async fn build_test_app() -> (axum::Router, sqlx::PgPool) {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for live Phase 07 tests");
        let db = PgPoolOptions::new()
            .max_connections(4)
            .connect(&db_url)
            .await
            .expect("connect to postgres");
        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .expect("run migrations");
        sqlx::query("DELETE FROM signing_keys")
            .execute(&db)
            .await
            .expect("reset signing key fixtures");
        let redis_url =
            std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6399".into());
        let redis_cfg = RedisConfig::from_url(redis_url.clone());
        let redis = redis_cfg
            .create_pool(Some(RedisRuntime::Tokio1))
            .expect("create redis pool");
        let config = Arc::new(AppConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            database_url: db_url,
            db_max_connections: 4,
            redis_url,
            jwt_secret: "phase07-test-jwt-secret-value-at-least-32-chars".to_string(),
            issuer_url: "http://localhost:8080".to_string(),
            signing_key_kek: "phase07-test-kek-secret-value-at-least-32-chars".to_string(),
            signing_key_kek_version: "v1".to_string(),
            jwt_access_expiry_secs: 900,
            jwt_refresh_expiry_secs: 604800,
            login_max_attempts: 5,
            login_lockout_duration_secs: 900,
            verification_code_expiry_secs: 600,
            password_reset_expiry_secs: 900,
            google_client_id: None,
            google_client_secret: None,
            google_redirect_uri: None,
            github_client_id: None,
            github_client_secret: None,
            github_redirect_uri: None,
            cors_origins: vec!["http://localhost:5173".to_string()],
            cookie_secure: false,
            cookie_domain: None,
            otel_exporter_otlp_endpoint: "http://localhost:4317".to_string(),
            otel_service_name: "auth-api-test".to_string(),
            otel_service_version: "0.1.0".to_string(),
            smtp_host: None,
            smtp_port: 1025,
            smtp_user: None,
            smtp_pass: None,
            smtp_from: "noreply@example.test".to_string(),
            smtp_from_name: "Auth Platform".to_string(),
        });
        let state = AppState {
            db: db.clone(),
            redis,
            config,
        };
        (build_router(state), db)
    }

    async fn sign_up_dev(app: &axum::Router, email: &str, workspace: &str) -> (String, Uuid) {
        static NEXT_TEST_PEER: AtomicU8 = AtomicU8::new(2);
        let peer = [127, 0, 0, NEXT_TEST_PEER.fetch_add(1, Ordering::Relaxed)];
        let res = send_request_from(
            app,
            Request::builder()
                .method(Method::POST)
                .uri("/v1/control/auth/sign-up")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "email": email,
                        "password": "StrongPassword123!",
                        "display_name": "Dev User",
                        "workspace_name": workspace
                    })
                    .to_string(),
                ))
                .unwrap(),
            peer,
        )
        .await;
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = read_json(res).await;
        let token = body["token"].as_str().unwrap().to_string();
        let ws_res = send_request(
            app,
            Request::builder()
                .method(Method::GET)
                .uri("/v1/control/workspaces")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let ws_body = read_json(ws_res).await;
        let ws_id = Uuid::parse_str(ws_body[0]["id"].as_str().unwrap()).unwrap();
        (token, ws_id)
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL and TEST_REDIS_URL"]
    async fn developer_security_uses_console_authentication_and_revokes_owned_sessions() {
        let (app, _) = build_test_app().await;
        let email = format!("security-{}@example.test", Uuid::new_v4());
        let (token, _) = sign_up_dev(&app, &email, "Developer security").await;
        let (other_token, _) = sign_up_dev(
            &app,
            &format!("other-{}@example.test", Uuid::new_v4()),
            "Other workspace",
        )
        .await;
        let auth = "/v1/control/auth";
        let overview_url = format!("{auth}/security");
        let me_url = format!("{auth}/me");

        let request = |method, uri: &str, token: &str| {
            Request::builder()
                .method(method)
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap()
        };
        let signed_in = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri(format!("{auth}/sign-in"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "StrongPassword123!" }).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(signed_in.status(), StatusCode::OK);
        let body = read_json(signed_in).await;
        let second_token = body["token"].as_str().unwrap();

        let response = send_request(
            &app,
            Request::builder()
                .uri(&overview_url)
                .header("cookie", format!("dev_session_token={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let overview = read_json(response).await;
        let sessions = overview["sessions"].as_array().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0]["current"], true);
        assert_eq!(
            overview["recent_activity"][0]["event"],
            "DEVELOPER_SIGNED_IN"
        );

        let id = sessions[1]["id"].as_str().unwrap();
        let revoke_url = format!("{overview_url}/sessions/{id}");
        let response = send_request(&app, request(Method::DELETE, &revoke_url, &other_token)).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = send_request(&app, request(Method::DELETE, &revoke_url, &token)).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let response = send_request(&app, request(Method::GET, &me_url, second_token)).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = send_request(
            &app,
            request(Method::GET, "/api/users/security/summary", &token),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = send_request(&app, request(Method::GET, &me_url, &token)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let change_password = |current_password: &str| {
            Request::builder()
                .method(Method::POST)
                .uri(format!("{auth}/change-password"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "current_password": current_password,
                        "new_password": "UpdatedFixturePassword456!"
                    })
                    .to_string(),
                ))
                .unwrap()
        };
        let response = send_request(&app, change_password("IncorrectPassword")).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let response = send_request(&app, change_password("StrongPassword123!")).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = send_request(&app, request(Method::GET, &overview_url, &token)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let overview = read_json(response).await;
        assert_eq!(
            overview["recent_activity"][0]["event"],
            "DEVELOPER_PASSWORD_CHANGED"
        );
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL and TEST_REDIS_URL"]
    async fn developer_authenticator_and_backup_codes_are_enforced_at_sign_in() {
        use auth_api::services::account_security::{base32_decode, compute_totp_code};
        use chrono::Utc;

        let (app, _) = build_test_app().await;
        let email = format!("authenticator-{}@example.test", Uuid::new_v4());
        let password = "StrongPassword123!";
        let (token, _) = sign_up_dev(&app, &email, "Authenticator tests").await;
        let request = |method, path: &str, bearer: Option<&str>, body: Value| {
            let mut request = Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json");
            if let Some(bearer) = bearer {
                request = request.header("authorization", format!("Bearer {bearer}"));
            }
            request
                .body(Body::from(body.to_string()))
                .expect("valid developer security request")
        };

        let mut peer = 20;
        let mut run = |req: Request<Body>| {
            peer += 1;
            send_request_from(&app, req, [198, 51, 100, peer])
        };
        let security = "/v1/control/auth/security";
        let start = format!("{security}/totp/start");
        let result = run(request(Method::POST, &start, Some(&token), json!({}))).await;
        assert_eq!(result.status(), StatusCode::FORBIDDEN);
        assert_eq!(read_json(result).await["error"]["code"], "STEP_UP_REQUIRED");

        let verify = format!("{security}/step-up");
        let result = run(request(
            Method::POST,
            &verify,
            Some(&token),
            json!({ "password": password }),
        ))
        .await;
        assert_eq!(result.status(), StatusCode::OK);

        let result = run(request(Method::POST, &start, Some(&token), json!({}))).await;
        assert_eq!(result.status(), StatusCode::OK);
        assert_eq!(result.headers()["cache-control"], "no-store");
        let enrollment = read_json(result).await;
        let secret = base32_decode(enrollment["secret"].as_str().unwrap()).unwrap();
        let code = compute_totp_code(&secret, Utc::now().timestamp() as u64);
        let finish = format!("{security}/totp/finish");
        let result = run(request(
            Method::POST,
            &finish,
            Some(&token),
            json!({ "code": &code }),
        ))
        .await;
        assert_eq!(result.status(), StatusCode::OK);
        let backup = read_json(result).await;
        let codes = backup["codes"].as_array().unwrap();
        assert_eq!(codes.len(), 10);

        let login = "/v1/control/auth/sign-in";
        let result = run(request(
            Method::POST,
            login,
            None,
            json!({ "email": &email, "password": password }),
        ))
        .await;
        assert_eq!(result.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            read_json(result).await["error"]["code"],
            "SECOND_FACTOR_REQUIRED"
        );
        let result = run(request(
            Method::POST,
            login,
            None,
            json!({
                "email": &email, "password": password, "second_factor_code": code
            }),
        ))
        .await;
        assert_eq!(
            read_json(result).await["error"]["code"],
            "INVALID_SECOND_FACTOR"
        );

        let backup_login =
            json!({ "email": &email, "password": password, "second_factor_code": codes[0] });
        let result = run(request(Method::POST, login, None, backup_login.clone())).await;
        assert_eq!(result.status(), StatusCode::OK);
        let result = run(request(Method::POST, login, None, backup_login)).await;
        assert_eq!(
            read_json(result).await["error"]["code"],
            "INVALID_SECOND_FACTOR"
        );

        let result = run(request(Method::GET, security, Some(&token), json!(null))).await;
        assert_eq!(result.status(), StatusCode::OK);
        let overview = read_json(result).await;
        assert_eq!(overview["totp_enabled"], true);
        assert_eq!(overview["backup_codes_remaining"], 9);
        assert_eq!(overview["sessions"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn test_phase07_rbac_and_cross_workspace_isolation() {
        let (app, db) = build_test_app().await;
        let suffix = Uuid::new_v4().simple().to_string();

        let (owner_token, ws_a_id) = sign_up_dev(
            &app,
            &format!("owner-a-{suffix}@example.test"),
            &format!("WS-A-{suffix}"),
        )
        .await;
        let viewer_email = format!("viewer-a-{suffix}@example.test");
        let (viewer_token, _ws_v_id) =
            sign_up_dev(&app, &viewer_email, &format!("WS-V-{suffix}")).await;
        let (outsider_token, _ws_b_id) = sign_up_dev(
            &app,
            &format!("outsider-b-{suffix}@example.test"),
            &format!("WS-B-{suffix}"),
        )
        .await;

        let create_app_res = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/v1/control/applications")
                .header("authorization", format!("Bearer {owner_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "workspace_id": ws_a_id,
                        "name": "Phase07 App",
                        "slug": format!("p07-app-{}", &suffix[..8])
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(create_app_res.status(), StatusCode::CREATED);
        let app_body = read_json(create_app_res).await;
        let app_id = app_body["application"]["id"].as_str().unwrap().to_string();

        let add_viewer = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri(format!("/v1/control/workspaces/{ws_a_id}/members"))
                .header("authorization", format!("Bearer {owner_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "email": viewer_email, "role": "viewer" }).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(add_viewer.status(), StatusCode::CREATED);

        let viewer_overview = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/v1/control/applications/{app_id}/environments/development/overview"
                ))
                .header("authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(viewer_overview.status(), StatusCode::OK);

        let viewer_mutate_oauth = send_request(
            &app,
            Request::builder()
                .method(Method::PUT)
                .uri(format!(
                    "/v1/control/applications/{app_id}/environments/development/oauth-providers/google"
                ))
                .header("authorization", format!("Bearer {viewer_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enabled": true,
                        "use_platform_credentials": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(viewer_mutate_oauth.status(), StatusCode::FORBIDDEN);

        let outsider_users = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/v1/control/applications/{app_id}/environments/development/users"
                ))
                .header("authorization", format!("Bearer {outsider_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(outsider_users.status(), StatusCode::FORBIDDEN);

        let outsider_audit = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/v1/control/applications/{app_id}/environments/development/audit-logs"
                ))
                .header("authorization", format!("Bearer {outsider_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(outsider_audit.status(), StatusCode::FORBIDDEN);

        let path = format!("/v1/control/workspaces/{ws_a_id}");
        let rename = |token: &str, name: &str| {
            Request::builder()
                .method(Method::PATCH)
                .uri(&path)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": name }).to_string()))
                .unwrap()
        };
        let member = read_json(add_viewer).await;
        let editor_id = member["developer_id"].as_str().unwrap();
        for token in [&viewer_token, &outsider_token] {
            let denied = send_request(&app, rename(token, "Unauthorized change")).await;
            assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        }

        let invalid = send_request(&app, rename(&owner_token, "   ")).await;
        assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let renamed = send_request(&app, rename(&owner_token, "  Studio workspace  ")).await;
        assert_eq!(renamed.status(), StatusCode::OK);
        let updated = read_json(renamed).await;
        assert_eq!(updated["id"], ws_a_id.to_string());
        assert_eq!(updated["role"], "owner");
        assert_eq!(updated["name"], "Studio workspace");
        assert!(updated["logo_version"].is_null());

        let logo_request = |token: &str, logo: Value| {
            Request::builder()
                .method(Method::PATCH)
                .uri(&path)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "logo": logo }).to_string()))
                .unwrap()
        };
        let read_logo = |token: &str| {
            Request::builder()
                .uri(format!("{path}/logo"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap()
        };
        let jpeg = |edge: u32| {
            let mut bytes = Vec::new();
            let image = image::RgbImage::from_pixel(edge, edge, image::Rgb([8, 75, 42]));
            image::codecs::jpeg::JpegEncoder::new(&mut bytes)
                .encode_image(&image)
                .unwrap();
            json!({ "action": "replace", "jpeg": STANDARD.encode(bytes) })
        };
        let denied = send_request(&app, logo_request(&viewer_token, jpeg(12))).await;
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        for invalid in [
            json!({ "action": "replace", "jpeg": STANDARD.encode(b"<svg/>") }),
            jpeg(257),
        ] {
            let response = send_request(&app, logo_request(&owner_token, invalid)).await;
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        }
        let replaced = send_request(&app, logo_request(&owner_token, jpeg(12))).await;
        assert_eq!(replaced.status(), StatusCode::OK);
        let profile = read_json(replaced).await;
        assert_eq!(profile["name"], "Studio workspace");
        assert!(profile["logo_version"].is_string());

        let outsider_logo = send_request(&app, read_logo(&outsider_token)).await;
        assert_eq!(outsider_logo.status(), StatusCode::NOT_FOUND);
        let member_logo = send_request(&app, read_logo(&viewer_token)).await;
        assert_eq!(member_logo.status(), StatusCode::OK);
        assert_eq!(member_logo.headers()["content-type"], "image/jpeg");
        assert_eq!(member_logo.headers()["cache-control"], "private, no-store");
        let image = axum::body::to_bytes(member_logo.into_body(), 65536)
            .await
            .unwrap();
        let decoded = image::load_from_memory_with_format(&image, image::ImageFormat::Jpeg)
            .expect("stored logo is a valid JPEG");
        assert_eq!((decoded.width(), decoded.height()), (12, 12));

        let promoted = send_request(
            &app,
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("{path}/members/{editor_id}"))
                .header("authorization", format!("Bearer {owner_token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "role": "admin" }).to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(promoted.status(), StatusCode::OK);

        let renamed = send_request(&app, rename(&viewer_token, "Studio and Labs")).await;
        assert_eq!(renamed.status(), StatusCode::OK);
        let admin_update = read_json(renamed).await;
        assert_eq!(admin_update["role"], "admin");
        assert_eq!(admin_update["slug"], updated["slug"]);
        assert_eq!(admin_update["logo_version"], profile["logo_version"]);

        let removed = send_request(
            &app,
            logo_request(&viewer_token, json!({ "action": "remove" })),
        )
        .await;
        assert_eq!(removed.status(), StatusCode::OK);
        assert!(read_json(removed).await["logo_version"].is_null());
        let missing = send_request(&app, read_logo(&owner_token)).await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);

        let persisted =
            sqlx::query_scalar::<_, String>("SELECT name FROM developer_workspaces WHERE id = $1")
                .bind(ws_a_id)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(persisted, "Studio and Labs");
    }
}
