import { base64UrlToBytes, bytesToUtf8, randomBase64Url, sha256Base64Url } from './encoding.js';
import { AuthSdkError, TokenVerificationError } from './errors.js';
import { fetchWithPolicy, parseJsonResponse, type FetchLike } from './http.js';
import type {
	ActiveOrganizationSwitchResponse,
	ClientAuthResponse,
	ClientSessionView,
	ClientSignUpInitiatedResponse,
	ClientTokenRefreshResponse,
	ClientUser,
	CreatedInvitationResponse,
	IdentityDataExportResponse,
	LinkedOAuthAccountView,
	OidcTokenResponse,
	OrganizationInvitationView,
	OrganizationMemberView,
	OrganizationView,
	UserEmailView,
	UserOrganizationMembership
} from './types.js';

export type {
	ActiveOrganizationSwitchResponse,
	ClientAuthResponse,
	ClientSessionView,
	ClientSignUpInitiatedResponse,
	ClientTokenRefreshResponse,
	ClientUser,
	CreatedInvitationResponse,
	FetchLike,
	IdentityDataExportResponse,
	LinkedOAuthAccountView,
	OidcTokenResponse,
	OrganizationInvitationView,
	OrganizationMemberView,
	OrganizationView,
	UserEmailView,
	UserOrganizationMembership
};

export interface StorageAdapter {
	getItem(key: string): string | null;
	setItem(key: string, value: string): void;
	removeItem(key: string): void;
}

export interface AuthClientConfig {
	publishableKey: string;
	issuerUrl: string;
	clientId?: string;
	redirectUri?: string;
	fetch?: FetchLike;
	storage?: StorageAdapter;
	timeoutMs?: number;
	maxRetries?: number;
}

export interface AuthSessionSnapshot {
	accessToken: string | null;
	refreshToken: string | null;
	idToken: string | null;
	sessionId: string | null;
	expiresAt: number | null;
	user: ClientUser | null;
	activeOrganizationId: string | null;
	orgSlug: string | null;
	orgRole: string | null;
	orgPermissions: string[];
}

export interface RedirectSignInOptions {
	redirectUri?: string;
	scope?: string;
	prompt?: 'none' | 'login' | 'consent';
	maxAge?: number;
}

export interface SignOutOptions {
	global?: boolean;
	idTokenHint?: string;
	postLogoutRedirectUri?: string;
	state?: string;
}

interface StoredPkceTransaction {
	state: string;
	nonce: string;
	codeVerifier: string;
	redirectUri: string;
}

const PKCE_STORAGE_KEY = 'secure_auth_pkce_tx';

class MemoryStorage implements StorageAdapter {
	private readonly store = new Map<string, string>();

	getItem(key: string): string | null {
		return this.store.get(key) ?? null;
	}

	setItem(key: string, value: string): void {
		this.store.set(key, value);
	}

	removeItem(key: string): void {
		this.store.delete(key);
	}
}

function defaultStorage(): StorageAdapter {
	if (typeof globalThis.sessionStorage !== 'undefined') {
		return globalThis.sessionStorage;
	}
	return new MemoryStorage();
}

function decodeJwtPayloadUnverified(token: string): Record<string, unknown> {
	const parts = token.split('.');
	if (parts.length !== 3 || !parts[1]) {
		throw new TokenVerificationError('Malformed JWT');
	}
	try {
		return JSON.parse(bytesToUtf8(base64UrlToBytes(parts[1]))) as Record<string, unknown>;
	} catch {
		throw new TokenVerificationError('Invalid JWT payload encoding');
	}
}

