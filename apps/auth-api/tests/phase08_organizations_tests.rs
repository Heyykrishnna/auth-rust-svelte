use auth_api::services::identity_advanced::{
    can_remove_email_guard, can_unlink_oauth_provider_guard, generate_email_otp_code,
};
use auth_api::services::organizations::{
    ensure_not_last_owner, evaluate_scoped_permission, generate_invitation_token,
    hash_invitation_token, validate_org_slug, validate_permission_slug,
};

#[test]
fn test_organization_and_permission_slug_validation() {
    assert_eq!(validate_org_slug("Acme-Corp-01").unwrap(), "acme-corp-01");
    assert_eq!(validate_org_slug("  ab  ").unwrap(), "ab");
    assert!(validate_org_slug("a").is_err());
    assert!(validate_org_slug("-leading").is_err());
    assert!(validate_org_slug("trailing-").is_err());
    assert!(validate_org_slug("bad_slug").is_err());
    assert!(validate_org_slug("has space").is_err());

    assert_eq!(
        validate_permission_slug("billing:invoices:write").unwrap(),
        "billing:invoices:write"
    );
    assert_eq!(
        validate_permission_slug("ORG.SETTINGS:READ").unwrap(),
        "org.settings:read"
    );
    assert!(validate_permission_slug("ab").is_err());
    assert!(validate_permission_slug(":empty").is_err());
    assert!(validate_permission_slug("1starts_with_digit").is_err());
    assert!(validate_permission_slug("bad char:write").is_err());
}

#[test]
fn test_invitation_token_entropy_and_hashing() {
    let t1 = generate_invitation_token();
    let t2 = generate_invitation_token();
    assert_ne!(t1, t2);
    assert!(t1.starts_with("orginv_"));
    assert!(t1.len() >= 45);

    let h1 = hash_invitation_token(&t1);
    let h1_repeat = hash_invitation_token(&t1);
    let h2 = hash_invitation_token(&t2);
    assert_eq!(h1, h1_repeat);
    assert_ne!(h1, h2);
    assert_eq!(h1.len(), 64);
}

#[test]
fn test_scoped_permission_evaluation_boundary() {
    let app_perms = vec!["profile.read".to_string(), "app:reports:read".to_string()];
    let org_perms = vec![
        "org:billing:write".to_string(),
        "org:members:manage".to_string(),
    ];

    assert!(evaluate_scoped_permission(
        &app_perms,
        &org_perms,
        Some("org_a"),
        Some("org_a"),
        "app:reports:read"
    ));
    assert!(evaluate_scoped_permission(
        &app_perms,
        &org_perms,
        Some("org_a"),
        Some("org_b"),
        "app:reports:read"
    ));
    assert!(evaluate_scoped_permission(
        &app_perms,
        &org_perms,
        Some("org_a"),
        Some("org_a"),
        "org:billing:write"
    ));
    assert!(!evaluate_scoped_permission(
        &app_perms,
        &org_perms,
        Some("org_a"),
        Some("org_b"),
        "org:billing:write"
    ));
    assert!(!evaluate_scoped_permission(
        &app_perms,
        &org_perms,
        None,
        Some("org_a"),
        "org:billing:write"
    ));
    assert!(evaluate_scoped_permission(
        &app_perms,
        &org_perms,
        Some("org_a"),
        None,
        "org:billing:write"
    ));
    assert!(!evaluate_scoped_permission(
        &app_perms,
        &org_perms,
        Some("org_a"),
        Some("org_a"),
        "org:billing:delete"
    ));
}

#[test]
fn test_last_owner_and_account_lockout_guards() {
    assert!(ensure_not_last_owner(true, false, 1).is_err());
    assert!(ensure_not_last_owner(true, false, 0).is_err());
    assert!(ensure_not_last_owner(true, false, 2).is_ok());
    assert!(ensure_not_last_owner(true, true, 1).is_ok());
    assert!(ensure_not_last_owner(false, false, 1).is_ok());

    assert!(can_remove_email_guard(true, true, 2).is_err());
    assert!(can_remove_email_guard(false, true, 1).is_err());
    assert!(can_remove_email_guard(false, true, 2).is_ok());
    assert!(can_remove_email_guard(false, false, 1).is_ok());

    assert!(can_unlink_oauth_provider_guard(false, 1).is_err());
    assert!(can_unlink_oauth_provider_guard(false, 2).is_ok());
    assert!(can_unlink_oauth_provider_guard(true, 1).is_ok());

    let otp = generate_email_otp_code();
    assert_eq!(otp.len(), 6);
    assert!(otp.chars().all(|c| c.is_ascii_digit()));
}

