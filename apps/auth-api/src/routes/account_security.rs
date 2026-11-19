use axum::routing::{delete, get, post};
use axum::Router;

use crate::handlers::account_security as h;
use crate::middleware::{require_permission, sensitive_rate_limit_layer};
use crate::models::Permission;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    let sensitive_ops = Router::new()
        .route("/totp", delete(h::disable_totp))
        .route("/backup-codes/generate", post(h::generate_backup_codes))
        .route("/step-up", post(h::authenticate_step_up))
        .route("/sessions/revoke-others", post(h::revoke_other_sessions))
        .layer(sensitive_rate_limit_layer());

    let general_security = Router::new()
        .route("/summary", get(h::get_security_summary))
        .route(
            "/settings",
            get(h::get_security_settings).put(h::update_security_settings),
        )
        .route("/totp/enroll", post(h::enroll_totp))
        .route("/totp/confirm", post(h::confirm_totp))
        .route(
            "/passkeys/options",
            get(h::get_passkey_registration_options),
        )
        .route("/passkeys", get(h::list_passkeys).post(h::register_passkey))
        .route("/passkeys/:id", delete(h::delete_passkey))
        .route("/backup-codes/status", get(h::get_backup_codes_status))
        .route("/backup-codes/verify", post(h::verify_backup_code))
        .route("/devices", get(h::list_devices))
        .route(
            "/devices/:id/trust",
            post(h::trust_device).delete(h::untrust_device),
        )
        .route("/devices/:id", delete(h::remove_device))
        .route("/alerts", get(h::list_security_alerts))
        .route("/alerts/:id/resolve", post(h::resolve_security_alert))
        .route("/threat-intel", get(h::get_threat_intelligence))
        .route("/login-history", get(h::get_login_history))
        .route("/recovery", get(h::get_recovery_status))
        .route_layer(require_permission(Permission::ProfileRead));

    Router::new().merge(sensitive_ops).merge(general_security)
}
