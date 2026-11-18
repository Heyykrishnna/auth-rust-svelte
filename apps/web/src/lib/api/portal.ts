import { apiClient, type RequestOptions } from './client';

export interface PortalActiveUser {
	sub: string;
	email: string;
	display_name: string;
	avatar_url: string | null;
}

export interface PortalTransactionContext {
	tx_id: string;
	application_name: string;
	application_slug: string;
	environment_kind: string;
	enrollment_policy: string;
	branding: Record<string, unknown>;
	scopes: string[];
	redirect_uri: string;
	csrf_token: string;
	requires_consent: boolean;
	active_user: PortalActiveUser | null;
	social_providers: string[];
}

export interface PortalInteractionOutcome {
	action: 'redirect' | 'consent';
	redirect_url?: string;
	tx_id?: string;
}

export interface PortalConnectedAppView {
	environment_id: string;
	application_name: string;
	environment_kind: string;
	active_sessions: number;
	joined_at: string;
}

export interface PortalSsoSessionView {
	id: string;
	is_current: boolean;
	user_agent: string | null;
	ip_address: string | null;
	authenticated_at: string;
	last_seen_at: string;
	expires_at: string;
}

export interface PortalAccountView {
	user: PortalActiveUser;
	sso_sessions: PortalSsoSessionView[];
	connected_apps: PortalConnectedAppView[];
}

export interface PortalLogoutResponse {
	logged_out: boolean;
	global_logout: boolean;
	revoked_app_sessions: number;
}

export const portalApi = {
	getTransaction: (txId: string, options?: RequestOptions) =>
		apiClient.get<PortalTransactionContext>(
			`/oauth2/portal/transactions/${encodeURIComponent(txId)}`,
			options
		),

	login: (
		txId: string,
		payload: { csrf_token: string; email: string; password: string },
		options?: RequestOptions
	) =>
		apiClient.post<PortalInteractionOutcome>(
			`/oauth2/portal/transactions/${encodeURIComponent(txId)}/login`,
			payload,
			options
		),

	signUp: (
		txId: string,
		payload: { csrf_token: string; email: string; password: string; display_name: string },
		options?: RequestOptions
	) =>
		apiClient.post<{ status: string; email: string; message: string }>(
			`/oauth2/portal/transactions/${encodeURIComponent(txId)}/sign-up`,
			payload,
			options
		),

	verifySignUp: (
		txId: string,
		payload: { csrf_token: string; email: string; code: string },
		options?: RequestOptions
	) =>
		apiClient.post<PortalInteractionOutcome>(
			`/oauth2/portal/transactions/${encodeURIComponent(txId)}/sign-up/verify`,
			payload,
			options
		),

	submitConsent: (
		txId: string,
		payload: { csrf_token: string; approved: boolean },
		options?: RequestOptions
	) =>
		apiClient.post<PortalInteractionOutcome>(
			`/oauth2/portal/transactions/${encodeURIComponent(txId)}/consent`,
			payload,
			options
		),

	getAccount: (options?: RequestOptions) =>
		apiClient.get<PortalAccountView>('/oauth2/portal/account', options),

	revokeSsoSession: (ssoId: string, options?: RequestOptions) =>
		apiClient.post<void>(`/oauth2/portal/sessions/${encodeURIComponent(ssoId)}/revoke`, undefined, options),

	logout: (
		payload: {
			global: boolean;
			id_token_hint?: string;
			post_logout_redirect_uri?: string;
			state?: string;
		},
		options?: RequestOptions
	) => apiClient.post<PortalLogoutResponse>('/oauth2/logout', payload, options)
};