export interface AuthClient {
	buildAuthorizeUrl(options?: RedirectSignInOptions): Promise<{
		url: string;
		state: string;
		nonce: string;
		codeVerifier: string;
	}>;
	signInWithRedirect(options?: RedirectSignInOptions): Promise<string>;
	handleRedirectCallback(callbackUrlOrParams: string | URLSearchParams): Promise<OidcTokenResponse>;
	signInWithPassword(input: { email: string; password: string }): Promise<ClientAuthResponse>;
	signUp(input: {
		email: string;
		password: string;
		displayName: string;
	}): Promise<ClientSignUpInitiatedResponse>;
	verifyOtp(input: { email: string; code: string }): Promise<ClientAuthResponse>;
	resendOtp(email: string): Promise<{ message: string }>;
	requestPasswordReset(email: string): Promise<{ message: string }>;
	completePasswordReset(input: { token: string; newPassword: string }): Promise<{ message: string }>;
	refreshSession(overrideRefreshToken?: string): Promise<ClientTokenRefreshResponse>;
	signOut(options?: SignOutOptions): Promise<{ logoutUrl?: string }>;
	getUser(): Promise<ClientUser>;
	updateProfile(input: { displayName: string; avatarUrl?: string | null }): Promise<ClientUser>;
	listSessions(): Promise<ClientSessionView[]>;
	revokeSession(sessionId: string): Promise<void>;
	listOrganizations(): Promise<UserOrganizationMembership[]>;
	createOrganization(input: {
		name: string;
		slug: string;
		logoUrl?: string | null;
		metadata?: Record<string, unknown>;
	}): Promise<OrganizationView>;
	getOrganization(orgId: string): Promise<OrganizationView>;
	updateOrganization(
		orgId: string,
		input: {
			name?: string;
			slug?: string;
			logoUrl?: string | null;
			status?: string;
			metadata?: Record<string, unknown>;
		}
	): Promise<OrganizationView>;
	deleteOrganization(orgId: string): Promise<void>;
	listOrganizationMembers(orgId: string): Promise<OrganizationMemberView[]>;
	updateOrganizationMemberRole(
		orgId: string,
		memberId: string,
		roleSlug: string
	): Promise<OrganizationMemberView>;
	removeOrganizationMember(orgId: string, memberId: string): Promise<void>;
	listOrganizationInvitations(orgId: string): Promise<OrganizationInvitationView[]>;
	createOrganizationInvitation(
		orgId: string,
		input: { email: string; roleSlug?: string; expiresInDays?: number }
	): Promise<CreatedInvitationResponse>;
	revokeOrganizationInvitation(orgId: string, invitationId: string): Promise<void>;
	acceptOrganizationInvitation(token: string): Promise<OrganizationMemberView>;
	switchActiveOrganization(organizationId: string | null): Promise<ActiveOrganizationSwitchResponse>;
	listEmails(): Promise<UserEmailView[]>;
	addEmail(email: string): Promise<UserEmailView>;
	verifyEmailAddress(input: { email: string; code: string }): Promise<UserEmailView>;
	setPrimaryEmail(emailId: string): Promise<UserEmailView>;
	removeEmail(emailId: string): Promise<void>;
	listOAuthAccounts(): Promise<LinkedOAuthAccountView[]>;
	unlinkOAuthAccount(provider: string): Promise<void>;
	exportAccountData(): Promise<IdentityDataExportResponse>;
	deleteAccount(): Promise<{ deleted: boolean; notified_environments: number }>;
	getSession(): AuthSessionSnapshot;
	setSession(snapshot: Partial<AuthSessionSnapshot>): void;
	subscribe(listener: (snapshot: AuthSessionSnapshot) => void): () => void;
}