#[cfg(test)]
mod live_phase08_organizations_suite {
    use auth_api::models::{AppUserId, EnvironmentId, OrganizationId, UserId};
    use auth_api::services::organizations::{
        self, CreateEnvironmentRoleInput, CreateInvitationInput, CreateOrganizationInput,
        UpdateOrganizationInput,
    };
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn test_live_org_isolation_invitations_rbac_and_sync() {
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

        let suffix = Uuid::new_v4().to_string()[..8].to_string();
        let dev_id: Uuid = sqlx::query_scalar(
            "INSERT INTO developers (email, password_hash, display_name) VALUES ($1, 'hash', 'Phase08 Dev') RETURNING id",
        )
        .bind(format!("p08-dev-{suffix}@example.com"))
        .fetch_one(&pool)
        .await
        .unwrap();

        let ws_id: Uuid = sqlx::query_scalar(
            "INSERT INTO developer_workspaces (name, slug) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("P08 WS {suffix}"))
        .bind(format!("p08-ws-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap();

        let app_id: Uuid = sqlx::query_scalar(
            "INSERT INTO applications (workspace_id, name, slug) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(ws_id)
        .bind(format!("P08 App {suffix}"))
        .bind(format!("p08-app-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap();

        let env_a_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO application_environments (application_id, kind, slug) VALUES ($1, 'development', $2) RETURNING id",
        )
        .bind(app_id)
        .bind(format!("p08-dev-env-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let env_b_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO application_environments (application_id, kind, slug) VALUES ($1, 'production', $2) RETURNING id",
        )
        .bind(app_id)
        .bind(format!("p08-prod-env-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let env_a = EnvironmentId::from(env_a_uuid);
        let env_b = EnvironmentId::from(env_b_uuid);

        let owner_user_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, password_hash, display_name, email_verified) VALUES ($1, 'hash', 'Owner', TRUE) RETURNING id",
        )
        .bind(format!("owner-{suffix}@example.com"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let invitee_email = format!("invitee-{suffix}@example.com");
        let invitee_user_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, password_hash, display_name, email_verified) VALUES ($1, 'hash', 'Invitee', TRUE) RETURNING id",
        )
        .bind(&invitee_email)
        .fetch_one(&pool)
        .await
        .unwrap();
        let wrong_user_uuid: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, password_hash, display_name, email_verified) VALUES ($1, 'hash', 'Wrong', TRUE) RETURNING id",
        )
        .bind(format!("wrong-{suffix}@example.com"))
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO user_emails (user_id, email, verified_at, is_primary) VALUES ($1, $2, NOW(), TRUE)",
        )
        .bind(invitee_user_uuid)
        .bind(&invitee_email)
        .execute(&pool)
        .await
        .unwrap();

        let owner_app_user: Uuid = sqlx::query_scalar(
            "INSERT INTO application_users (environment_id, user_id) VALUES ($1, $2) RETURNING id",
        )
        .bind(env_a)
        .bind(owner_user_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();

        let org = organizations::create_organization(
            &pool,
            env_a,
            Some(AppUserId::from(owner_app_user)),
            "user",
            Some(owner_user_uuid),
            CreateOrganizationInput {
                name: "Acme Workspace".to_string(),
                slug: format!("acme-{suffix}"),
                logo_url: None,
                metadata: None,
            },
        )
        .await
        .unwrap();

        assert!(organizations::get_organization(&pool, env_b, org.id)
            .await
            .is_err());

        let members = organizations::list_organization_members(&pool, env_a, org.id)
            .await
            .unwrap();
        assert_eq!(members.len(), 1);
        assert!(members[0].roles.contains(&"org:owner".to_string()));
        assert!(organizations::remove_organization_member(
            &pool,
            env_a,
            org.id,
            members[0].id,
            "user",
            Some(owner_user_uuid),
        )
        .await
        .is_err());

        let created_inv = organizations::create_invitation(
            &pool,
            env_a,
            org.id,
            CreateInvitationInput {
                email: &invitee_email,
                role_slug: Some("org:admin"),
                expires_in_days: Some(7),
                invited_by_app_user_id: Some(AppUserId::from(owner_app_user)),
                actor_type: "user",
                actor_id: Some(owner_user_uuid),
            },
        )
        .await
        .unwrap();

        assert!(organizations::accept_invitation(
            &pool,
            env_a,
            UserId::from(wrong_user_uuid),
            &created_inv.invitation_token,
        )
        .await
        .is_err());

        let accepted = organizations::accept_invitation(
            &pool,
            env_a,
            UserId::from(invitee_user_uuid),
            &created_inv.invitation_token,
        )
        .await
        .unwrap();
        assert!(accepted.roles.contains(&"org:admin".to_string()));

        assert!(organizations::accept_invitation(
            &pool,
            env_a,
            UserId::from(invitee_user_uuid),
            &created_inv.invitation_token,
        )
        .await
        .is_err());

        let perm = organizations::create_environment_permission(
            &pool,
            env_a,
            "billing:invoices:write",
            Some("Write invoices"),
            Some(dev_id),
        )
        .await
        .unwrap();
        let perms = vec![perm.slug.clone()];
        let role = organizations::create_environment_role(
            &pool,
            env_a,
            CreateEnvironmentRoleInput {
                slug: "org:billing_admin",
                name: "Billing Admin",
                scope: "organization",
                description: Some("Manages billing"),
                permission_slugs: &perms,
                actor_id: Some(dev_id),
            },
        )
        .await
        .unwrap();
        assert!(!role.is_system);
        assert_eq!(role.permissions, vec!["billing:invoices:write".to_string()]);

        let updated_org = organizations::update_organization(
            &pool,
            env_a,
            org.id,
            "developer",
            Some(dev_id),
            UpdateOrganizationInput {
                name: Some("Acme Workspace Renamed".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(updated_org.version > org.version);

        let page =
            organizations::list_organizations_paginated(&pool, env_a, None, None, None, Some(10))
                .await
                .unwrap();
        assert!(page
            .data
            .iter()
            .any(|o| o.id == OrganizationId::from(org.id.into_inner())));

        organizations::delete_environment_role(&pool, env_a, role.id, Some(dev_id))
            .await
            .unwrap();
        organizations::delete_environment_permission(&pool, env_a, perm.id, Some(dev_id))
            .await
            .unwrap();
    }
}
