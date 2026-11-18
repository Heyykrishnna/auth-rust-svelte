use auth_api::middleware::{SecretKeyAuth, TenantContext, TenantSource};
use auth_api::models::{
    AppSessionId, AppUserId, ApplicationId, EnvironmentId, EnvironmentKind, OrganizationId, UserId,
};
use auth_api::services::credentials::{
    generate_publishable_key, generate_secret_key, key_matches_environment_kind,
    validate_secret_scopes,
};
use auth_api::services::organizations::evaluate_scoped_permission;
use auth_api::services::signing_keys::generate_es256_keypair;
use auth_api::services::tokens::{
    issue_es256_access_token_with_key, issue_es256_id_token_with_key,
    verify_es256_access_token_with_jwk, verify_es256_id_token_with_jwk, IdTokenClaims,
    IssueAccessTokenInput,
};
use chrono::Utc;
use uuid::Uuid;

#[test]
fn test_cross_environment_credential_and_scope_escalation_isolation() {
    let (_, ferrum_dev_pk) = generate_publishable_key(EnvironmentKind::Development);
    let (_, ferrum_prod_pk) = generate_publishable_key(EnvironmentKind::Production);
    let ferrum_dev_sk = generate_secret_key(EnvironmentKind::Development);
    let ferrum_prod_sk = generate_secret_key(EnvironmentKind::Production);

    assert!(key_matches_environment_kind(
        &ferrum_dev_pk,
        EnvironmentKind::Development
    ));
    assert!(!key_matches_environment_kind(
        &ferrum_dev_pk,
        EnvironmentKind::Production
    ));
    assert!(key_matches_environment_kind(
        &ferrum_prod_pk,
        EnvironmentKind::Production
    ));
    assert!(!key_matches_environment_kind(
        &ferrum_prod_pk,
        EnvironmentKind::Development
    ));

    assert!(key_matches_environment_kind(
        &ferrum_dev_sk.plaintext,
        EnvironmentKind::Development
    ));
    assert!(!key_matches_environment_kind(
        &ferrum_dev_sk.plaintext,
        EnvironmentKind::Production
    ));
    assert!(key_matches_environment_kind(
        &ferrum_prod_sk.plaintext,
        EnvironmentKind::Production
    ));
    assert!(!key_matches_environment_kind(
        &ferrum_prod_sk.plaintext,
        EnvironmentKind::Development
    ));

    let readonly_scopes = vec!["users:read".to_string(), "sync:read".to_string()];
    let readonly_ctx = TenantContext {
        environment_id: EnvironmentId::new(),
        application_id: ApplicationId::new(),
        environment_kind: EnvironmentKind::Production,
        source: TenantSource::SecretKey {
            scopes: readonly_scopes.clone(),
        },
    };
    let secret_auth = SecretKeyAuth {
        tenant: readonly_ctx.clone(),
        credential_id: Uuid::new_v4(),
        scopes: readonly_scopes,
    };

    assert!(readonly_ctx.has_scope("users:read"));
    assert!(readonly_ctx.has_scope("sync:read"));
    assert!(!readonly_ctx.has_scope("users:write"));
    assert!(!readonly_ctx.has_scope("sessions:revoke"));
    assert!(!readonly_ctx.has_scope("orgs:manage"));
    assert!(secret_auth.require_scope("users:read").is_ok());
    assert!(secret_auth.require_scope("users:write").is_err());
    assert!(secret_auth.require_scope("sessions:revoke").is_err());
    assert!(secret_auth.require_scope("orgs:manage").is_err());

    assert!(validate_secret_scopes(&[]).is_err());
    assert!(validate_secret_scopes(&["admin:super".to_string()]).is_err());
    assert!(validate_secret_scopes(&["users:read".to_string(), "orgs:read".to_string()]).is_ok());
}

