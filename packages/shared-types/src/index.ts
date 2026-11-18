export type UserStatus = 'active' | 'suspended' | 'pending' | string;

export interface AuthUser {
  id: string;
  email: string;
  display_name: string;
  avatar_url: string | null;
  email_verified: boolean;
  status: UserStatus;
  created_at: string;
  roles?: string[];
  permissions?: string[];
}

export type UserProfile = AuthUser;

export interface SessionItem {
  id: string;
  user_agent: string | null;
  ip_address: string | null;
  created_at: string;
  last_used_at: string;
  is_current: boolean;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  token_type: 'Bearer';
  expires_in: number;
}

export interface AuthSession {
  user: AuthUser;
  tokens?: AuthTokens;
  message?: string;
}

export interface ActiveSession {
  session_id: string;
  user_id: string;
  user_agent: string | null;
  ip_address: string | null;
  created_at: string;
  expires_at: string;
  last_seen_at: string;
}

export interface JwtPayload {
  sub: string;
  email: string;
  roles: string[];
  permissions: string[];
  session_id: string;
  exp: number;
  iat: number;
  iss?: string;
  aud?: string;
}

export type TokenType = 'access' | 'refresh' | 'reset' | 'verification';

export interface RegisterRequest {
  email: string;
  password: string;
  display_name: string;
}

export interface RegisterInitiateResponse {
  status: 'pending_verification' | string;
  email: string;
  message: string;
}

export interface RegisterVerifyRequest {
  email: string;
  code: string;
}

