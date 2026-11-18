use uuid::Uuid;

use crate::models::{AppSessionId, EnvironmentId, SsoSessionId};

pub const GLOBAL_PREFIX: &str = "idp:global";

pub fn global_key(namespace: &str, identifier: &str) -> String {
    format!("{GLOBAL_PREFIX}:{namespace}:{identifier}")
}

pub fn env_key(environment_id: EnvironmentId, namespace: &str, identifier: &str) -> String {
    format!("env:{environment_id}:{namespace}:{identifier}")
}

pub fn global_sso_session_key(session_id: SsoSessionId) -> String {
    global_key("sso_session", &session_id.to_string())
}

pub fn global_legacy_session_key(session_id: Uuid) -> String {
    global_key("session", &session_id.to_string())
}

pub fn global_token_blacklist_key(jti: &str) -> String {
    global_key("blacklist", jti)
}

pub fn global_login_attempt_key(normalized_email: &str) -> String {
    global_key("login_attempt", normalized_email)
}

pub fn global_verify_code_key(code: &str) -> String {
    global_key("verify_code", code)
}

pub fn global_password_reset_key(token: &str) -> String {
    global_key("password_reset", token)
}

pub fn global_rate_limit_key(identifier: &str) -> String {
    global_key("rate_limit", identifier)
}

pub fn global_pending_registration_key(normalized_email: &str) -> String {
    global_key("pending_reg", normalized_email)
}

pub fn env_app_session_key(environment_id: EnvironmentId, session_id: AppSessionId) -> String {
    env_key(environment_id, "app_session", &session_id.to_string())
}

pub fn env_token_blacklist_key(environment_id: EnvironmentId, jti: &str) -> String {
    env_key(environment_id, "blacklist", jti)
}

pub fn env_login_attempt_key(environment_id: EnvironmentId, normalized_email: &str) -> String {
    env_key(environment_id, "login_attempt", normalized_email)
}

pub fn env_pending_registration_key(
    environment_id: EnvironmentId,
    normalized_email: &str,
) -> String {
    env_key(environment_id, "pending_reg", normalized_email)
}

pub fn env_rate_limit_key(environment_id: EnvironmentId, identifier: &str) -> String {
    env_key(environment_id, "rate_limit", identifier)
}

pub fn env_credential_cache_key(environment_id: EnvironmentId, key_digest: &str) -> String {
    env_key(environment_id, "credential", key_digest)
}

pub fn env_cors_origins_key(environment_id: EnvironmentId) -> String {
    env_key(environment_id, "cors_origins", "allowlist")
}
