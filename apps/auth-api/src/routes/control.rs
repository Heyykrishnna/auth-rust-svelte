use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::handlers::{
    analytics, backend_sync, client_auth, control_ops, control_plane, control_webhooks,
    organizations,
};
use crate::middleware::{api_rate_limit_layer, auth_rate_limit_layer, sensitive_rate_limit_layer};
use crate::AppState;

pub fn control_routes() -> Router<AppState> {
    let auth_routes = Router::new()
        .route("/auth/sign-up", post(control_plane::developer_sign_up))
        .route("/auth/sign-in", post(control_plane::developer_sign_in))
        .route("/auth/sign-out", post(control_plane::developer_sign_out))
        .route("/auth/me", get(control_plane::developer_me))
        .route(
            "/auth/change-password",
            post(control_ops::change_developer_password),
        )
        .layer(auth_rate_limit_layer());

    let credential_mutation_routes = Router::new()
        .route(
            "/applications/:app_id/environments/:env_kind/credentials",
            post(control_plane::create_credential),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/credentials/:cred_id/rotate",
            post(control_plane::rotate_credential),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/credentials/:cred_id",
            delete(control_plane::revoke_credential),
        )
        .layer(sensitive_rate_limit_layer());

    let management_routes = Router::new()
        .route(
            "/workspaces",
            get(control_plane::list_workspaces).post(control_plane::create_workspace),
        )
        .route(
            "/workspaces/:workspace_id",
            patch(control_plane::update_workspace),
        )
        .route(
            "/applications",
            get(control_plane::list_applications).post(control_plane::create_application),
        )
        .route(
            "/applications/:app_id",
            get(control_plane::get_application)
                .patch(control_plane::update_application)
                .delete(control_plane::delete_application),
        )
        .route(
            "/applications/:app_id/environments/:env_kind",
            get(control_plane::get_environment).patch(control_plane::update_environment),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/credentials",
            get(control_plane::list_credentials),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/domains",
            get(control_plane::list_domains).post(control_plane::create_domain),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/domains/:domain_id",
            delete(control_plane::delete_domain),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/webhooks",
            get(control_webhooks::list_webhooks).post(control_webhooks::create_webhook),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/webhooks/:endpoint_id",
            patch(control_webhooks::update_webhook).delete(control_webhooks::delete_webhook),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/webhooks/:endpoint_id/rotate-secret",
            post(control_webhooks::rotate_webhook_secret),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/webhooks/:endpoint_id/test",
            post(control_webhooks::test_webhook),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/webhooks/:endpoint_id/deliveries",
            get(control_webhooks::list_deliveries),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/webhooks/:endpoint_id/deliveries/:delivery_id/replay",
            post(control_webhooks::replay_delivery),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/webhooks/:endpoint_id/replay",
            post(control_webhooks::replay_range),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/overview",
            get(control_ops::get_overview),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/analytics",
            get(analytics::get_analytics),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/analytics/refresh",
            post(analytics::refresh_analytics),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/oauth-providers",
            get(control_ops::list_oauth_providers),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/oauth-providers/:provider",
            axum::routing::put(control_ops::upsert_oauth_provider),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/users",
            get(control_ops::list_users),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/users/:app_user_id",
            get(control_ops::get_user_detail),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/users/:app_user_id/metadata",
            patch(control_ops::update_user_metadata),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/users/:app_user_id/roles",
            post(control_ops::update_user_role),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/users/:app_user_id/suspend",
            post(control_ops::suspend_user),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/users/:app_user_id/restore",
            post(control_ops::restore_user),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/users/:app_user_id/password-reset",
            post(control_ops::trigger_password_reset),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/users/:app_user_id/sessions/revoke-all",
            post(control_ops::revoke_all_user_sessions),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/sessions/:session_id",
            delete(control_ops::revoke_session),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/audit-logs",
            get(control_ops::list_audit_logs),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/organizations",
            get(organizations::control_list_organizations)
                .post(organizations::control_create_organization),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/organizations/:org_id",
            get(organizations::control_get_organization)
                .patch(organizations::control_update_organization)
                .delete(organizations::control_delete_organization),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/organizations/:org_id/members",
            get(organizations::control_list_org_members)
                .post(organizations::control_add_org_member),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/organizations/:org_id/members/:member_id",
            patch(organizations::control_update_org_member_role)
                .delete(organizations::control_remove_org_member),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/organizations/:org_id/invitations",
            get(organizations::control_list_org_invitations)
                .post(organizations::control_create_org_invitation),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/organizations/:org_id/invitations/:invitation_id",
            delete(organizations::control_revoke_org_invitation),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/permissions",
            get(organizations::control_list_permissions)
                .post(organizations::control_create_permission),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/permissions/:permission_id",
            delete(organizations::control_delete_permission),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/roles",
            get(organizations::control_list_roles).post(organizations::control_create_role),
        )
        .route(
            "/applications/:app_id/environments/:env_kind/roles/:role_id",
            patch(organizations::control_update_role).delete(organizations::control_delete_role),
        )
        .route(
            "/workspaces/:workspace_id/members",
            get(control_ops::list_workspace_members).post(control_ops::add_workspace_member),
        )
        .route(
            "/workspaces/:workspace_id/members/:member_id",
            patch(control_ops::update_workspace_member_role)
                .delete(control_ops::remove_workspace_member),
        )
        .layer(api_rate_limit_layer());

    Router::new()
        .merge(auth_routes)
        .merge(credential_mutation_routes)
        .merge(management_routes)
}

