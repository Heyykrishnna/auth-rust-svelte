// ─── Auth API ─────────────────────────────────────────────────────────────────
// All auth-related API calls to the Rust auth-api service

import { apiClient } from './client';

// ─── Types ────────────────────────────────────────────────────────────────────

export interface AuthUser {
	id: string;
	email: string;
	display_name: string;
	avatar_url: string | null;
	email_verified: boolean;
	created_at: string;
}

export interface AuthTokens {
	access_token: string;
	refresh_token: string;
	token_type: 'Bearer';
	expires_in: number; // seconds
}

export interface AuthSession {
	user: AuthUser;
	tokens: AuthTokens;
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
	refresh_token: string;
}

export interface OidcProvider {
	provider: 'google' | 'github';
	authorization_url: string;
}

// ─── Auth API Methods ──────────────────────────────────────────────────────────

export const authApi = {
	/**
	 * Register a new user with email and password.
	 */
	register: (data: RegisterRequest) =>
		apiClient.post<AuthSession>('/auth/register', data),

	/**
	 * Login with email and password.
	 */
	login: (data: LoginRequest) =>
		apiClient.post<AuthSession>('/auth/login', data),

	/**
	 * Logout the current session.
	 */
	logout: (token: string) =>
		apiClient.post<void>('/auth/logout', {}, { token }),

	/**
	 * Refresh the access token using a refresh token.
	 */
	refresh: (data: RefreshRequest) =>
		apiClient.post<AuthTokens>('/auth/refresh', data),

	/**
	 * Get the current authenticated user's profile.
	 */
	me: (token: string) =>
		apiClient.get<AuthUser>('/auth/me', { token }),

	/**
	 * Get OIDC authorization URL for a provider.
	 */
	getOidcUrl: (provider: 'google' | 'github') =>
		apiClient.get<OidcProvider>(`/auth/oidc/${provider}`),

	/**
	 * Handle OIDC callback with authorization code.
	 */
	oidcCallback: (provider: 'google' | 'github', code: string, state: string) =>
		apiClient.post<AuthSession>(`/auth/oidc/${provider}/callback`, { code, state }),

	/**
	 * Health check for the auth API.
	 */
	health: () => apiClient.get<{ status: string; version: string }>('/health')
};
