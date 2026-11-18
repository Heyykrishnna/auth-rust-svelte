pub mod auth;
pub mod authorization;
pub mod client_auth;
pub mod control_auth;
pub mod csrf;
pub mod dynamic_cors;
pub mod rate_limit;
pub mod security_headers;
pub mod tenant;
pub mod tracing;

pub use auth::AuthenticatedUser;
pub use authorization::{
    require_all_permissions, require_any_permission, require_permission, require_role,
    RequirePermission, RequireRole,
};
pub use client_auth::AuthenticatedClientUser;
pub use control_auth::AuthenticatedDeveloper;
pub use csrf::{csrf_protection_middleware, generate_csrf_token};
pub use dynamic_cors::dynamic_cors_middleware;
pub use rate_limit::{api_rate_limit_layer, auth_rate_limit_layer, sensitive_rate_limit_layer};
pub use security_headers::security_headers_middleware;
pub use tenant::{
    PublishableKeyTenant, SecretKeyAuth, TenantContext, TenantSource, PUBLISHABLE_KEY_HEADER,
};
pub use tracing::{init_telemetry, init_worker_telemetry, telemetry_correlation_middleware};
