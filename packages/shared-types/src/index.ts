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
