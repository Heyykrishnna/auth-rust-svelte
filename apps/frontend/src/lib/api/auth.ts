import { apiClient } from './client';

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
	expires_in: number;
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

export const authApi = {
	register: (data: RegisterRequest) =>
		apiClient.post<AuthSession>('/auth/register', data),

	login: (data: LoginRequest) =>
		apiClient.post<AuthSession>('/auth/login', data),

	logout: (token: string) =>
		apiClient.post<void>('/auth/logout', {}, { token }),

	refresh: (data: RefreshRequest) =>
		apiClient.post<AuthTokens>('/auth/refresh', data),

	me: (token: string) =>
		apiClient.get<AuthUser>('/auth/me', { token }),

	getOidcUrl: (provider: 'google' | 'github') =>
		apiClient.get<OidcProvider>(`/auth/oidc/${provider}`),

	oidcCallback: (provider: 'google' | 'github', code: string, state: string) =>
		apiClient.post<AuthSession>(`/auth/oidc/${provider}/callback`, { code, state }),

	health: () => apiClient.get<{ status: string; version: string }>('/health')
};
