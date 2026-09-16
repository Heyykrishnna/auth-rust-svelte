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

export interface UserProfile {
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

export interface ApiResponse<T> {
  data?: T;
  error?: string;
}

export interface MessageResponse {
  message: string;
  reset_token?: string;
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

export type Role = 'admin' | 'user' | 'moderator' | string;

export type Permission =
  | 'user:read'
  | 'user:write'
  | 'user:delete'
  | 'session:read'
  | 'session:delete'
  | 'admin:access'
  | string;

export interface UserRole {
  user_id: string;
  role: Role;
}