#[test]
fn test_cross_application_jwt_aud_and_org_permission_isolation() {
    let keypair = generate_es256_keypair().unwrap();
    let issuer = "https://auth.dradix.test";

    let ferrum_dev = EnvironmentId::new();
    let ferrum_prod = EnvironmentId::new();
    let elysiar_dev = EnvironmentId::new();
    let elysiar_prod = EnvironmentId::new();

    let user_id = UserId::new();
    let app_user_id = AppUserId::new();
    let session_id = AppSessionId::new();
    let org_ferrum = OrganizationId::new();

    let (ferrum_dev_token, _) = issue_es256_access_token_with_key(
        IssueAccessTokenInput {
            issuer_url: issuer,
            user_id,
            environment_id: ferrum_dev,
            authorized_party: "pk_test_ferrum",
            app_user_id,
            session_id,
            active_organization_id: Some(org_ferrum),
            roles: vec!["member".to_string()],
            permissions: vec!["profile.read".to_string()],
            amr: vec!["pwd".to_string()],
            acr: "aal1".to_string(),
            version: 1,
            ttl_secs: 300,
        },
        &keypair.kid,
        &keypair.pkcs8_der,
    )
    .unwrap();

    assert!(verify_es256_access_token_with_jwk(
        &ferrum_dev_token,
        &keypair.public_jwk,
        issuer,
        &ferrum_dev.to_string(),
    )
    .is_ok());

    for foreign_env in [ferrum_prod, elysiar_dev, elysiar_prod] {
        assert!(
            verify_es256_access_token_with_jwk(
                &ferrum_dev_token,
                &keypair.public_jwk,
                issuer,
                &foreign_env.to_string(),
            )
            .is_err(),
            "Access token minted for ferrum_dev must not verify for foreign env {foreign_env}"
        );
    }

    let now = Utc::now().timestamp();
    let ferrum_id_claims = IdTokenClaims {
        iss: issuer.to_string(),
        sub: user_id.to_string(),
        aud: "pk_live_ferrum".to_string(),
        nonce: Some("ferrum-nonce".to_string()),
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
    let ferrum_id_token =
        issue_es256_id_token_with_key(&ferrum_id_claims, &keypair.kid, &keypair.pkcs8_der).unwrap();

    assert!(verify_es256_id_token_with_jwk(
        &ferrum_id_token,
        &keypair.public_jwk,
        issuer,
        "pk_live_ferrum",
    )
    .is_ok());
    assert!(verify_es256_id_token_with_jwk(
        &ferrum_id_token,
        &keypair.public_jwk,
        issuer,
        "pk_live_elysiar",
    )
    .is_err());

    let app_perms = vec!["app:profile:read".to_string()];
    let org_perms = vec![
        "org:members:manage".to_string(),
        "org:billing:write".to_string(),
    ];
    assert!(evaluate_scoped_permission(
        &app_perms,
        &org_perms,
        Some("org_ferrum_1"),
        Some("org_ferrum_1"),
        "org:members:manage",
    ));
    assert!(!evaluate_scoped_permission(
        &app_perms,
        &org_perms,
        Some("org_ferrum_1"),
        Some("org_elysiar_1"),
        "org:members:manage",
    ));
    assert!(!evaluate_scoped_permission(
        &app_perms,
        &org_perms,
        None,
        Some("org_ferrum_1"),
        "org:billing:write",
    ));
}

#[cfg(test)]
mod live_phase10_red_team_suite {
    use auth_api::config::AppConfig;
    use auth_api::models::{
        AggregateType, AppSessionId, AppUserId, DeveloperId, DomainEventEnvelope, DomainEventType,
        EnvironmentId, EnvironmentKind, UserId, WorkspaceId, WorkspaceRole,
    };
    use auth_api::repositories::outbox;
    use auth_api::services::analytics::{
        query_environment_analytics, record_environment_api_request, AnalyticsWindow,
    };
    use auth_api::services::backend_sync::{self, UpdateBackendUserMetadataInput};
    use auth_api::services::control_plane;
    use auth_api::services::organizations::{self, CreateInvitationInput, CreateOrganizationInput};
    use auth_api::services::tokens::{
        issue_tenant_access_token, verify_tenant_access_token, IssueAccessTokenInput,
    };
    use auth_api::services::webhooks::{self, CreateWebhookInput};
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    fn red_team_config(db_url: String) -> AppConfig {
        AppConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            database_url: db_url,
            db_max_connections: 5,
            redis_url: "redis://127.0.0.1:6379".to_string(),
            jwt_secret: "red_team_jwt_secret_material_32_bytes_minimum_001".to_string(),
            issuer_url: "https://auth.dradix.test".to_string(),
            signing_key_kek: "red_team_kek_secret_material_32_bytes_minimum_001".to_string(),
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
            cookie_secure: false,
            cookie_domain: None,
            otel_exporter_otlp_endpoint: "http://127.0.0.1:4317".to_string(),
            otel_service_name: "auth-api-red-team".to_string(),
            otel_service_version: "0.1.0".to_string(),
            smtp_host: None,
            smtp_port: 587,
            smtp_user: None,
            smtp_pass: None,
            smtp_from: "Dradix <noreply@dradix.test>".to_string(),
            smtp_from_name: "Dradix".to_string(),
        }
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn test_live_two_workspaces_ferrum_elysiar_four_env_red_team_isolation() {
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
        sqlx::query("DELETE FROM signing_keys")
            .execute(&pool)
            .await
            .expect("reset signing key fixtures");

        let cfg = red_team_config(db_url);
        let suffix = Uuid::new_v4().to_string()[..8].to_string();

        let dev_a_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO developers (email, password_hash, display_name) VALUES ($1, 'hash', 'Dev Ferrum') RETURNING id",
        )
        .bind(format!("rt-dev-a-{suffix}@example.com"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let dev_b_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO developers (email, password_hash, display_name) VALUES ($1, 'hash', 'Dev Elysiar') RETURNING id",
        )
        .bind(format!("rt-dev-b-{suffix}@example.com"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let dev_a = DeveloperId::from(dev_a_uuid);
        let dev_b = DeveloperId::from(dev_b_uuid);

        let ws_a_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO developer_workspaces (name, slug) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("WS Ferrum {suffix}"))
        .bind(format!("ws-ferrum-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let ws_b_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO developer_workspaces (name, slug) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("WS Elysiar {suffix}"))
        .bind(format!("ws-elysiar-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO workspace_members (workspace_id, developer_id, role) VALUES ($1, $2, 'owner')",
        )
        .bind(ws_a_uuid)
        .bind(dev_a_uuid)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO workspace_members (workspace_id, developer_id, role) VALUES ($1, $2, 'owner')",
        )
        .bind(ws_b_uuid)
        .bind(dev_b_uuid)
        .execute(&pool)
        .await
        .unwrap();

        let ferrum = control_plane::create_application(
            &pool,
            dev_a,
            WorkspaceId::from(ws_a_uuid),
            "Ferrum",
            &format!("ferrum-{suffix}"),
            Some("http://localhost:3001/callback"),
        )
        .await
        .unwrap();

        let elysiar = control_plane::create_application(
            &pool,
            dev_b,
            WorkspaceId::from(ws_b_uuid),
            "Elysiar",
            &format!("elysiar-{suffix}"),
            Some("http://localhost:3002/callback"),
        )
        .await
        .unwrap();

        let ferrum_dev_env = ferrum
            .environments
            .iter()
            .find(|e| e.kind == "development")
            .unwrap()
            .id;
        let ferrum_prod_env = ferrum
            .environments
            .iter()
            .find(|e| e.kind == "production")
            .unwrap()
            .id;
        let elysiar_dev_env = elysiar
            .environments
            .iter()
            .find(|e| e.kind == "development")
            .unwrap()
            .id;
        let elysiar_prod_env = elysiar
            .environments
            .iter()
            .find(|e| e.kind == "production")
            .unwrap()
            .id;

        assert!(control_plane::require_application_access(
            &pool,
            dev_b,
            ferrum.application.id,
            WorkspaceRole::Viewer,
        )
        .await
        .is_err());
        assert!(control_plane::require_application_access(
            &pool,
            dev_a,
            elysiar.application.id,
            WorkspaceRole::Viewer,
        )
        .await
        .is_err());

        let alice_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, password_hash, display_name, email_verified) VALUES ($1, 'hash', 'Alice', TRUE) RETURNING id",
        )
        .bind(format!("rt-alice-{suffix}@example.com"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let bob_email = format!("rt-bob-{suffix}@example.com");
        let bob_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, password_hash, display_name, email_verified) VALUES ($1, 'hash', 'Bob', TRUE) RETURNING id",
        )
        .bind(&bob_email)
        .fetch_one(&pool)
        .await
        .unwrap();

        let ferrum_dev_alice: Uuid = sqlx::query_scalar(
            "INSERT INTO application_users (environment_id, user_id, public_metadata, private_metadata) VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(ferrum_dev_env)
        .bind(alice_uuid)
        .bind(serde_json::json!({ "tier": "ferrum_dev_only" }))
        .bind(serde_json::json!({ "secret_note": "ferrum_internal" }))
        .fetch_one(&pool)
        .await
        .unwrap();
        let ferrum_dev_alice_id = AppUserId::from(ferrum_dev_alice);

        let ferrum_dev_session_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO app_sessions (environment_id, application_user_id, user_id, expires_at) VALUES ($1, $2, $3, NOW() + INTERVAL '1 hour') RETURNING id",
        )
        .bind(ferrum_dev_env)
        .bind(ferrum_dev_alice)
        .bind(alice_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();
        let ferrum_dev_session_id = AppSessionId::from(ferrum_dev_session_uuid);

        assert!(
            backend_sync::get_user_by_app_user_id(&pool, ferrum_dev_env, ferrum_dev_alice_id,)
                .await
                .is_ok()
        );

        for foreign_env in [ferrum_prod_env, elysiar_dev_env, elysiar_prod_env] {
            assert!(
                backend_sync::get_user_by_app_user_id(&pool, foreign_env, ferrum_dev_alice_id,)
                    .await
                    .is_err()
            );

            assert!(backend_sync::update_user_metadata(
                &pool,
                foreign_env,
                ferrum_dev_alice_id,
                UpdateBackendUserMetadataInput {
                    external_id: Some(Some("hijacked".to_string())),
                    public_metadata: Some(serde_json::json!({ "pwned": true })),
                    private_metadata: None,
                },
            )
            .await
            .is_err());

            assert!(
                backend_sync::get_session_status(&pool, foreign_env, ferrum_dev_session_id,)
                    .await
                    .is_err()
            );

            assert!(
                backend_sync::revoke_session(&pool, foreign_env, ferrum_dev_session_id,)
                    .await
                    .is_err()
            );
        }

        let (ferrum_jwt, _) = issue_tenant_access_token(
            &pool,
            &cfg,
            IssueAccessTokenInput {
                issuer_url: &cfg.issuer_url,
                user_id: UserId::from(alice_uuid),
                environment_id: ferrum_dev_env,
                authorized_party: &ferrum.development_publishable_key,
                app_user_id: ferrum_dev_alice_id,
                session_id: ferrum_dev_session_id,
                active_organization_id: None,
                roles: vec!["member".to_string()],
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
            verify_tenant_access_token(&pool, &cfg, &ferrum_jwt, ferrum_dev_env)
                .await
                .is_ok()
        );
        for foreign_env in [ferrum_prod_env, elysiar_dev_env, elysiar_prod_env] {
            assert!(
                verify_tenant_access_token(&pool, &cfg, &ferrum_jwt, foreign_env)
                    .await
                    .is_err()
            );
        }

        let ferrum_org = organizations::create_organization(
            &pool,
            ferrum_dev_env,
            Some(ferrum_dev_alice_id),
            "user",
            Some(alice_uuid),
            CreateOrganizationInput {
                name: "Ferrum Core".to_string(),
                slug: format!("ferrum-core-{suffix}"),
                logo_url: None,
                metadata: None,
            },
        )
        .await
        .unwrap();

        let ferrum_inv = organizations::create_invitation(
            &pool,
            ferrum_dev_env,
            ferrum_org.id,
            CreateInvitationInput {
                email: &bob_email,
                role_slug: Some("org:admin"),
                expires_in_days: Some(3),
                invited_by_app_user_id: Some(ferrum_dev_alice_id),
                actor_type: "user",
                actor_id: Some(alice_uuid),
            },
        )
        .await
        .unwrap();

        for foreign_env in [ferrum_prod_env, elysiar_dev_env, elysiar_prod_env] {
            assert!(
                organizations::get_organization(&pool, foreign_env, ferrum_org.id)
                    .await
                    .is_err()
            );
            assert!(organizations::accept_invitation(
                &pool,
                foreign_env,
                UserId::from(bob_uuid),
                &ferrum_inv.invitation_token,
            )
            .await
            .is_err());
        }

        let ferrum_wh = webhooks::create_webhook_endpoint(
            &pool,
            &cfg,
            dev_a,
            ferrum.application.id,
            EnvironmentKind::Development,
            CreateWebhookInput {
                url: "https://hooks.ferrum.test/events".to_string(),
                description: Some("Ferrum Dev Hook".to_string()),
                subscribed_events: vec!["user.updated".to_string()],
            },
        )
        .await
        .unwrap();

        let elysiar_wh = webhooks::create_webhook_endpoint(
            &pool,
            &cfg,
            dev_b,
            elysiar.application.id,
            EnvironmentKind::Development,
            CreateWebhookInput {
                url: "https://hooks.elysiar.test/events".to_string(),
                description: Some("Elysiar Dev Hook".to_string()),
                subscribed_events: vec!["user.updated".to_string()],
            },
        )
        .await
        .unwrap();

        assert!(webhooks::list_webhook_endpoints(
            &pool,
            dev_b,
            ferrum.application.id,
            EnvironmentKind::Development,
        )
        .await
        .is_err());

        let evt = DomainEventEnvelope::new(
            DomainEventType::UserUpdated,
            Some(ferrum_dev_env.into_inner()),
            AggregateType::ApplicationUser,
            ferrum_dev_alice,
            2,
            serde_json::json!({ "app_user_id": ferrum_dev_alice }),
        );
        let inserted_event_id = outbox::insert_outbox_event(&pool, ferrum_dev_env, &evt)
            .await
            .unwrap();

        let _ = webhooks::dispatch_outbox_to_deliveries(&pool, 50)
            .await
            .unwrap();

        let ferrum_deliveries: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM webhook_deliveries WHERE webhook_endpoint_id = $1 AND outbox_event_id = $2",
        )
        .bind(ferrum_wh.endpoint.id)
        .bind(inserted_event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let elysiar_deliveries: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM webhook_deliveries WHERE webhook_endpoint_id = $1 AND outbox_event_id = $2",
        )
        .bind(elysiar_wh.endpoint.id)
        .bind(inserted_event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(ferrum_deliveries, 1);
        assert_eq!(elysiar_deliveries, 0);

        assert!(backend_sync::list_events_catchup(
            &pool,
            elysiar_dev_env,
            Some(inserted_event_id),
            None,
            Some(25),
        )
        .await
        .is_err());
        assert!(backend_sync::list_events_catchup(
            &pool,
            ferrum_prod_env,
            Some(inserted_event_id),
            None,
            Some(25),
        )
        .await
        .is_err());

        record_environment_api_request(&pool, ferrum_dev_env, "client").await;
        record_environment_api_request(&pool, ferrum_dev_env, "backend").await;

        let ferrum_dev_analytics =
            query_environment_analytics(&pool, ferrum_dev_env, AnalyticsWindow::Days7, true)
                .await
                .unwrap();
        assert_eq!(ferrum_dev_analytics.summary.registrations, 1);
        assert_eq!(ferrum_dev_analytics.summary.client_api_requests, 1);
        assert_eq!(ferrum_dev_analytics.summary.backend_api_requests, 1);

        for foreign_env in [ferrum_prod_env, elysiar_dev_env, elysiar_prod_env] {
            let foreign_analytics =
                query_environment_analytics(&pool, foreign_env, AnalyticsWindow::Days7, true)
                    .await
                    .unwrap();
            assert_eq!(foreign_analytics.summary.registrations, 0);
            assert_eq!(foreign_analytics.summary.client_api_requests, 0);
            assert_eq!(foreign_analytics.summary.backend_api_requests, 0);
        }

        let _ = EnvironmentId::from(ferrum_dev_env.into_inner());
    }
}