export function createAuthClient(config: AuthClientConfig): AuthClient {
	const baseUrl = config.issuerUrl.replace(/\/+$/, '');
	const clientId = config.clientId ?? config.publishableKey;
	const fetchImpl: FetchLike = config.fetch ?? globalThis.fetch.bind(globalThis);
	const storage = config.storage ?? defaultStorage();
	const timeoutMs = config.timeoutMs ?? 10_000;
	const maxRetries = config.maxRetries ?? 2;

	let state: AuthSessionSnapshot = {
		accessToken: null,
		refreshToken: null,
		idToken: null,
		sessionId: null,
		expiresAt: null,
		user: null,
		activeOrganizationId: null,
		orgSlug: null,
		orgRole: null,
		orgPermissions: []
	};

	const listeners = new Set<(snapshot: AuthSessionSnapshot) => void>();
	let inFlightRefresh: Promise<ClientTokenRefreshResponse> | null = null;

	function notify(): void {
		const copy: AuthSessionSnapshot = { ...state };
		for (const listener of listeners) {
			listener(copy);
		}
	}

	function updateState(patch: Partial<AuthSessionSnapshot>): void {
		state = { ...state, ...patch };
		notify();
	}

	function baseHeaders(includeAuth = false): Record<string, string> {
		const headers: Record<string, string> = {
			'x-publishable-key': config.publishableKey,
			'content-type': 'application/json'
		};
		if (includeAuth && state.accessToken) {
			headers['authorization'] = `Bearer ${state.accessToken}`;
		}
		return headers;
	}

	async function buildAuthorizeUrl(options: RedirectSignInOptions = {}) {
		const redirectUri = options.redirectUri ?? config.redirectUri;
		if (!redirectUri) {
			throw new AuthSdkError('CONFIG_ERROR', 'redirectUri is required for OIDC redirect flow');
		}

		const codeVerifier = randomBase64Url(32);
		const codeChallenge = await sha256Base64Url(codeVerifier);
		const oauthState = randomBase64Url(24);
		const nonce = randomBase64Url(24);

		const tx: StoredPkceTransaction = {
			state: oauthState,
			nonce,
			codeVerifier,
			redirectUri
		};
		storage.setItem(PKCE_STORAGE_KEY, JSON.stringify(tx));

		const params = new URLSearchParams({
			client_id: clientId,
			redirect_uri: redirectUri,
			response_type: 'code',
			scope: options.scope ?? 'openid profile email offline_access',
			state: oauthState,
			nonce,
			code_challenge: codeChallenge,
			code_challenge_method: 'S256'
		});
		if (options.prompt) {
			params.set('prompt', options.prompt);
		}
		if (typeof options.maxAge === 'number') {
			params.set('max_age', String(options.maxAge));
		}

		return {
			url: `${baseUrl}/oauth2/authorize?${params.toString()}`,
			state: oauthState,
			nonce,
			codeVerifier
		};
	}

	async function signInWithRedirect(options: RedirectSignInOptions = {}): Promise<string> {
		const { url } = await buildAuthorizeUrl(options);
		if (typeof globalThis.window !== 'undefined' && globalThis.window.location) {
			globalThis.window.location.assign(url);
		}
		return url;
	}

	async function handleRedirectCallback(
		callbackUrlOrParams: string | URLSearchParams
	): Promise<OidcTokenResponse> {
		const params =
			typeof callbackUrlOrParams === 'string'
				? new URL(callbackUrlOrParams).searchParams
				: callbackUrlOrParams;

		const error = params.get('error');
		if (error) {
			throw new AuthSdkError(
				error.toUpperCase(),
				params.get('error_description') ?? `OIDC authorization error: ${error}`
			);
		}

		const code = params.get('code');
		const returnedState = params.get('state');
		const rawStored = storage.getItem(PKCE_STORAGE_KEY);
		storage.removeItem(PKCE_STORAGE_KEY);

		if (!rawStored) {
			throw new AuthSdkError('OIDC_STATE_MISSING', 'Missing stored PKCE authorization transaction');
		}
		const stored = JSON.parse(rawStored) as StoredPkceTransaction;
		if (!code || !returnedState || returnedState !== stored.state) {
			throw new AuthSdkError('OIDC_STATE_MISMATCH', 'Invalid or mismatched OIDC state parameter');
		}

		const body = new URLSearchParams({
			grant_type: 'authorization_code',
			client_id: clientId,
			code,
			redirect_uri: stored.redirectUri,
			code_verifier: stored.codeVerifier
		});

		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/oauth2/token`, {
			method: 'POST',
			headers: { 'content-type': 'application/x-www-form-urlencoded' },
			body: body.toString(),
			timeoutMs,
			idempotent: false
		});
		const tokens = await parseJsonResponse<OidcTokenResponse>(res);

		if (tokens.id_token) {
			const idPayload = decodeJwtPayloadUnverified(tokens.id_token);
			if (idPayload['nonce'] !== stored.nonce) {
				throw new TokenVerificationError('ID token nonce mismatch', 'INVALID_NONCE');
			}
		}

		const accessPayload = decodeJwtPayloadUnverified(tokens.access_token);
		const sid = typeof accessPayload['sid'] === 'string' ? accessPayload['sid'] : null;

		updateState({
			accessToken: tokens.access_token,
			refreshToken: tokens.refresh_token ?? null,
			idToken: tokens.id_token ?? null,
			sessionId: sid,
			expiresAt: Math.floor(Date.now() / 1000) + tokens.expires_in
		});

		return tokens;
	}

	async function signInWithPassword(input: {
		email: string;
		password: string;
	}): Promise<ClientAuthResponse> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/auth/sign-in`, {
			method: 'POST',
			headers: baseHeaders(false),
			body: JSON.stringify(input),
			timeoutMs,
			idempotent: false
		});
		const data = await parseJsonResponse<ClientAuthResponse>(res);
		updateState({
			accessToken: data.access_token,
			refreshToken: data.refresh_token,
			sessionId: data.session_id,
			expiresAt: Math.floor(Date.now() / 1000) + data.expires_in,
			user: data.user
		});
		return data;
	}

	async function signUp(input: {
		email: string;
		password: string;
		displayName: string;
	}): Promise<ClientSignUpInitiatedResponse> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/auth/sign-up`, {
			method: 'POST',
			headers: baseHeaders(false),
			body: JSON.stringify({
				email: input.email,
				password: input.password,
				display_name: input.displayName
			}),
			timeoutMs,
			idempotent: false
		});
		return parseJsonResponse<ClientSignUpInitiatedResponse>(res);
	}

	async function verifyOtp(input: { email: string; code: string }): Promise<ClientAuthResponse> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/auth/sign-up/verify`, {
			method: 'POST',
			headers: baseHeaders(false),
			body: JSON.stringify(input),
			timeoutMs,
			idempotent: false
		});
		const data = await parseJsonResponse<ClientAuthResponse>(res);
		updateState({
			accessToken: data.access_token,
			refreshToken: data.refresh_token,
			sessionId: data.session_id,
			expiresAt: Math.floor(Date.now() / 1000) + data.expires_in,
			user: data.user
		});
		return data;
	}

	async function resendOtp(email: string): Promise<{ message: string }> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/auth/sign-up/resend-otp`, {
			method: 'POST',
			headers: baseHeaders(false),
			body: JSON.stringify({ email }),
			timeoutMs,
			idempotent: false
		});
		return parseJsonResponse<{ message: string }>(res);
	}

	async function requestPasswordReset(email: string): Promise<{ message: string }> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/auth/password/forgot`, {
			method: 'POST',
			headers: baseHeaders(false),
			body: JSON.stringify({ email }),
			timeoutMs,
			idempotent: false
		});
		return parseJsonResponse<{ message: string }>(res);
	}

	async function completePasswordReset(input: {
		token: string;
		newPassword: string;
	}): Promise<{ message: string }> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/auth/password/reset`, {
			method: 'POST',
			headers: baseHeaders(false),
			body: JSON.stringify({ token: input.token, new_password: input.newPassword }),
			timeoutMs,
			idempotent: false
		});
		return parseJsonResponse<{ message: string }>(res);
	}

	async function refreshSession(
		overrideRefreshToken?: string
	): Promise<ClientTokenRefreshResponse> {
		if (inFlightRefresh) {
			return inFlightRefresh;
		}
		const tokenToUse = overrideRefreshToken ?? state.refreshToken;
		if (!tokenToUse) {
			throw new AuthSdkError('MISSING_REFRESH_TOKEN', 'No refresh token available');
		}

		const promise = (async () => {
			const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/auth/refresh`, {
				method: 'POST',
				headers: baseHeaders(false),
				body: JSON.stringify({ refresh_token: tokenToUse }),
				timeoutMs,
				idempotent: false
			});
			const data = await parseJsonResponse<ClientTokenRefreshResponse>(res);
			updateState({
				accessToken: data.access_token,
				refreshToken: data.refresh_token,
				sessionId: data.session_id,
				expiresAt: Math.floor(Date.now() / 1000) + data.expires_in
			});
			return data;
		})();

		inFlightRefresh = promise;
		try {
			return await promise;
		} finally {
			inFlightRefresh = null;
		}
	}

	async function signOut(options: SignOutOptions = {}): Promise<{ logoutUrl?: string }> {
		const currentRefresh = state.refreshToken;
		const currentAccess = state.accessToken;
		const currentIdToken = options.idTokenHint ?? state.idToken ?? undefined;

		if (options.global) {
			const params = new URLSearchParams({ global: 'true' });
			if (currentIdToken) params.set('id_token_hint', currentIdToken);
			if (options.postLogoutRedirectUri) {
				params.set('post_logout_redirect_uri', options.postLogoutRedirectUri);
			}
			if (options.state) params.set('state', options.state);
			updateState({
				accessToken: null,
				refreshToken: null,
				idToken: null,
				sessionId: null,
				expiresAt: null,
				user: null,
				activeOrganizationId: null,
				orgSlug: null,
				orgRole: null,
				orgPermissions: []
			});
			const logoutUrl = `${baseUrl}/oauth2/logout?${params.toString()}`;
			if (typeof globalThis.window !== 'undefined' && globalThis.window.location) {
				globalThis.window.location.assign(logoutUrl);
			}
			return { logoutUrl };
		}

		if (currentAccess) {
			await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/auth/sign-out`, {
				method: 'POST',
				headers: baseHeaders(true),
				timeoutMs,
				idempotent: false
			}).catch(() => undefined);
		} else if (currentRefresh) {
			const body = new URLSearchParams({ token: currentRefresh, client_id: clientId });
			await fetchWithPolicy(fetchImpl, `${baseUrl}/oauth2/revoke`, {
				method: 'POST',
				headers: { 'content-type': 'application/x-www-form-urlencoded' },
				body: body.toString(),
				timeoutMs,
				idempotent: false
			}).catch(() => undefined);
		}

		updateState({
			accessToken: null,
			refreshToken: null,
			idToken: null,
			sessionId: null,
			expiresAt: null,
			user: null,
			activeOrganizationId: null,
			orgSlug: null,
			orgRole: null,
			orgPermissions: []
		});
		return {};
	}

	async function getUser(): Promise<ClientUser> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/users/me`, {
			method: 'GET',
			headers: baseHeaders(true),
			timeoutMs,
			maxRetries,
			idempotent: true
		});
		const user = await parseJsonResponse<ClientUser>(res);
		updateState({ user });
		return user;
	}

	async function updateProfile(input: {
		displayName: string;
		avatarUrl?: string | null;
	}): Promise<ClientUser> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/users/me`, {
			method: 'PATCH',
			headers: baseHeaders(true),
			body: JSON.stringify({
				display_name: input.displayName,
				avatar_url: input.avatarUrl ?? null
			}),
			timeoutMs,
			idempotent: false
		});
		const user = await parseJsonResponse<ClientUser>(res);
		updateState({ user });
		return user;
	}

	async function listSessions(): Promise<ClientSessionView[]> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/sessions`, {
			method: 'GET',
			headers: baseHeaders(true),
			timeoutMs,
			maxRetries,
			idempotent: true
		});
		return parseJsonResponse<ClientSessionView[]>(res);
	}

	async function revokeSession(sessionId: string): Promise<void> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/sessions/${encodeURIComponent(sessionId)}`,
			{
				method: 'DELETE',
				headers: baseHeaders(true),
				timeoutMs,
				idempotent: false
			}
		);
		if (!res.ok) {
			await parseJsonResponse<unknown>(res);
		}
	}

	async function listOrganizations(): Promise<UserOrganizationMembership[]> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/organizations`, {
			method: 'GET',
			headers: baseHeaders(true),
			timeoutMs,
			maxRetries,
			idempotent: true
		});
		return parseJsonResponse<UserOrganizationMembership[]>(res);
	}

	async function createOrganization(input: {
		name: string;
		slug: string;
		logoUrl?: string | null;
		metadata?: Record<string, unknown>;
	}): Promise<OrganizationView> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/organizations`, {
			method: 'POST',
			headers: baseHeaders(true),
			body: JSON.stringify({
				name: input.name,
				slug: input.slug,
				logo_url: input.logoUrl ?? null,
				metadata: input.metadata
			}),
			timeoutMs,
			idempotent: false
		});
		return parseJsonResponse<OrganizationView>(res);
	}

	async function getOrganization(orgId: string): Promise<OrganizationView> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/organizations/${encodeURIComponent(orgId)}`,
			{
				method: 'GET',
				headers: baseHeaders(true),
				timeoutMs,
				maxRetries,
				idempotent: true
			}
		);
		return parseJsonResponse<OrganizationView>(res);
	}

	async function updateOrganization(
		orgId: string,
		input: {
			name?: string;
			slug?: string;
			logoUrl?: string | null;
			status?: string;
			metadata?: Record<string, unknown>;
		}
	): Promise<OrganizationView> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/organizations/${encodeURIComponent(orgId)}`,
			{
				method: 'PATCH',
				headers: baseHeaders(true),
				body: JSON.stringify({
					name: input.name,
					slug: input.slug,
					logo_url: input.logoUrl,
					status: input.status,
					metadata: input.metadata
				}),
				timeoutMs,
				idempotent: false
			}
		);
		return parseJsonResponse<OrganizationView>(res);
	}

	async function deleteOrganization(orgId: string): Promise<void> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/organizations/${encodeURIComponent(orgId)}`,
			{
				method: 'DELETE',
				headers: baseHeaders(true),
				timeoutMs,
				idempotent: false
			}
		);
		if (!res.ok) {
			await parseJsonResponse<unknown>(res);
		}
	}

	async function listOrganizationMembers(orgId: string): Promise<OrganizationMemberView[]> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/organizations/${encodeURIComponent(orgId)}/members`,
			{
				method: 'GET',
				headers: baseHeaders(true),
				timeoutMs,
				maxRetries,
				idempotent: true
			}
		);
		return parseJsonResponse<OrganizationMemberView[]>(res);
	}

	async function updateOrganizationMemberRole(
		orgId: string,
		memberId: string,
		roleSlug: string
	): Promise<OrganizationMemberView> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/organizations/${encodeURIComponent(orgId)}/members/${encodeURIComponent(memberId)}`,
			{
				method: 'PATCH',
				headers: baseHeaders(true),
				body: JSON.stringify({ role_slug: roleSlug }),
				timeoutMs,
				idempotent: false
			}
		);
		return parseJsonResponse<OrganizationMemberView>(res);
	}

	async function removeOrganizationMember(orgId: string, memberId: string): Promise<void> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/organizations/${encodeURIComponent(orgId)}/members/${encodeURIComponent(memberId)}`,
			{
				method: 'DELETE',
				headers: baseHeaders(true),
				timeoutMs,
				idempotent: false
			}
		);
		if (!res.ok) {
			await parseJsonResponse<unknown>(res);
		}
	}

	async function listOrganizationInvitations(orgId: string): Promise<OrganizationInvitationView[]> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/organizations/${encodeURIComponent(orgId)}/invitations`,
			{
				method: 'GET',
				headers: baseHeaders(true),
				timeoutMs,
				maxRetries,
				idempotent: true
			}
		);
		return parseJsonResponse<OrganizationInvitationView[]>(res);
	}

	async function createOrganizationInvitation(
		orgId: string,
		input: { email: string; roleSlug?: string; expiresInDays?: number }
	): Promise<CreatedInvitationResponse> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/organizations/${encodeURIComponent(orgId)}/invitations`,
			{
				method: 'POST',
				headers: baseHeaders(true),
				body: JSON.stringify({
					email: input.email,
					role_slug: input.roleSlug,
					expires_in_days: input.expiresInDays
				}),
				timeoutMs,
				idempotent: false
			}
		);
		return parseJsonResponse<CreatedInvitationResponse>(res);
	}

	async function revokeOrganizationInvitation(orgId: string, invitationId: string): Promise<void> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/organizations/${encodeURIComponent(orgId)}/invitations/${encodeURIComponent(invitationId)}`,
			{
				method: 'DELETE',
				headers: baseHeaders(true),
				timeoutMs,
				idempotent: false
			}
		);
		if (!res.ok) {
			await parseJsonResponse<unknown>(res);
		}
	}

	async function acceptOrganizationInvitation(token: string): Promise<OrganizationMemberView> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/organizations/invitations/accept`,
			{
				method: 'POST',
				headers: baseHeaders(true),
				body: JSON.stringify({ token }),
				timeoutMs,
				idempotent: false
			}
		);
		return parseJsonResponse<OrganizationMemberView>(res);
	}

	async function switchActiveOrganization(
		organizationId: string | null
	): Promise<ActiveOrganizationSwitchResponse> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/sessions/active-organization`,
			{
				method: 'POST',
				headers: baseHeaders(true),
				body: JSON.stringify({ organization_id: organizationId }),
				timeoutMs,
				idempotent: false
			}
		);
		const switched = await parseJsonResponse<ActiveOrganizationSwitchResponse>(res);
		updateState({
			accessToken: switched.access_token,
			expiresAt: Date.now() + switched.expires_in * 1000,
			activeOrganizationId: switched.active_organization_id,
			orgSlug: switched.org_slug,
			orgRole: switched.org_role,
			orgPermissions: switched.org_permissions
		});
		return switched;
	}

	async function listEmails(): Promise<UserEmailView[]> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/users/me/emails`, {
			method: 'GET',
			headers: baseHeaders(true),
			timeoutMs,
			maxRetries,
			idempotent: true
		});
		return parseJsonResponse<UserEmailView[]>(res);
	}

	async function addEmail(email: string): Promise<UserEmailView> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/users/me/emails`, {
			method: 'POST',
			headers: baseHeaders(true),
			body: JSON.stringify({ email }),
			timeoutMs,
			idempotent: false
		});
		return parseJsonResponse<UserEmailView>(res);
	}

	async function verifyEmailAddress(input: {
		email: string;
		code: string;
	}): Promise<UserEmailView> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/users/me/emails/verify`, {
			method: 'POST',
			headers: baseHeaders(true),
			body: JSON.stringify(input),
			timeoutMs,
			idempotent: false
		});
		return parseJsonResponse<UserEmailView>(res);
	}

	async function setPrimaryEmail(emailId: string): Promise<UserEmailView> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/users/me/emails/${encodeURIComponent(emailId)}/primary`,
			{
				method: 'POST',
				headers: baseHeaders(true),
				timeoutMs,
				idempotent: false
			}
		);
		return parseJsonResponse<UserEmailView>(res);
	}

	async function removeEmail(emailId: string): Promise<void> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/users/me/emails/${encodeURIComponent(emailId)}`,
			{
				method: 'DELETE',
				headers: baseHeaders(true),
				timeoutMs,
				idempotent: false
			}
		);
		if (!res.ok) {
			await parseJsonResponse<unknown>(res);
		}
	}

	async function listOAuthAccounts(): Promise<LinkedOAuthAccountView[]> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/users/me/oauth-accounts`, {
			method: 'GET',
			headers: baseHeaders(true),
			timeoutMs,
			maxRetries,
			idempotent: true
		});
		return parseJsonResponse<LinkedOAuthAccountView[]>(res);
	}

	async function unlinkOAuthAccount(provider: string): Promise<void> {
		const res = await fetchWithPolicy(
			fetchImpl,
			`${baseUrl}/v1/client/users/me/oauth-accounts/${encodeURIComponent(provider)}`,
			{
				method: 'DELETE',
				headers: baseHeaders(true),
				timeoutMs,
				idempotent: false
			}
		);
		if (!res.ok) {
			await parseJsonResponse<unknown>(res);
		}
	}

	async function exportAccountData(): Promise<IdentityDataExportResponse> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/users/me/export`, {
			method: 'GET',
			headers: baseHeaders(true),
			timeoutMs,
			maxRetries,
			idempotent: true
		});
		return parseJsonResponse<IdentityDataExportResponse>(res);
	}

	async function deleteAccount(): Promise<{ deleted: boolean; notified_environments: number }> {
		const res = await fetchWithPolicy(fetchImpl, `${baseUrl}/v1/client/users/me`, {
			method: 'DELETE',
			headers: baseHeaders(true),
			timeoutMs,
			idempotent: false
		});
		const result = await parseJsonResponse<{ deleted: boolean; notified_environments: number }>(
			res
		);
		updateState({
			accessToken: null,
			refreshToken: null,
			idToken: null,
			sessionId: null,
			expiresAt: null,
			user: null,
			activeOrganizationId: null,
			orgSlug: null,
			orgRole: null,
			orgPermissions: []
		});
		return result;
	}

	return {
		buildAuthorizeUrl,
		signInWithRedirect,
		handleRedirectCallback,
		signInWithPassword,
		signUp,
		verifyOtp,
		resendOtp,
		requestPasswordReset,
		completePasswordReset,
		refreshSession,
		signOut,
		getUser,
		updateProfile,
		listSessions,
		revokeSession,
		listOrganizations,
		createOrganization,
		getOrganization,
		updateOrganization,
		deleteOrganization,
		listOrganizationMembers,
		updateOrganizationMemberRole,
		removeOrganizationMember,
		listOrganizationInvitations,
		createOrganizationInvitation,
		revokeOrganizationInvitation,
		acceptOrganizationInvitation,
		switchActiveOrganization,
		listEmails,
		addEmail,
		verifyEmailAddress,
		setPrimaryEmail,
		removeEmail,
		listOAuthAccounts,
		unlinkOAuthAccount,
		exportAccountData,
		deleteAccount,
		getSession: () => ({ ...state }),
		setSession: (patch) => updateState(patch),
		subscribe: (listener) => {
			listeners.add(listener);
			listener({ ...state });
			return () => {
				listeners.delete(listener);
			};
		}
	};
}
