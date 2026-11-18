use auth_api::middleware::{
    tenant::{extract_publishable_key, extract_secret_key_bearer},
    TenantContext, TenantSource, PUBLISHABLE_KEY_HEADER,
};
use auth_api::models::{
    AppSessionId, AppUserId, ApplicationId, AuditEvent, EnvironmentId, EnvironmentKind,
    OrganizationId, SsoSessionId, UserId,
};
use auth_api::repositories::redis_keys;
use axum::http::Request;
use std::str::FromStr;
use uuid::Uuid;

#[test]
fn test_strong_id_newtypes_serde_and_roundtrip() {
    let raw = Uuid::new_v4();
    let env_id = EnvironmentId::from_uuid(raw);
    let app_id = ApplicationId::from_uuid(raw);
    let user_id = UserId::from_uuid(raw);
    let app_user_id = AppUserId::from_uuid(raw);
    let org_id = OrganizationId::from_uuid(raw);
    let app_session_id = AppSessionId::from_uuid(raw);
    let sso_session_id = SsoSessionId::from_uuid(raw);

    assert_eq!(env_id.into_inner(), raw);
    assert_eq!(app_id.into_inner(), raw);
    assert_eq!(user_id.into_inner(), raw);
    assert_eq!(app_user_id.into_inner(), raw);
    assert_eq!(org_id.into_inner(), raw);
    assert_eq!(app_session_id.into_inner(), raw);
    assert_eq!(sso_session_id.into_inner(), raw);

    let json = serde_json::to_string(&env_id).expect("EnvironmentId should serialize");
    assert_eq!(json, format!("\"{raw}\""));
    let parsed: EnvironmentId =
        serde_json::from_str(&json).expect("EnvironmentId should deserialize");
    assert_eq!(parsed, env_id);
    assert_eq!(
        EnvironmentId::from_str(&raw.to_string()).expect("should parse from string"),
        env_id
    );
}

#[test]
fn test_redis_keys_enforce_global_and_environment_namespaces() {
    let env_a = EnvironmentId::new();
    let env_b = EnvironmentId::new();
    let sso_id = SsoSessionId::new();
    let session_id = AppSessionId::new();

    let global_sso = redis_keys::global_sso_session_key(sso_id);
    assert!(global_sso.starts_with("idp:global:sso_session:"));

    let global_login = redis_keys::global_login_attempt_key("alice@example.com");
    assert_eq!(global_login, "idp:global:login_attempt:alice@example.com");

    let env_a_sess = redis_keys::env_app_session_key(env_a, session_id);
    let env_b_sess = redis_keys::env_app_session_key(env_b, session_id);
    assert!(env_a_sess.starts_with(&format!("env:{env_a}:app_session:")));
    assert!(env_b_sess.starts_with(&format!("env:{env_b}:app_session:")));
    assert_ne!(env_a_sess, env_b_sess);

    let env_a_rl = redis_keys::env_rate_limit_key(env_a, "127.0.0.1");
    let env_b_rl = redis_keys::env_rate_limit_key(env_b, "127.0.0.1");
    assert_ne!(env_a_rl, env_b_rl);

    let env_a_cors = redis_keys::env_cors_origins_key(env_a);
    assert_eq!(env_a_cors, format!("env:{env_a}:cors_origins:allowlist"));
}

