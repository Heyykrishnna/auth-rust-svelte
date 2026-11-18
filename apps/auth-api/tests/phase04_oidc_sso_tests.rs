use auth_api::models::EnrollmentPolicy;
use auth_api::services::oidc_provider::{
    evaluate_sso_policy, verify_pkce_s256, SsoPolicyDecision, SsoPolicyInput,
};
use auth_api::services::social_oauth::generate_pkce_s256_pair;
use chrono::{Duration, Utc};

#[test]
fn pkce_s256_validation_and_malformed_rejection() {
    let (verifier, challenge) = generate_pkce_s256_pair();
    assert!(verify_pkce_s256(&verifier, &challenge));
    assert!(!verify_pkce_s256(
        "wrong_verifier_that_is_still_at_least_43_chars_long_1234",
        &challenge
    ));
    assert!(!verify_pkce_s256("too_short", &challenge));
    assert!(!verify_pkce_s256(
        "invalid chars !@#$%^&*()_+{}|:\"<>?1234567890123456",
        &challenge
    ));
}

#[test]
fn sso_policy_engine_decision_matrix() {
    let now = Utc::now();
    let base = SsoPolicyInput {
        has_active_sso_session: true,
        sso_authenticated_at: Some(now - Duration::seconds(30)),
        sso_amr: vec!["pwd".to_string()],
        user_active: true,
        user_email_verified: true,
        existing_membership_status: None,
        enrollment_policy: EnrollmentPolicy::AutoJoin,
        require_email_verification: true,
        require_mfa: false,
        prompt: None,
        max_age: None,
        now,
    };

    assert_eq!(
        evaluate_sso_policy(&base),
        SsoPolicyDecision::AutoJoinAndIssueCode
    );

    let mut no_sso = base.clone();
    no_sso.has_active_sso_session = false;
    assert_eq!(
        evaluate_sso_policy(&no_sso),
        SsoPolicyDecision::RequireLogin
    );

    let mut prompt_none_no_sso = no_sso.clone();
    prompt_none_no_sso.prompt = Some("none".to_string());
    assert_eq!(
        evaluate_sso_policy(&prompt_none_no_sso),
        SsoPolicyDecision::ErrorRedirect {
            error: "login_required",
            description: "No active IdP SSO session",
        }
    );

    let mut prompt_login = base.clone();
    prompt_login.prompt = Some("login".to_string());
    assert_eq!(
        evaluate_sso_policy(&prompt_login),
        SsoPolicyDecision::RequireLogin
    );

    let mut max_age_zero = base.clone();
    max_age_zero.max_age = Some(0);
    assert_eq!(
        evaluate_sso_policy(&max_age_zero),
        SsoPolicyDecision::RequireLogin
    );

    let mut consent_req = base.clone();
    consent_req.enrollment_policy = EnrollmentPolicy::ConsentRequired;
    assert_eq!(
        evaluate_sso_policy(&consent_req),
        SsoPolicyDecision::RequireConsent
    );

    let mut consent_prompt_none = consent_req.clone();
    consent_prompt_none.prompt = Some("none".to_string());
    assert_eq!(
        evaluate_sso_policy(&consent_prompt_none),
        SsoPolicyDecision::ErrorRedirect {
            error: "consent_required",
            description: "User consent is required to join this application",
        }
    );

    let mut invite_only = base.clone();
    invite_only.enrollment_policy = EnrollmentPolicy::InviteOnly;
    assert_eq!(
        evaluate_sso_policy(&invite_only),
        SsoPolicyDecision::ErrorRedirect {
            error: "access_denied",
            description: "An invitation is required to join this application",
        }
    );

    let mut suspended = base.clone();
    suspended.existing_membership_status = Some("suspended".to_string());
    assert_eq!(
        evaluate_sso_policy(&suspended),
        SsoPolicyDecision::ErrorRedirect {
            error: "access_denied",
            description: "Application membership is suspended or revoked",
        }
    );

    let mut active_member = base.clone();
    active_member.existing_membership_status = Some("active".to_string());
    assert_eq!(
        evaluate_sso_policy(&active_member),
        SsoPolicyDecision::IssueCode
    );
}

