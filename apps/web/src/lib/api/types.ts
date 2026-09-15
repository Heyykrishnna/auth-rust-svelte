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

export interface SessionItem {
	id: string;
	user_agent: string | null;
	ip_address: string | null;
	created_at: string;
	last_used_at: string;
	is_current: boolean;
}

export interface RegisterRequest {
	email: string;
	password: string;
	display_name: string;
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