#[test]
fn test_tenant_context_header_extraction_and_scopes() {
    let pk_req = Request::builder()
        .header(PUBLISHABLE_KEY_HEADER, "pk_test_ferrum_123")
        .body(())
        .unwrap();
    let (parts, _) = pk_req.into_parts();
    assert_eq!(extract_publishable_key(&parts), Some("pk_test_ferrum_123"));
    assert_eq!(extract_secret_key_bearer(&parts), None);

    let invalid_pk_req = Request::builder()
        .header(PUBLISHABLE_KEY_HEADER, "sk_test_wrong_header")
        .body(())
        .unwrap();
    let (parts, _) = invalid_pk_req.into_parts();
    assert_eq!(extract_publishable_key(&parts), None);

    let sk_req = Request::builder()
        .header("authorization", "Bearer sk_live_elysiar_secret")
        .body(())
        .unwrap();
    let (parts, _) = sk_req.into_parts();
    assert_eq!(
        extract_secret_key_bearer(&parts),
        Some("sk_live_elysiar_secret")
    );
    assert_eq!(extract_publishable_key(&parts), None);

    let jwt_req = Request::builder()
        .header("authorization", "Bearer eyJhbGciOiJFUzI1NiJ9.payload.sig")
        .body(())
        .unwrap();
    let (parts, _) = jwt_req.into_parts();
    assert_eq!(extract_secret_key_bearer(&parts), None);

    let ctx = TenantContext {
        application_id: ApplicationId::new(),
        environment_id: EnvironmentId::new(),
        environment_kind: EnvironmentKind::Production,
        source: TenantSource::SecretKey {
            scopes: vec!["users:read".to_string(), "sync:read".to_string()],
        },
    };
    assert!(ctx.has_scope("users:read"));
    assert!(ctx.has_scope("sync:read"));
    assert!(!ctx.has_scope("users:write"));

    let pk_ctx = TenantContext {
        source: TenantSource::PublishableKey,
        ..ctx
    };
    assert!(!pk_ctx.has_scope("users:read"));
}

#[test]
fn test_audit_event_text_roundtrip() {
    let all_events = [
        AuditEvent::LoginSuccess,
        AuditEvent::LoginFailed,
        AuditEvent::Logout,
        AuditEvent::PasswordChanged,
        AuditEvent::EmailVerified,
        AuditEvent::SessionRevoked,
        AuditEvent::MfaEnabled,
        AuditEvent::MfaFailed,
        AuditEvent::AccountLocked,
    ];

    for event in all_events {
        let parsed = AuditEvent::from_str(event.as_str()).expect("valid audit event text");
        assert_eq!(parsed, event);
    }

    assert!(AuditEvent::from_str("UNKNOWN_EVENT").is_err());
}

#[cfg(test)]
mod postgres_isolation_suite {
    use auth_api::models::{EnrollmentPolicy, EnvironmentKind, User, UserId, WorkspaceRole};
    use auth_api::repositories::db::{begin_rls_enforced_tenant_tx, commit_tx};
    use auth_api::repositories::{global_identity, tenant_memberships, tenants, users};
    use chrono::{Duration, Utc};
    use sqlx::postgres::PgPoolOptions;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn connect_and_migrate() -> PgPool {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set to run PostgreSQL isolation tests");

        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .expect("connect to TEST_DATABASE_URL");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("apply migrations 001..011");

        pool
    }