export interface ResendOtpRequest {
  email: string;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface RefreshRequest {
  refresh_token?: string;
}

export interface ForgotPasswordRequest {
  email: string;
}

export interface ResetPasswordRequest {
  token: string;
  new_password: string;
}

export interface VerifyCodeRequest {
  code: string;
  email?: string;
}

export interface UpdateProfileRequest {
  display_name: string;
  avatar_url?: string | null;
}

export interface PaginationQuery {
  limit?: number;
  offset?: number;
}

export interface OidcProvider {
  provider: 'google' | 'github';
  authorization_url: string;
}

export interface ApiErrorBody {
  code: string;
  message: string;
  request_id?: string;
  details?: Record<string, unknown> | null;
}

export interface ApiErrorEnvelope {
  error: ApiErrorBody;
  status: number;
}

export interface ApiResponse<T> {
  data?: T;
  error?: string | ApiErrorBody;
}

export interface MessageResponse {
  message: string;
}

export interface HealthResponse {
  status: string;
  version: string;
}

export interface AuditLogEntry {
  id: string;
  user_id: string | null;
  action: string;
  ip_address: string | null;
  user_agent: string | null;
  timestamp: string;
  status: 'success' | 'failure';
  metadata?: Record<string, unknown>;
}

export type DomainEventType =
  | 'user.created'
  | 'user.updated'
  | 'user.deleted'
  | 'user.suspended'
  | 'user.email_verified'
  | 'membership.created'
  | 'membership.updated'
  | 'membership.revoked'
  | 'session.created'
  | 'session.revoked'
  | 'role.assigned'
  | 'role.revoked'
  | 'organization.created'
  | 'organization.updated'
  | 'organization.deleted'
  | 'organization.member_added'
  | 'organization.member_updated'
  | 'organization.member_removed';

export type AggregateType =
  | 'user'
  | 'application_user'
  | 'app_session'
  | 'organization'
  | 'organization_member';

export interface DomainEventEnvelope<T = Record<string, unknown>> {
  spec_version: '1.0';
  event_id: string;
  event_type: DomainEventType;
  environment_id?: string;
  occurred_at: string;
  aggregate_type: AggregateType;
  aggregate_id: string;
  aggregate_version: number;
  data: T;
}

export type Role = 'admin' | 'user' | 'moderator' | string;

export type Permission =
  | 'profile.read'
  | 'profile.write'
  | 'sessions.read'
  | 'sessions.delete'
  | 'users.read'
  | 'users.delete'
  | string;

export interface UserRole {
  user_id: string;
  role: Role;
}

export type EnvironmentKind = 'development' | 'production';
export type EnrollmentPolicy = 'consent_required' | 'auto_join' | 'invite_only';
export type WorkspaceRole = 'owner' | 'admin' | 'developer' | 'viewer';
export type CredentialType = 'publishable' | 'secret';

export interface DeveloperProfile {
  id: string;
  email: string;
  display_name: string;
  email_verified: boolean;
  status: string;
  created_at: string;
}

export interface DeveloperAuthResponse {
  developer: DeveloperProfile;
  token: string;
}

export interface WorkspaceSummary {
  id: string;
  slug: string;
  name: string;
  status: string;
  role: WorkspaceRole | string;
  created_at: string;
}

export interface ApplicationDto {
  id: string;
  workspace_id: string;
  slug: string;
  name: string;
  status: string;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface ApplicationEnvironmentDto {
  id: string;
  application_id: string;
  kind: EnvironmentKind;
  slug: string;
  enrollment_policy: EnrollmentPolicy;
  session_ttl_secs: number;
  access_token_ttl_secs: number;
  require_email_verification: boolean;
  require_mfa: boolean;
  branding: Record<string, unknown>;
  settings: Record<string, unknown>;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface ApplicationWithEnvironmentsDto {
  application: ApplicationDto;
  environments: ApplicationEnvironmentDto[];
}

export interface CreatedApplicationResponse {
  application: ApplicationDto;
  environments: ApplicationEnvironmentDto[];
  development_publishable_key: string;
  production_publishable_key: string;
  initial_development_secret_key: string;
}

export interface CredentialSummaryDto {
  id: string;
  environment_id: string;
  credential_type: CredentialType;
  key_prefix: string;
  public_key: string | null;
  name: string;
  scopes: string[];
  last_used_at: string | null;
  expires_at: string | null;
  revoked_at: string | null;
  created_at: string;
}

export interface CreatedCredentialResponse {
  credential: CredentialSummaryDto;
  secret_plaintext?: string;
}

export interface ApplicationDomainDto {
  id: string;
  environment_id: string;
  domain: string;
  is_verified: boolean;
  verification_token: string | null;
  allowed_origins: string[];
  redirect_uris: string[];
  post_logout_redirect_uris: string[];
  created_at: string;
  updated_at: string;
}

export interface AuditLogEntryDto {
  id: string;
  environment_id: string | null;
  user_id: string | null;
  actor_type: string;
  actor_id: string | null;
  resource_type: string | null;
  resource_id: string | null;
  event: string;
  ip_address: string | null;
  user_agent: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface EnvironmentOverviewDto {
  environment_id: string;
  environment_kind: EnvironmentKind;
  environment_slug: string;
  total_users: number;
  active_users: number;
  suspended_users: number;
  unverified_users: number;
  new_users_7d: number;
  active_sessions: number;
  auth_success_24h: number;
  auth_failure_24h: number;
  auth_success_7d: number;
  auth_failure_7d: number;
  active_api_keys: number;
  webhook_endpoints_active: number;
  webhook_deliveries_succeeded_24h: number;
  webhook_deliveries_failed_24h: number;
  webhook_dead_letters: number;
  issuer_url: string;
  jwks_uri: string;
  publishable_key: string | null;
  recent_events: AuditLogEntryDto[];
}

export interface OAuthProviderConfigDto {
  provider: 'google' | 'github' | string;
  enabled: boolean;
  use_platform_credentials: boolean;
  client_id: string | null;
  has_custom_secret: boolean;
  scopes: string[];
  callback_url: string;
  updated_at: string | null;
}

export interface EnvironmentUserListItemDto {
  app_user_id: string;
  sub: string;
  email: string;
  email_verified: boolean;
  display_name: string;
  avatar_url: string | null;
  status: string;
  external_id: string | null;
  roles: string[];
  active_sessions_count: number;
  version: number;
  joined_at: string;
  last_active_at: string | null;
  updated_at: string;
}

export interface PaginatedEnvironmentUsersDto {
  users: EnvironmentUserListItemDto[];
  total: number;
  limit: number;
  offset: number;
}

export interface UserSessionItemDto {
  id: string;
  user_agent: string | null;
  ip_address: string | null;
  active_organization_id: string | null;
  created_at: string;
  last_used_at: string;
  expires_at: string;
  revoked_at: string | null;
}

export interface LinkedOAuthProviderDto {
  provider: string;
  email: string | null;
  linked_at: string;
}

export interface EnvironmentUserDetailDto {
  app_user_id: string;
  sub: string;
  email: string;
  email_verified: boolean;
  display_name: string;
  avatar_url: string | null;
  status: string;
  external_id: string | null;
  public_metadata: Record<string, unknown>;
  private_metadata: Record<string, unknown>;
  version: number;
  joined_at: string;
  last_active_at: string | null;
  updated_at: string;
  roles: string[];
  available_roles: string[];
  linked_providers: LinkedOAuthProviderDto[];
  sessions: UserSessionItemDto[];
  audit_logs: AuditLogEntryDto[];
}

export interface PaginatedAuditLogsDto {
  logs: AuditLogEntryDto[];
  total: number;
  limit: number;
  offset: number;
}

export interface WorkspaceMemberDto {
  developer_id: string;
  email: string;
  display_name: string;
  role: WorkspaceRole | string;
  status: string;
  joined_at: string;
}

export interface WebhookEndpointViewDto {
  id: string;
  environment_id: string;
  url: string;
  description: string;
  subscribed_events: string[];
  status: 'active' | 'paused' | 'disabled' | string;
  consecutive_failures: number;
  disabled_reason: string | null;
  previous_secret_expires_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreatedWebhookEndpointDto {
  endpoint: WebhookEndpointViewDto;
  signing_secret: string;
}

export interface RotatedWebhookSecretDto {
  endpoint: WebhookEndpointViewDto;
  signing_secret: string;
  previous_secret_expires_at: string;
}

export interface WebhookDeliveryDto {
  id: string;
  environment_id: string;
  webhook_endpoint_id: string;
  outbox_event_id: string;
  attempt_number: number;
  request_headers: Record<string, unknown>;
  response_status: number | null;
  response_body_excerpt: string | null;
  latency_ms: number | null;
  status: 'pending' | 'succeeded' | 'retrying' | 'failed' | 'dead_letter' | string;
  next_retry_at: string | null;
  delivered_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface OrganizationDto {
  id: string;
  environment_id: string;
  slug: string;
  name: string;
  logo_url: string | null;
  status: 'active' | 'suspended' | 'deleted' | string;
  metadata: Record<string, unknown>;
  version: number;
  member_count: number;
  created_at: string;
  updated_at: string;
}

export interface PaginatedOrganizationsDto {
  data: OrganizationDto[];
  next_cursor: string | null;
  has_more: boolean;
}

export interface OrganizationMemberDto {
  id: string;
  organization_id: string;
  application_user_id: string;
  sub: string;
  email: string;
  display_name: string;
  status: string;
  roles: string[];
  version: number;
  created_at: string;
  updated_at: string;
}

export interface OrganizationInvitationDto {
  id: string;
  organization_id: string;
  email: string;
  role_slug: string | null;
  status: 'pending' | 'accepted' | 'revoked' | 'expired' | string;
  expires_at: string;
  accepted_at: string | null;
  created_at: string;
}

export interface CreatedInvitationDto {
  invitation: OrganizationInvitationDto;
  invitation_token: string;
}

export interface EnvironmentPermissionDto {
  id: string;
  slug: string;
  description: string;
  created_at: string;
}

export interface EnvironmentRoleDto {
  id: string;
  slug: string;
  name: string;
  scope: 'application' | 'organization' | string;
  description: string;
  is_system: boolean;
  permissions: string[];
  created_at: string;
  updated_at: string;
}

export interface UserEmailDto {
  id: string;
  email: string;
  verified: boolean;
  verified_at: string | null;
  is_primary: boolean;
  created_at: string;
}

export interface LinkedOAuthAccountDto {
  id: string;
  provider: string;
  email: string | null;
  email_verified: boolean;
  created_at: string;
}

export interface IdentityDataExportDto {
  exported_at: string;
  user: Record<string, unknown>;
  emails: UserEmailDto[];
  oauth_accounts: LinkedOAuthAccountDto[];
  memberships: Record<string, unknown>[];
  organizations: Record<string, unknown>[];
  recent_audit_logs: Record<string, unknown>[];
}

export type AnalyticsWindow = '24h' | '7d' | '30d' | '90d';

export interface FailureReasonCountDto {
  reason: string;
  count: number;
}

export interface ProviderBreakdownDto {
  password: number;
  google: number;
  github: number;
  oidc_sso: number;
}

export interface AnalyticsSummaryDto {
  registrations: number;
  dau: number;
  mau: number;
  sign_in_success: number;
  sign_in_failure: number;
  lockouts: number;
  active_sessions: number;
  sessions_created: number;
  sessions_revoked: number;
  refresh_reuse_detected: number;
  client_api_requests: number;
  backend_api_requests: number;
  webhook_deliveries_succeeded: number;
  webhook_deliveries_failed: number;
  webhook_deliveries_dead_letter: number;
  webhook_success_rate: number;
  webhook_avg_latency_ms: number;
  failure_reasons: FailureReasonCountDto[];
}

export interface AnalyticsSeriesPointDto {
  bucket_start: string;
  registrations: number;
  active_users: number;
  sign_in_success: number;
  sign_in_failure: number;
  client_api_requests: number;
  backend_api_requests: number;
  webhook_succeeded: number;
  webhook_failed: number;
}

export interface EnvironmentAnalyticsDto {
  environment_id: string;
  window: AnalyticsWindow | string;
  granularity: 'hour' | 'day' | string;
  generated_at: string;
  summary: AnalyticsSummaryDto;
  provider_breakdown: ProviderBreakdownDto;
  series: AnalyticsSeriesPointDto[];
}