pub fn client_v1_routes() -> Router<AppState> {
    let sensitive_auth = Router::new()
        .route(
            "/auth/sign-up/verify",
            post(client_auth::client_verify_sign_up),
        )
        .route(
            "/auth/sign-up/resend-otp",
            post(client_auth::client_resend_sign_up_otp),
        )
        .route("/auth/verify-email", post(client_auth::client_verify_email))
        .route(
            "/auth/password/forgot",
            post(client_auth::client_forgot_password),
        )
        .route(
            "/auth/password/reset",
            post(client_auth::client_reset_password),
        )
        .layer(sensitive_rate_limit_layer());

    let auth_flow = Router::new()
        .route("/auth/sign-up", post(client_auth::client_sign_up))
        .route("/auth/sign-in", post(client_auth::client_sign_in))
        .route("/auth/refresh", post(client_auth::client_refresh))
        .route("/auth/sign-out", post(client_auth::client_sign_out))
        .route(
            "/auth/password/change",
            post(client_auth::client_change_password),
        )
        .route(
            "/auth/oauth/:provider/start",
            get(client_auth::client_oauth_start),
        )
        .route(
            "/auth/oauth/:provider/callback",
            post(client_auth::client_oauth_callback),
        )
        .layer(auth_rate_limit_layer());

    let resource_routes = Router::new()
        .route("/config", get(control_plane::client_environment_config))
        .route(
            "/users/me",
            get(client_auth::client_get_me)
                .patch(client_auth::client_update_me)
                .delete(organizations::client_delete_account),
        )
        .route(
            "/users/me/emails",
            get(organizations::client_list_emails).post(organizations::client_add_email),
        )
        .route(
            "/users/me/emails/verify",
            post(organizations::client_verify_email_address),
        )
        .route(
            "/users/me/emails/:email_id/primary",
            post(organizations::client_set_primary_email),
        )
        .route(
            "/users/me/emails/:email_id",
            delete(organizations::client_remove_email),
        )
        .route(
            "/users/me/oauth-accounts",
            get(organizations::client_list_oauth_accounts),
        )
        .route(
            "/users/me/oauth-accounts/:provider",
            delete(organizations::client_unlink_oauth_account),
        )
        .route(
            "/users/me/export",
            get(organizations::client_export_user_data),
        )
        .route(
            "/sessions",
            get(client_auth::client_list_sessions).delete(client_auth::client_revoke_all_sessions),
        )
        .route(
            "/sessions/active-organization",
            post(organizations::client_switch_active_organization),
        )
        .route("/sessions/:id", delete(client_auth::client_revoke_session))
        .route(
            "/organizations",
            get(organizations::client_list_organizations)
                .post(organizations::client_create_organization),
        )
        .route(
            "/organizations/invitations/accept",
            post(organizations::client_accept_org_invitation),
        )
        .route(
            "/organizations/:org_id",
            get(organizations::client_get_organization)
                .patch(organizations::client_update_organization)
                .delete(organizations::client_delete_organization),
        )
        .route(
            "/organizations/:org_id/members",
            get(organizations::client_list_org_members),
        )
        .route(
            "/organizations/:org_id/members/:member_id",
            patch(organizations::client_update_org_member_role)
                .delete(organizations::client_remove_org_member),
        )
        .route(
            "/organizations/:org_id/invitations",
            get(organizations::client_list_org_invitations)
                .post(organizations::client_create_org_invitation),
        )
        .route(
            "/organizations/:org_id/invitations/:invitation_id",
            delete(organizations::client_revoke_org_invitation),
        )
        .layer(api_rate_limit_layer());

    Router::new()
        .merge(sensitive_auth)
        .merge(auth_flow)
        .merge(resource_routes)
}

pub fn backend_v1_routes() -> Router<AppState> {
    Router::new()
        .route("/ping", get(control_plane::backend_ping))
        .route("/users", get(backend_sync::list_users))
        .route("/users/by-sub/:sub", get(backend_sync::get_user_by_sub))
        .route(
            "/users/:app_user_id",
            get(backend_sync::get_user_by_app_user_id),
        )
        .route(
            "/users/:app_user_id/metadata",
            patch(backend_sync::update_user_metadata),
        )
        .route(
            "/users/:app_user_id/suspend",
            post(backend_sync::suspend_user),
        )
        .route(
            "/users/:app_user_id/restore",
            post(backend_sync::restore_user),
        )
        .route(
            "/users/:app_user_id/sessions/revoke-all",
            post(backend_sync::revoke_all_user_sessions),
        )
        .route(
            "/sessions/:session_id",
            get(backend_sync::get_session_status),
        )
        .route(
            "/sessions/:session_id/revoke",
            post(backend_sync::revoke_session),
        )
        .route("/events", get(backend_sync::list_events))
        .route(
            "/organizations",
            get(organizations::backend_list_organizations)
                .post(organizations::backend_create_organization),
        )
        .route(
            "/organizations/:org_id",
            get(organizations::backend_get_organization)
                .patch(organizations::backend_update_organization)
                .delete(organizations::backend_delete_organization),
        )
        .route(
            "/organizations/:org_id/members",
            get(organizations::backend_list_org_members)
                .post(organizations::backend_add_org_member),
        )
        .route(
            "/organizations/:org_id/members/:member_id",
            patch(organizations::backend_update_org_member_role)
                .delete(organizations::backend_remove_org_member),
        )
        .route(
            "/organizations/:org_id/invitations",
            get(organizations::backend_list_org_invitations)
                .post(organizations::backend_create_org_invitation),
        )
        .route(
            "/organizations/:org_id/invitations/:invitation_id",
            delete(organizations::backend_revoke_org_invitation),
        )
        .route("/permissions", get(organizations::backend_list_permissions))
        .route("/roles", get(organizations::backend_list_roles))
        .layer(api_rate_limit_layer())
}