    async fn seed_two_environments(
        pool: &PgPool,
    ) -> (
        auth_api::models::EnvironmentId,
        auth_api::models::EnvironmentId,
    ) {
        let suffix = Uuid::new_v4().simple().to_string();
        let dev = tenants::create_developer(
            pool,
            &format!("dev-{suffix}@example.com"),
            "Platform Dev",
            None,
        )
        .await
        .expect("create developer");

        let ws = tenants::create_workspace(pool, &format!("ws-{suffix}"), "Workspace")
            .await
            .expect("create workspace");
        tenants::add_workspace_member(pool, ws.id, dev.id, WorkspaceRole::Owner)
            .await
            .expect("add workspace owner");

        let ferrum =
            tenants::create_application(pool, ws.id, &format!("ferrum-{suffix}"), "Ferrum")
                .await
                .expect("create ferrum");
        let elysiar =
            tenants::create_application(pool, ws.id, &format!("elysiar-{suffix}"), "Elysiar")
                .await
                .expect("create elysiar");

        let env_ferrum = tenants::create_environment(
            pool,
            ferrum.id,
            EnvironmentKind::Production,
            "prod",
            EnrollmentPolicy::ConsentRequired,
        )
        .await
        .expect("create ferrum env");

        let env_elysiar = tenants::create_environment(
            pool,
            elysiar.id,
            EnvironmentKind::Production,
            "prod",
            EnrollmentPolicy::AutoJoin,
        )
        .await
        .expect("create elysiar env");

        (env_ferrum.id, env_elysiar.id)
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn test_composite_foreign_keys_reject_cross_environment_links() {
        let pool = connect_and_migrate().await;
        let (env_ferrum, env_elysiar) = seed_two_environments(&pool).await;

        let suffix = Uuid::new_v4().simple().to_string();
        let created_user = users::create_user(
            &pool,
            User::new(
                format!("alice-{suffix}@example.com"),
                "Alice".to_string(),
                None,
            ),
        )
        .await
        .expect("create global user");
        let user_id = UserId::from_uuid(created_user.id);

        let ferrum_member =
            tenant_memberships::create_application_user(&pool, env_ferrum, user_id, None)
                .await
                .expect("create ferrum membership");

        let cross_env_session = tenant_memberships::create_app_session(
            &pool,
            env_elysiar,
            ferrum_member.id,
            user_id,
            None,
            "hash_cross_env",
            None,
            None,
            Utc::now() + Duration::hours(2),
        )
        .await;
        assert!(
            cross_env_session.is_err(),
            "composite FK must reject app_session linking env_elysiar to ferrum application_user"
        );

        let elysiar_org = tenant_memberships::create_organization(
            &pool,
            env_elysiar,
            &format!("org-{suffix}"),
            "Elysiar Org",
        )
        .await
        .expect("create elysiar org");

        let cross_env_org_member = tenant_memberships::add_organization_member(
            &pool,
            env_elysiar,
            elysiar_org.id,
            ferrum_member.id,
        )
        .await;
        assert!(
            cross_env_org_member.is_err(),
            "composite FK must reject organization_member linking env_elysiar org to ferrum application_user"
        );
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn test_concurrent_rls_pool_isolation_zero_leakage() {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set to run PostgreSQL isolation tests");

        let migration_pool = connect_and_migrate().await;
        let (env_a, env_b) = seed_two_environments(&migration_pool).await;

        let suffix = Uuid::new_v4().simple().to_string();
        let u1 = users::create_user(
            &migration_pool,
            User::new(format!("u1-{suffix}@example.com"), "U1".to_string(), None),
        )
        .await
        .expect("create u1");
        let u2 = users::create_user(
            &migration_pool,
            User::new(format!("u2-{suffix}@example.com"), "U2".to_string(), None),
        )
        .await
        .expect("create u2");

        let small_pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("connect small pool");

        let mut tx_a = begin_rls_enforced_tenant_tx(&small_pool, env_a)
            .await
            .expect("begin rls tx_a");
        tenant_memberships::create_application_user(
            &mut *tx_a,
            env_a,
            UserId::from_uuid(u1.id),
            None,
        )
        .await
        .expect("insert env_a member");
        commit_tx(tx_a).await.expect("commit tx_a");

        let mut tx_b = begin_rls_enforced_tenant_tx(&small_pool, env_b)
            .await
            .expect("begin rls tx_b");
        tenant_memberships::create_application_user(
            &mut *tx_b,
            env_b,
            UserId::from_uuid(u2.id),
            None,
        )
        .await
        .expect("insert env_b member");
        commit_tx(tx_b).await.expect("commit tx_b");

        let mut tx_cross = begin_rls_enforced_tenant_tx(&small_pool, env_a)
            .await
            .expect("begin rls cross write check");
        let forbidden_insert = tenant_memberships::create_application_user(
            &mut *tx_cross,
            env_b,
            UserId::from_uuid(u1.id),
            None,
        )
        .await;
        assert!(
            forbidden_insert.is_err(),
            "RLS WITH CHECK must reject inserting env_b row inside env_a transaction"
        );
        drop(tx_cross);

        let mut handles = Vec::new();
        for i in 0..20 {
            let pool_clone = small_pool.clone();
            let (target_env, expected_user_id) = if i % 2 == 0 {
                (env_a, UserId::from_uuid(u1.id))
            } else {
                (env_b, UserId::from_uuid(u2.id))
            };

            handles.push(tokio::spawn(async move {
                let mut tx = begin_rls_enforced_tenant_tx(&pool_clone, target_env)
                    .await
                    .expect("begin rls tx");
                let visible_rows = sqlx::query_scalar::<_, Uuid>(
                    "SELECT user_id FROM application_users ORDER BY created_at ASC",
                )
                .fetch_all(&mut *tx)
                .await
                .expect("query visible rows under RLS");
                commit_tx(tx).await.expect("commit rls read tx");
                assert_eq!(visible_rows, vec![expected_user_id.into_inner()]);
            }));
        }

        for handle in handles {
            handle.await.expect("concurrent RLS task succeeded");
        }
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn test_verified_email_uniqueness_and_monotonic_version_triggers() {
        let pool = connect_and_migrate().await;
        let (env_a, _) = seed_two_environments(&pool).await;

        let suffix = Uuid::new_v4().simple().to_string();
        let u1 = users::create_user(
            &pool,
            User::new(
                format!("owner-{suffix}@example.com"),
                "Owner".to_string(),
                None,
            ),
        )
        .await
        .expect("create owner");
        let u2 = users::create_user(
            &pool,
            User::new(
                format!("attacker-{suffix}@example.com"),
                "Attacker".to_string(),
                None,
            ),
        )
        .await
        .expect("create second user");

        let shared_email = format!("Verified-{suffix}@Example.COM");
        let verified_row = global_identity::create_user_email(
            &pool,
            UserId::from_uuid(u1.id),
            &shared_email,
            Some(Utc::now()),
            true,
        )
        .await
        .expect("create verified email for owner");
        assert!(verified_row.is_verified());

        let hijack_attempt = global_identity::create_user_email(
            &pool,
            UserId::from_uuid(u2.id),
            &shared_email.to_lowercase(),
            Some(Utc::now()),
            false,
        )
        .await;
        assert!(
            hijack_attempt.is_err(),
            "partial unique index must block duplicate verified email ownership across users"
        );

        let membership = tenant_memberships::create_application_user(
            &pool,
            env_a,
            UserId::from_uuid(u1.id),
            None,
        )
        .await
        .expect("create membership");
        assert_eq!(membership.version, 1);

        let updated = tenant_memberships::update_application_user_metadata(
            &pool,
            env_a,
            membership.id,
            serde_json::json!({ "tier": "pro" }),
            serde_json::json!({}),
        )
        .await
        .expect("update membership metadata");
        assert_eq!(updated.version, 2);

        let updated_again = tenant_memberships::update_application_user_metadata(
            &pool,
            env_a,
            membership.id,
            serde_json::json!({ "tier": "enterprise" }),
            serde_json::json!({}),
        )
        .await
        .expect("second update");
        assert_eq!(updated_again.version, 3);
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn test_audit_logs_text_and_credential_resolution_live() {
        use auth_api::models::AuditEvent;
        use auth_api::repositories::audit_logs;
        use auth_api::services::tokens::hash_token;

        let pool = connect_and_migrate().await;
        let (env_a, _) = seed_two_environments(&pool).await;
        let suffix = Uuid::new_v4().simple().to_string();

        let pk = format!("pk_live_{suffix}");
        let sk_raw = format!("sk_live_{suffix}");
        let sk_hash = hash_token(&sk_raw);

        tenants::create_publishable_credential(&pool, env_a, "pk_live_", &pk, "Primary PK", None)
            .await
            .expect("create publishable credential");
        tenants::create_secret_credential(
            &pool,
            env_a,
            "sk_live_",
            &sk_hash,
            "Sync SK",
            &["users:read".to_string(), "sync:read".to_string()],
            None,
        )
        .await
        .expect("create secret credential");

        let resolved_pk_env = tenants::find_active_environment_by_publishable_key(&pool, &pk)
            .await
            .expect("query pk")
            .expect("pk should resolve active environment");
        assert_eq!(resolved_pk_env.id, env_a);

        let (resolved_sk_env, scopes) =
            tenants::find_active_environment_by_secret_hash(&pool, &sk_hash)
                .await
                .expect("query sk")
                .expect("sk should resolve active environment");
        assert_eq!(resolved_sk_env.id, env_a);
        assert_eq!(
            scopes,
            vec!["users:read".to_string(), "sync:read".to_string()]
        );

        let log = audit_logs::record_audit_log_direct(
            &pool,
            None,
            AuditEvent::LoginSuccess,
            Some("127.0.0.1".to_string()),
            Some("Phase01Test/1.0".to_string()),
            serde_json::json!({ "phase": 1 }),
        )
        .await
        .expect("insert TEXT audit_log");
        assert_eq!(log.event, AuditEvent::LoginSuccess);
    }
}