#[cfg(test)]
mod live_postgres_phase04_tests {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::Arc;

    use auth_api::config::AppConfig;
    use auth_api::models::EnvironmentId;
    use auth_api::routes::build_router;
    use auth_api::services::client_auth;
    use auth_api::services::signing_keys::EcPublicJwk;
    use auth_api::services::social_oauth::generate_pkce_s256_pair;
    use auth_api::services::tokens::{
        verify_es256_access_token_with_jwk, verify_es256_id_token_with_jwk,
    };
    use auth_api::AppState;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::header::{AUTHORIZATION, CONTENT_TYPE, COOKIE, LOCATION};
    use axum::http::{Method, Request, StatusCode};
    use axum::Router;
    use deadpool_redis::{Config as RedisConfig, Runtime as RedisRuntime};
    use serde_json::{json, Value};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;
    use url::Url;
    use uuid::Uuid;

    static IP_COUNTER: AtomicU16 = AtomicU16::new(1);

    async fn build_live_test_env() -> (Router, AppState) {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for live Phase 04 tests");
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
            jwt_secret: "phase04_integration_test_jwt_secret_at_least_32_chars".to_string(),
            issuer_url: "https://auth.phase04.test".to_string(),
            signing_key_kek: "phase04_live_test_kek_material_at_least_32_characters_01".to_string(),
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
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 4, octet], 43000))));
        app.clone().oneshot(req).await.expect("send request")
    }

    async fn read_json(res: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&bytes).expect("parse json")
    }

    fn parse_location_query(res: &axum::response::Response) -> (String, HashMap<String, String>) {
        let loc = res
            .headers()
            .get(LOCATION)
            .and_then(|v| v.to_str().ok())
            .expect("Location header present")
            .to_string();
        let parsed = Url::parse(&loc).expect("valid redirect URL");
        let params: HashMap<String, String> = parsed
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        (loc, params)
    }

    fn extract_cookie_pair(res: &axum::response::Response, name: &str) -> Option<String> {
        res.headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find_map(|raw| {
                let first = raw.split(';').next()?.trim();
                if first.starts_with(&format!("{name}=")) {
                    Some(first.to_string())
                } else {
                    None
                }
            })
    }

    struct OidcApp {
        env_id: EnvironmentId,
        publishable_key: String,
        redirect_uri: String,
    }

    async fn provision_oidc_apps(
        app: &Router,
        state: &AppState,
        ferrum_policy: &str,
        elysiar_policy: &str,
    ) -> (OidcApp, OidcApp) {
        let dev_email = format!("dev-{}@phase04.test", Uuid::new_v4().simple());
        let sign_up = Request::builder()
            .method(Method::POST)
            .uri("/v1/control/auth/sign-up")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "email": dev_email,
                    "password": "StrongDevPassword123!",
                    "display_name": "Phase 04 Architect",
                    "workspace_name": "Phase 04 Workspace"
                })
                .to_string(),
            ))
            .unwrap();
        let res = send_request(app, sign_up).await;
        assert_eq!(res.status(), StatusCode::CREATED);

        let dev_cookie = extract_cookie_pair(&res, "dev_session_token").unwrap();
        let csrf_pair = extract_cookie_pair(&res, "csrf_token").unwrap();
        let csrf_val = csrf_pair.split_once('=').unwrap().1.to_string();
        let cookie_header = format!("{dev_cookie}; {csrf_pair}");

        let ws_res = send_request(
            app,
            Request::builder()
                .method(Method::GET)
                .uri("/v1/control/workspaces")
                .header(COOKIE, &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let ws_id = read_json(ws_res).await[0]["id"]
            .as_str()
            .unwrap()
            .to_string();

        let make_app = |name: &str, slug_prefix: &str, redirect: &str, policy: &str| {
            let ws = ws_id.clone();
            let ck = cookie_header.clone();
            let cs = csrf_val.clone();
            let router = app.clone();
            let db = state.db.clone();
            let app_name = name.to_string();
            let slug = format!(
                "{}-{}",
                slug_prefix,
                &Uuid::new_v4().simple().to_string()[..8]
            );
            let redir = redirect.to_string();
            let pol = policy.to_string();
            async move {
                let res = send_request(
                    &router,
                    Request::builder()
                        .method(Method::POST)
                        .uri("/v1/control/applications")
                        .header(COOKIE, &ck)
                        .header("x-csrf-token", &cs)
                        .header("content-type", "application/json")
                        .body(Body::from(
                            json!({
                                "workspace_id": ws,
                                "name": app_name,
                                "slug": slug,
                                "initial_dev_redirect_uri": redir
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await;
                assert_eq!(res.status(), StatusCode::CREATED);
                let body = read_json(res).await;
                let dev_env = body["environments"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|e| e["kind"] == "development")
                    .unwrap();
                let env_id: EnvironmentId = dev_env["id"].as_str().unwrap().parse().unwrap();
                let pk = body["development_publishable_key"]
                    .as_str()
                    .unwrap()
                    .to_string();

                sqlx::query(
                    "UPDATE application_environments SET enrollment_policy = $2 WHERE id = $1",
                )
                .bind(env_id)
                .bind(&pol)
                .execute(&db)
                .await
                .expect("set enrollment policy");

                OidcApp {
                    env_id,
                    publishable_key: pk,
                    redirect_uri: redir,
                }
            }
        };

        let ferrum = make_app(
            "Ferrum",
            "ferrum",
            "http://localhost:4101/auth/callback",
            ferrum_policy,
        )
        .await;
        let elysiar = make_app(
            "Elysiar",
            "elysiar",
            "http://localhost:4102/auth/callback",
            elysiar_policy,
        )
        .await;
        (ferrum, elysiar)
    }

    async fn create_global_user_on_ferrum(
        state: &AppState,
        ferrum: &OidcApp,
        email: &str,
        password: &str,
    ) -> Value {
        let (_, otp) = client_auth::initiate_sign_up(
            &state.db,
            &state.config,
            ferrum.env_id,
            email,
            password,
            "Alice SSO",
        )
        .await
        .expect("initiate sign up");
        let auth = client_auth::verify_sign_up_otp(
            &state.db,
            &state.config,
            ferrum.env_id,
            &ferrum.publishable_key,
            email,
            &otp,
            None,
            None,
        )
        .await
        .expect("verify sign up");
        serde_json::to_value(auth).unwrap()
    }

    async fn exchange_code(
        app: &Router,
        client_id: &str,
        code: &str,
        redirect_uri: &str,
        verifier: &str,
    ) -> axum::response::Response {
        let form = format!(
            "grant_type=authorization_code&client_id={}&code={}&redirect_uri={}&code_verifier={}",
            urlencoding::encode(client_id),
            urlencoding::encode(code),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(verifier),
        );
        send_request(
            app,
            Request::builder()
                .method(Method::POST)
                .uri("/oauth2/token")
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap(),
        )
        .await
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn oidc_discovery_and_dual_app_sso_ferrum_to_elysiar_no_second_password() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, elysiar) = provision_oidc_apps(&app, &state, "auto_join", "auto_join").await;

        let disc_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri("/.well-known/openid-configuration")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(disc_res.status(), StatusCode::OK);
        let disc = read_json(disc_res).await;
        assert_eq!(disc["issuer"], "https://auth.phase04.test");
        assert_eq!(
            disc["jwks_uri"],
            "https://auth.phase04.test/.well-known/jwks.json"
        );
        assert_eq!(disc["code_challenge_methods_supported"], json!(["S256"]));

        let alice_email = format!("alice-sso-{}@example.com", Uuid::new_v4().simple());
        let alice_pass = "AliceSsoPass123!";
        let _ = create_global_user_on_ferrum(&state, &ferrum, &alice_email, alice_pass).await;

        let (v1, c1) = generate_pkce_s256_pair();
        let ferrum_authz_uri = format!(
            "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state=S1&nonce=N1&code_challenge={}&code_challenge_method=S256",
            ferrum.env_id,
            urlencoding::encode(&ferrum.redirect_uri),
            urlencoding::encode("openid profile email offline_access"),
            urlencoding::encode(&c1)
        );
        let authz1_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(&ferrum_authz_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(authz1_res.status(), StatusCode::FOUND);
        let (loc1, q1) = parse_location_query(&authz1_res);
        assert!(loc1.contains("/portal/login"));
        let tx_ferrum = q1.get("tx").expect("tx param").clone();

        let tx_ctx_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!("/oauth2/portal/transactions/{tx_ferrum}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(tx_ctx_res.status(), StatusCode::OK);
        let tx_ctx = read_json(tx_ctx_res).await;
        assert_eq!(tx_ctx["application_name"], "Ferrum");
        let portal_csrf = tx_ctx["csrf_token"].as_str().unwrap().to_string();

        let login_res = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri(format!("/oauth2/portal/transactions/{tx_ferrum}/login"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "csrf_token": portal_csrf,
                        "email": alice_email,
                        "password": alice_pass
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(login_res.status(), StatusCode::OK);
        let sso_cookie =
            extract_cookie_pair(&login_res, "idp_sso_session").expect("SSO cookie must be set");
        let login_body = read_json(login_res).await;
        assert_eq!(login_body["action"], "redirect");
        let ferrum_cb_url = Url::parse(login_body["redirect_url"].as_str().unwrap()).unwrap();
        let ferrum_params: HashMap<String, String> =
            ferrum_cb_url.query_pairs().into_owned().collect();
        assert_eq!(ferrum_params.get("state").map(String::as_str), Some("S1"));
        let ferrum_code = ferrum_params.get("code").expect("ferrum code");

        let ferrum_tok_res = exchange_code(
            &app,
            &ferrum.env_id.to_string(),
            ferrum_code,
            &ferrum.redirect_uri,
            &v1,
        )
        .await;
        assert_eq!(ferrum_tok_res.status(), StatusCode::OK);
        let ferrum_tokens = read_json(ferrum_tok_res).await;
        let ferrum_at = ferrum_tokens["access_token"].as_str().unwrap().to_string();
        let ferrum_id = ferrum_tokens["id_token"].as_str().unwrap().to_string();
        assert!(ferrum_tokens["refresh_token"].as_str().is_some());

        let jwks_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri("/.well-known/jwks.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let jwks_json = read_json(jwks_res).await;
        let jwk: EcPublicJwk =
            serde_json::from_value(jwks_json["keys"][0].clone()).expect("parse jwk");

        let ferrum_at_claims = verify_es256_access_token_with_jwk(
            &ferrum_at,
            &jwk,
            "https://auth.phase04.test",
            &ferrum.env_id.to_string(),
        )
        .expect("ferrum access token verifies with jwks");
        let ferrum_id_claims = verify_es256_id_token_with_jwk(
            &ferrum_id,
            &jwk,
            "https://auth.phase04.test",
            &ferrum.env_id.to_string(),
        )
        .expect("ferrum id token verifies with jwks");
        assert_eq!(ferrum_id_claims.nonce.as_deref(), Some("N1"));

        let (v2, c2) = generate_pkce_s256_pair();
        let elysiar_authz_uri = format!(
            "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state=S2&nonce=N2&code_challenge={}&code_challenge_method=S256",
            elysiar.env_id,
            urlencoding::encode(&elysiar.redirect_uri),
            urlencoding::encode("openid profile email offline_access"),
            urlencoding::encode(&c2)
        );
        let authz2_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(&elysiar_authz_uri)
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(authz2_res.status(), StatusCode::FOUND);
        let (loc2, q2) = parse_location_query(&authz2_res);
        assert!(
            loc2.starts_with(&elysiar.redirect_uri),
            "Silent SSO must skip password prompt and redirect directly to Elysiar"
        );
        assert_eq!(q2.get("state").map(String::as_str), Some("S2"));
        let elysiar_code = q2.get("code").expect("elysiar code");

        let elysiar_tok_res = exchange_code(
            &app,
            &elysiar.env_id.to_string(),
            elysiar_code,
            &elysiar.redirect_uri,
            &v2,
        )
        .await;
        assert_eq!(elysiar_tok_res.status(), StatusCode::OK);
        let elysiar_tokens = read_json(elysiar_tok_res).await;
        let elysiar_at = elysiar_tokens["access_token"].as_str().unwrap().to_string();
        let elysiar_id = elysiar_tokens["id_token"].as_str().unwrap().to_string();

        let elysiar_at_claims = verify_es256_access_token_with_jwk(
            &elysiar_at,
            &jwk,
            "https://auth.phase04.test",
            &elysiar.env_id.to_string(),
        )
        .expect("elysiar access token verifies with jwks");
        let elysiar_id_claims = verify_es256_id_token_with_jwk(
            &elysiar_id,
            &jwk,
            "https://auth.phase04.test",
            &elysiar.env_id.to_string(),
        )
        .expect("elysiar id token verifies with jwks");
        assert_eq!(elysiar_id_claims.nonce.as_deref(), Some("N2"));
        assert_eq!(ferrum_at_claims.sub, elysiar_at_claims.sub);
        assert_ne!(ferrum_at_claims.sid, elysiar_at_claims.sid);
        assert_ne!(ferrum_at_claims.app_user_id, elysiar_at_claims.app_user_id);

        let userinfo_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri("/oauth2/userinfo")
                .header(AUTHORIZATION, format!("Bearer {elysiar_at}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(userinfo_res.status(), StatusCode::OK);
        let userinfo = read_json(userinfo_res).await;
        assert_eq!(userinfo["email"], alice_email);
        assert_eq!(userinfo["app_user_id"], elysiar_at_claims.app_user_id);

        let ferrum_on_elysiar = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri("/v1/client/users/me")
                .header("x-publishable-key", &elysiar.publishable_key)
                .header(AUTHORIZATION, format!("Bearer {ferrum_at}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(ferrum_on_elysiar.status(), StatusCode::UNAUTHORIZED);

        let elysiar_on_ferrum = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri("/v1/client/users/me")
                .header("x-publishable-key", &ferrum.publishable_key)
                .header(AUTHORIZATION, format!("Bearer {elysiar_at}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(elysiar_on_ferrum.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn oidc_policy_engine_consent_prompt_max_age_invite_only_and_suspension() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, elysiar) =
            provision_oidc_apps(&app, &state, "auto_join", "consent_required").await;

        let email = format!("policy-{}@example.com", Uuid::new_v4().simple());
        let pass = "PolicyPass123!";
        let _ = create_global_user_on_ferrum(&state, &ferrum, &email, pass).await;

        let (_, c0) = generate_pkce_s256_pair();
        let prompt_none_no_cookie = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid&state=PN1&prompt=none&code_challenge={}&code_challenge_method=S256",
                    ferrum.env_id,
                    urlencoding::encode(&ferrum.redirect_uri),
                    urlencoding::encode(&c0)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(prompt_none_no_cookie.status(), StatusCode::FOUND);
        let (_, q_pn1) = parse_location_query(&prompt_none_no_cookie);
        assert_eq!(
            q_pn1.get("error").map(String::as_str),
            Some("login_required")
        );

        let (_, c1) = generate_pkce_s256_pair();
        let authz_f = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid&state=F1&code_challenge={}&code_challenge_method=S256",
                    ferrum.env_id,
                    urlencoding::encode(&ferrum.redirect_uri),
                    urlencoding::encode(&c1)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (_, qf) = parse_location_query(&authz_f);
        let tx_f = qf["tx"].clone();
        let ctx_f = read_json(
            send_request(
                &app,
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/oauth2/portal/transactions/{tx_f}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await,
        )
        .await;
        let login_res = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri(format!("/oauth2/portal/transactions/{tx_f}/login"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "csrf_token": ctx_f["csrf_token"],
                        "email": email,
                        "password": pass
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        let sso_cookie = extract_cookie_pair(&login_res, "idp_sso_session").unwrap();

        let prompt_login_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid&prompt=login&code_challenge={}&code_challenge_method=S256",
                    ferrum.env_id,
                    urlencoding::encode(&ferrum.redirect_uri),
                    urlencoding::encode(&c1)
                ))
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (loc_pl, _) = parse_location_query(&prompt_login_res);
        assert!(loc_pl.contains("/portal/login"));

        let max_age_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid&max_age=0&code_challenge={}&code_challenge_method=S256",
                    ferrum.env_id,
                    urlencoding::encode(&ferrum.redirect_uri),
                    urlencoding::encode(&c1)
                ))
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (loc_ma, _) = parse_location_query(&max_age_res);
        assert!(loc_ma.contains("/portal/login"));

        let pn_consent_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid&prompt=none&code_challenge={}&code_challenge_method=S256",
                    elysiar.env_id,
                    urlencoding::encode(&elysiar.redirect_uri),
                    urlencoding::encode(&c1)
                ))
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (_, q_pnc) = parse_location_query(&pn_consent_res);
        assert_eq!(
            q_pnc.get("error").map(String::as_str),
            Some("consent_required")
        );

        let (ve, ce) = generate_pkce_s256_pair();
        let consent_route_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email&state=EC1&code_challenge={}&code_challenge_method=S256",
                    elysiar.env_id,
                    urlencoding::encode(&elysiar.redirect_uri),
                    urlencoding::encode(&ce)
                ))
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (loc_c, qc) = parse_location_query(&consent_route_res);
        assert!(loc_c.contains("/portal/consent"));
        let tx_c = qc["tx"].clone();
        let ctx_c = read_json(
            send_request(
                &app,
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/oauth2/portal/transactions/{tx_c}"))
                    .header(COOKIE, &sso_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await,
        )
        .await;
        let approve_res = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri(format!("/oauth2/portal/transactions/{tx_c}/consent"))
                .header(COOKIE, &sso_cookie)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "csrf_token": ctx_c["csrf_token"], "approved": true }).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(approve_res.status(), StatusCode::OK);
        let approve_body = read_json(approve_res).await;
        let cb_url = Url::parse(approve_body["redirect_url"].as_str().unwrap()).unwrap();
        let cb_params: HashMap<String, String> = cb_url.query_pairs().into_owned().collect();
        let code_e = cb_params["code"].clone();

        let tok_e = exchange_code(
            &app,
            &elysiar.env_id.to_string(),
            &code_e,
            &elysiar.redirect_uri,
            &ve,
        )
        .await;
        assert_eq!(tok_e.status(), StatusCode::OK);

        sqlx::query("UPDATE application_users SET status = 'suspended' WHERE environment_id = $1")
            .bind(elysiar.env_id)
            .execute(&state.db)
            .await
            .unwrap();

        let susp_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid&code_challenge={}&code_challenge_method=S256",
                    elysiar.env_id,
                    urlencoding::encode(&elysiar.redirect_uri),
                    urlencoding::encode(&ce)
                ))
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (_, q_susp) = parse_location_query(&susp_res);
        assert_eq!(
            q_susp.get("error").map(String::as_str),
            Some("access_denied")
        );

        let ferrum_still_ok = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid&code_challenge={}&code_challenge_method=S256",
                    ferrum.env_id,
                    urlencoding::encode(&ferrum.redirect_uri),
                    urlencoding::encode(&ce)
                ))
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (_, q_fok) = parse_location_query(&ferrum_still_ok);
        assert!(q_fok.contains_key("code"));

        sqlx::query("DELETE FROM application_users WHERE environment_id = $1")
            .bind(elysiar.env_id)
            .execute(&state.db)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE application_environments SET enrollment_policy = 'invite_only' WHERE id = $1",
        )
        .bind(elysiar.env_id)
        .execute(&state.db)
        .await
        .unwrap();

        let invite_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid&code_challenge={}&code_challenge_method=S256",
                    elysiar.env_id,
                    urlencoding::encode(&elysiar.redirect_uri),
                    urlencoding::encode(&ce)
                ))
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (_, q_inv) = parse_location_query(&invite_res);
        assert_eq!(
            q_inv.get("error").map(String::as_str),
            Some("access_denied")
        );
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn oidc_attack_regressions_code_replay_pkce_mismatch_and_open_redirect() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, _) = provision_oidc_apps(&app, &state, "auto_join", "auto_join").await;

        let (v1, c1) = generate_pkce_s256_pair();
        let open_redirect_probe = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid&code_challenge={}&code_challenge_method=S256",
                    ferrum.env_id,
                    urlencoding::encode("https://evil.attacker.example/steal"),
                    urlencoding::encode(&c1)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(
            open_redirect_probe.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert!(open_redirect_probe.headers().get(LOCATION).is_none());

        let plain_pkce_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid&code_challenge={}&code_challenge_method=plain",
                    ferrum.env_id,
                    urlencoding::encode(&ferrum.redirect_uri),
                    urlencoding::encode(&c1)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(plain_pkce_res.status(), StatusCode::FOUND);
        let (_, q_plain) = parse_location_query(&plain_pkce_res);
        assert_eq!(
            q_plain.get("error").map(String::as_str),
            Some("invalid_request")
        );

        let email = format!("replay-{}@example.com", Uuid::new_v4().simple());
        let pass = "ReplayPass123!";
        let _ = create_global_user_on_ferrum(&state, &ferrum, &email, pass).await;

        let authz_res = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20offline_access&code_challenge={}&code_challenge_method=S256",
                    ferrum.env_id,
                    urlencoding::encode(&ferrum.redirect_uri),
                    urlencoding::encode(&c1)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (_, qa) = parse_location_query(&authz_res);
        let tx_id = qa["tx"].clone();
        let ctx = read_json(
            send_request(
                &app,
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/oauth2/portal/transactions/{tx_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await,
        )
        .await;
        let login_res = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri(format!("/oauth2/portal/transactions/{tx_id}/login"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "csrf_token": ctx["csrf_token"], "email": email, "password": pass })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await;
        let login_body = read_json(login_res).await;
        let cb_url = Url::parse(login_body["redirect_url"].as_str().unwrap()).unwrap();
        let cb_params: HashMap<String, String> = cb_url.query_pairs().into_owned().collect();
        let code = cb_params["code"].clone();

        let wrong_pkce = exchange_code(
            &app,
            &ferrum.env_id.to_string(),
            &code,
            &ferrum.redirect_uri,
            "wrong_verifier_abcdefghijklmnopqrstuvwxyz_0123456789",
        )
        .await;
        assert_eq!(wrong_pkce.status(), StatusCode::UNAUTHORIZED);

        let wrong_redir = exchange_code(
            &app,
            &ferrum.env_id.to_string(),
            &code,
            "http://localhost:4101/wrong-path",
            &v1,
        )
        .await;
        assert_eq!(wrong_redir.status(), StatusCode::UNAUTHORIZED);

        let good_exchange = exchange_code(
            &app,
            &ferrum.env_id.to_string(),
            &code,
            &ferrum.redirect_uri,
            &v1,
        )
        .await;
        assert_eq!(good_exchange.status(), StatusCode::OK);
        let tokens = read_json(good_exchange).await;
        let issued_rt = tokens["refresh_token"].as_str().unwrap().to_string();

        let replay_res = exchange_code(
            &app,
            &ferrum.env_id.to_string(),
            &code,
            &ferrum.redirect_uri,
            &v1,
        )
        .await;
        assert_eq!(replay_res.status(), StatusCode::UNAUTHORIZED);

        let rt_after_replay = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/v1/client/auth/refresh")
                .header("x-publishable-key", &ferrum.publishable_key)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refresh_token": issued_rt }).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(rt_after_replay.status(), StatusCode::UNAUTHORIZED);

        let replay_audits: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs WHERE environment_id = $1 AND event = 'OIDC_CODE_REPLAY_DETECTED'",
        )
        .bind(ferrum.env_id)
        .fetch_one(&state.db)
        .await
        .unwrap();
        assert!(replay_audits >= 1);
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn oidc_local_vs_global_logout_semantics() {
        let (app, state) = build_live_test_env().await;
        let (ferrum, elysiar) = provision_oidc_apps(&app, &state, "auto_join", "auto_join").await;

        let email = format!("logout-{}@example.com", Uuid::new_v4().simple());
        let pass = "LogoutPass123!";
        let _ = create_global_user_on_ferrum(&state, &ferrum, &email, pass).await;

        let (v1, c1) = generate_pkce_s256_pair();
        let authz_f = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20offline_access&code_challenge={}&code_challenge_method=S256",
                    ferrum.env_id,
                    urlencoding::encode(&ferrum.redirect_uri),
                    urlencoding::encode(&c1)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (_, qf) = parse_location_query(&authz_f);
        let tx_f = qf["tx"].clone();
        let ctx_f = read_json(
            send_request(
                &app,
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/oauth2/portal/transactions/{tx_f}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await,
        )
        .await;
        let login_res = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri(format!("/oauth2/portal/transactions/{tx_f}/login"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "csrf_token": ctx_f["csrf_token"], "email": email, "password": pass })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await;
        let sso_cookie = extract_cookie_pair(&login_res, "idp_sso_session").unwrap();
        let login_body = read_json(login_res).await;
        let ferrum_code = Url::parse(login_body["redirect_url"].as_str().unwrap())
            .unwrap()
            .query_pairs()
            .find(|(k, _)| k == "code")
            .unwrap()
            .1
            .into_owned();
        let ferrum_toks = read_json(
            exchange_code(
                &app,
                &ferrum.env_id.to_string(),
                &ferrum_code,
                &ferrum.redirect_uri,
                &v1,
            )
            .await,
        )
        .await;
        let ferrum_rt = ferrum_toks["refresh_token"].as_str().unwrap().to_string();
        let ferrum_id_tok = ferrum_toks["id_token"].as_str().unwrap().to_string();

        let (v2, c2) = generate_pkce_s256_pair();
        let authz_e = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20offline_access&code_challenge={}&code_challenge_method=S256",
                    elysiar.env_id,
                    urlencoding::encode(&elysiar.redirect_uri),
                    urlencoding::encode(&c2)
                ))
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let (_, qe) = parse_location_query(&authz_e);
        let elysiar_toks = read_json(
            exchange_code(
                &app,
                &elysiar.env_id.to_string(),
                &qe["code"],
                &elysiar.redirect_uri,
                &v2,
            )
            .await,
        )
        .await;
        let elysiar_rt = elysiar_toks["refresh_token"].as_str().unwrap().to_string();
        let elysiar_id_tok = elysiar_toks["id_token"].as_str().unwrap().to_string();

        let local_logout = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/logout?id_token_hint={}&post_logout_redirect_uri={}&global=false",
                    urlencoding::encode(&ferrum_id_tok),
                    urlencoding::encode(&ferrum.redirect_uri)
                ))
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(local_logout.status(), StatusCode::FOUND);

        let ferrum_rt_check = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/v1/client/auth/refresh")
                .header("x-publishable-key", &ferrum.publishable_key)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refresh_token": ferrum_rt }).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(ferrum_rt_check.status(), StatusCode::UNAUTHORIZED);

        let elysiar_rt_check = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/v1/client/auth/refresh")
                .header("x-publishable-key", &elysiar.publishable_key)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refresh_token": elysiar_rt }).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(elysiar_rt_check.status(), StatusCode::OK);
        let elysiar_rt_rotated = read_json(elysiar_rt_check).await["refresh_token"]
            .as_str()
            .unwrap()
            .to_string();

        let account_still_alive = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri("/oauth2/portal/account")
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(account_still_alive.status(), StatusCode::OK);

        let global_logout = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/oauth2/logout?id_token_hint={}&post_logout_redirect_uri={}&global=true",
                    urlencoding::encode(&elysiar_id_tok),
                    urlencoding::encode(&elysiar.redirect_uri)
                ))
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(global_logout.status(), StatusCode::FOUND);
        assert!(extract_cookie_pair(&global_logout, "idp_sso_session").is_some());

        let account_dead = send_request(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri("/oauth2/portal/account")
                .header(COOKIE, &sso_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(account_dead.status(), StatusCode::UNAUTHORIZED);

        let elysiar_rt_dead = send_request(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/v1/client/auth/refresh")
                .header("x-publishable-key", &elysiar.publishable_key)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refresh_token": elysiar_rt_rotated }).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(elysiar_rt_dead.status(), StatusCode::UNAUTHORIZED);
    }
}
