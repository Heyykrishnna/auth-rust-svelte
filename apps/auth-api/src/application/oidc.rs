// OIDC use case — placeholder for Google and GitHub OIDC flows.
// Full implementation requires running OIDC provider configuration.
// See infrastructure/oidc.rs for the discovery and token exchange logic.

use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{domain::errors::DomainError, AppState};

/// Supported OIDC providers.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OidcProvider {
    Google,
    Github,
}

/// Get the authorization URL for a given OIDC provider.
#[instrument(skip(state))]
pub async fn get_authorization_url(
    state: &AppState,
    provider: OidcProvider,
) -> Result<String, DomainError> {
    match provider {
        OidcProvider::Google => {
            let client_id = state.config.google_client_id.as_deref()
                .ok_or_else(|| DomainError::OidcError("Google OIDC not configured".to_string()))?;
            let redirect_uri = state.config.google_redirect_uri.as_deref()
                .ok_or_else(|| DomainError::OidcError("Google redirect URI not configured".to_string()))?;

            // Build Google OAuth2 authorization URL
            Ok(format!(
                "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid+email+profile&state={}",
                client_id,
                urlencoding::encode(redirect_uri),
                uuid::Uuid::new_v4()
            ))
        }
        OidcProvider::Github => {
            let client_id = state.config.github_client_id.as_deref()
                .ok_or_else(|| DomainError::OidcError("GitHub OIDC not configured".to_string()))?;
            let redirect_uri = state.config.github_redirect_uri.as_deref()
                .ok_or_else(|| DomainError::OidcError("GitHub redirect URI not configured".to_string()))?;

            Ok(format!(
                "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=user:email&state={}",
                client_id,
                urlencoding::encode(redirect_uri),
                uuid::Uuid::new_v4()
            ))
        }
    }
}

/// Handle the OIDC callback (code exchange + user provisioning).
#[instrument(skip(state, code))]
pub async fn handle_callback(
    _state: &AppState,
    _provider: OidcProvider,
    _code: String,
    _state_param: String,
) -> Result<(), DomainError> {
    // TODO: Exchange code for tokens, fetch user profile, upsert user in DB
    // This requires HTTP calls to the provider's token endpoint.
    Err(DomainError::OidcError("OIDC callback not yet implemented — configure client ID/secret first".to_string()))
}
