import { apiClient } from './client';

export interface AuthUser {
	id: string;
	email: string;
	display_name: string;
	avatar_url: string | null;
	email_verified: boolean;
	status: string;
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
	tokens?: AuthTokens;
	message?: string;
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

export interface OidcProvider {
	provider: 'google' | 'github';
	authorization_url: string;
}

export interface SessionItem {
	id: string;
	user_agent: string | null;
	ip_address: string | null;
	created_at: string;
	last_used_at: string;
	is_current: boolean;
}

export interface UpdateProfileRequest {
	display_name: string;
	avatar_url?: string | null;
}

export const authApi = {
	register: (data: RegisterRequest) =>
		apiClient.post<AuthSession>('/api/auth/register', data),

	login: (data: LoginRequest) =>
		apiClient.post<AuthSession>('/api/auth/login', data),

	logout: () =>
		apiClient.post<void>('/api/auth/logout', {}),

	refresh: (data?: RefreshRequest) =>
		apiClient.post<AuthTokens>('/api/auth/refresh', data ?? {}),

	me: () =>
		apiClient.get<AuthUser>('/api/auth/me'),

	updateProfile: (data: UpdateProfileRequest) =>
		apiClient.put<AuthUser>('/api/users/profile', data),

	listSessions: () =>
		apiClient.get<SessionItem[]>('/api/sessions'),

	revokeSession: (sessionId: string) =>
		apiClient.delete<void>(`/api/sessions/${sessionId}`),

	getOidcUrl: (provider: 'google' | 'github') =>
		apiClient.get<OidcProvider>(`/api/auth/oidc/${provider}`),

	oidcCallback: (provider: 'google' | 'github', code: string, state: string) =>
		apiClient.post<AuthSession>(`/api/auth/oidc/${provider}/callback`, { code, state }),

	health: () => apiClient.get<{ status: string; version: string }>('/health')
};

