import { base64UrlToBytes, bytesToUtf8, verifyEs256SignatureWithJwk } from './encoding.js';
import { AuthSdkError, TokenExpiredError, TokenVerificationError } from './errors.js';
import { fetchWithPolicy, parseJsonResponse, type FetchLike } from './http.js';
import type {
	BackendSessionStatus,
	BackendUserSyncItem,
	EcPublicJwk,
	EnvironmentPermissionView,
	EnvironmentRoleView,
	IdTokenClaims,
	JwksDocument,
	OrganizationMemberView,
	OrganizationView,
	PaginatedBackendEvents,
	PaginatedBackendOrganizations,
	PaginatedBackendUsers,
	PermissionCheckInput,
	TenantAccessTokenClaims
} from './types.js';

export type {
	BackendSessionStatus,
	BackendUserSyncItem,
	EcPublicJwk,
	EnvironmentPermissionView,
	EnvironmentRoleView,
	FetchLike,
	IdTokenClaims,
	JwksDocument,
	OrganizationMemberView,
	OrganizationView,
	PaginatedBackendEvents,
	PaginatedBackendOrganizations,
	PaginatedBackendUsers,
	PermissionCheckInput,
	TenantAccessTokenClaims
};

export function hasPermission(
	claims: Pick<
		TenantAccessTokenClaims,
		'roles' | 'permissions' | 'org_id' | 'org_role' | 'org_permissions'
	> | null,
	check: PermissionCheckInput
): boolean {
	if (!claims) {
		return false;
	}
	if (check.orgId && claims.org_id !== check.orgId) {
		if (check.permission && claims.permissions.includes(check.permission)) {
			if (!check.role || claims.roles.includes(check.role)) {
				return true;
			}
		}
		if (check.role && !check.permission && claims.roles.includes(check.role)) {
			return true;
		}
		return false;
	}

	const activeOrgMatches = !check.orgId || claims.org_id === check.orgId;
	const orgPermissions = activeOrgMatches && claims.org_id ? (claims.org_permissions ?? []) : [];

	if (check.permission) {
		const allowed =
			claims.permissions.includes(check.permission) || orgPermissions.includes(check.permission);
		if (!allowed) {
			return false;
		}
	}

	if (check.role) {
		const roleAllowed =
			claims.roles.includes(check.role) || (activeOrgMatches && claims.org_role === check.role);
		if (!roleAllowed) {
			return false;
		}
	}

	return Boolean(check.permission || check.role);
}

export interface AuthServerConfig {
	issuerUrl: string;
	audience: string;
	secretKey?: string;
	publishableKey?: string;
	jwksUri?: string;
	jwksTtlMs?: number;
	minJwksRefreshIntervalMs?: number;
	clockToleranceSecs?: number;
	timeoutMs?: number;
	fetch?: FetchLike;
}

export interface VerifyIdTokenOptions {
	expectedNonce?: string;
	nowSecs?: number;
}

export interface VerifyAccessTokenOptions {
	nowSecs?: number;
}

export interface ListUsersParams {
	updatedSince?: string | Date;
	cursor?: string;
	limit?: number;
}

export interface ListOrganizationsParams {
	updatedSince?: string | Date;
	cursor?: string;
	q?: string;
	limit?: number;
}

export interface ListEventsParams {
	afterEventId?: string;
	types?: string[];
	limit?: number;
}

export interface UpdateUserMetadataParams {
	externalId?: string | null;
	publicMetadata?: Record<string, unknown>;
	privateMetadata?: Record<string, unknown>;
}

interface JwtHeader {
	alg?: string;
	typ?: string;
	kid?: string;
}

function decodeJwtParts(token: string): {
	header: JwtHeader;
	payload: Record<string, unknown>;
	signingInput: string;
	signatureBytes: Uint8Array;
} {
	const parts = token.trim().split('.');
	if (parts.length !== 3 || !parts[0] || !parts[1] || !parts[2]) {
		throw new TokenVerificationError('Malformed JWT format');
	}
	try {
		const header = JSON.parse(bytesToUtf8(base64UrlToBytes(parts[0]))) as JwtHeader;
		const payload = JSON.parse(bytesToUtf8(base64UrlToBytes(parts[1]))) as Record<string, unknown>;
		const signatureBytes = base64UrlToBytes(parts[2]);
		return {
			header,
			payload,
			signingInput: `${parts[0]}.${parts[1]}`,
			signatureBytes
		};
	} catch {
		throw new TokenVerificationError('Invalid JWT encoding');
	}
}

function audienceMatches(aud: unknown, expected: string): boolean {
	if (typeof aud === 'string') {
		return aud === expected;
	}
	if (Array.isArray(aud)) {
		return aud.includes(expected);
	}
	return false;
}

export interface AuthServer {
	verifyAccessToken(
		token: string,
		options?: VerifyAccessTokenOptions
	): Promise<TenantAccessTokenClaims>;
	verifyIdToken(token: string, options?: VerifyIdTokenOptions): Promise<IdTokenClaims>;
	getJwks(forceRefresh?: boolean): Promise<JwksDocument>;
	listUsers(params?: ListUsersParams): Promise<PaginatedBackendUsers>;
	iterateUsersUpdatedSince(
		updatedSince?: string | Date,
		options?: { pageSize?: number }
	): AsyncGenerator<BackendUserSyncItem, void, unknown>;
	getUserByAppUserId(appUserId: string): Promise<BackendUserSyncItem>;
	getUserBySub(sub: string): Promise<BackendUserSyncItem>;
	updateUserMetadata(
		appUserId: string,
		input: UpdateUserMetadataParams
	): Promise<BackendUserSyncItem>;
	suspendUser(appUserId: string, reason?: string): Promise<BackendUserSyncItem>;
	restoreUser(appUserId: string): Promise<BackendUserSyncItem>;
	getSession(sessionId: string): Promise<BackendSessionStatus>;
	revokeSession(sessionId: string): Promise<BackendSessionStatus>;
	revokeAllUserSessions(appUserId: string): Promise<{ revoked_sessions: number }>;
	listEvents(params?: ListEventsParams): Promise<PaginatedBackendEvents>;
	listOrganizations(params?: ListOrganizationsParams): Promise<PaginatedBackendOrganizations>;
	iterateOrganizationsUpdatedSince(
		updatedSince?: string | Date,
		options?: { pageSize?: number }
	): AsyncGenerator<OrganizationView, void, unknown>;
	getOrganization(orgId: string): Promise<OrganizationView>;
	createOrganization(input: {
		name: string;
		slug: string;
		logoUrl?: string | null;
		metadata?: Record<string, unknown>;
		creatorAppUserId?: string;
	}): Promise<OrganizationView>;
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
	addOrganizationMember(
		orgId: string,
		input: { applicationUserId: string; roleSlug?: string }
	): Promise<OrganizationMemberView>;
	updateOrganizationMemberRole(
		orgId: string,
		memberId: string,
		roleSlug: string
	): Promise<OrganizationMemberView>;
	removeOrganizationMember(orgId: string, memberId: string): Promise<void>;
	listPermissions(): Promise<EnvironmentPermissionView[]>;
	listRoles(): Promise<EnvironmentRoleView[]>;
}

export function createAuthServer(config: AuthServerConfig): AuthServer {
	const issuerUrl = config.issuerUrl.replace(/\/+$/, '');
	const jwksUri = config.jwksUri ?? `${issuerUrl}/.well-known/jwks.json`;
	const jwksTtlMs = config.jwksTtlMs ?? 300_000;
	const minJwksRefreshIntervalMs = config.minJwksRefreshIntervalMs ?? 5_000;
	const clockToleranceSecs = config.clockToleranceSecs ?? 5;
	const timeoutMs = config.timeoutMs ?? 10_000;
	const fetchImpl: FetchLike = config.fetch ?? globalThis.fetch.bind(globalThis);

	let cachedKeys = new Map<string, EcPublicJwk>();
	let cachedJwks: JwksDocument | null = null;
	let cacheExpiresAt = 0;
	let lastFetchAt = 0;
	let cachedEtag: string | null = null;

	async function fetchJwks(forceRefresh = false): Promise<JwksDocument> {
		const now = Date.now();
		if (!forceRefresh && cachedJwks && now < cacheExpiresAt) {
			return cachedJwks;
		}
		if (forceRefresh && cachedJwks && now - lastFetchAt < minJwksRefreshIntervalMs) {
			return cachedJwks;
		}

		const headers: Record<string, string> = {};
		if (cachedEtag && !forceRefresh) {
			headers['if-none-match'] = cachedEtag;
		}

		const res = await fetchWithPolicy(fetchImpl, jwksUri, {
			method: 'GET',
			headers,
			timeoutMs,
			maxRetries: 2,
			idempotent: true
		});

		lastFetchAt = Date.now();
		if (res.status === 304 && cachedJwks) {
			cacheExpiresAt = lastFetchAt + jwksTtlMs;
			return cachedJwks;
		}

		const doc = await parseJsonResponse<JwksDocument>(res);
		const etag = res.headers.get('etag');
		if (etag) {
			cachedEtag = etag;
		}

		const nextMap = new Map<string, EcPublicJwk>();
		for (const key of doc.keys ?? []) {
			if (key.kty === 'EC' && key.crv === 'P-256' && typeof key.kid === 'string') {
				nextMap.set(key.kid, key);
			}
		}

		cachedKeys = nextMap;
		cachedJwks = doc;
		cacheExpiresAt = lastFetchAt + jwksTtlMs;
		return doc;
	}

	async function resolveKeyByKid(kid: string): Promise<EcPublicJwk> {
		await fetchJwks(false);
		const hit = cachedKeys.get(kid);
		if (hit) {
			return hit;
		}
		await fetchJwks(true);
		const refreshedHit = cachedKeys.get(kid);
		if (!refreshedHit) {
			throw new TokenVerificationError(`Unknown or revoked signing key '${kid}'`);
		}
		return refreshedHit;
	}

	async function verifyJwtCore(
		token: string,
		expectedTyp: 'at+jwt' | 'JWT',
		nowSecs?: number
	): Promise<Record<string, unknown>> {
		const { header, payload, signingInput, signatureBytes } = decodeJwtParts(token);

		if (header.alg !== 'ES256') {
			throw new TokenVerificationError(
				`Unsupported JWT signing algorithm '${header.alg ?? 'none'}'`,
				'INVALID_ALG'
			);
		}
		if (header.typ !== expectedTyp) {
			throw new TokenVerificationError(
				`Unexpected JWT type '${header.typ ?? 'missing'}'; expected '${expectedTyp}'`,
				'INVALID_TYP'
			);
		}
		if (!header.kid) {
			throw new TokenVerificationError('Missing kid header', 'MISSING_KID');
		}

		const jwk = await resolveKeyByKid(header.kid);
		const validSig = await verifyEs256SignatureWithJwk(jwk, signingInput, signatureBytes);
		if (!validSig) {
			throw new TokenVerificationError('JWT signature verification failed', 'INVALID_SIGNATURE');
		}

		if (payload['iss'] !== issuerUrl) {
			throw new TokenVerificationError('JWT issuer mismatch', 'INVALID_ISSUER');
		}
		if (!audienceMatches(payload['aud'], config.audience)) {
			throw new TokenVerificationError('JWT audience mismatch', 'INVALID_AUDIENCE');
		}

		const now = nowSecs ?? Math.floor(Date.now() / 1000);
		const exp = payload['exp'];
		if (typeof exp !== 'number' || !Number.isFinite(exp)) {
			throw new TokenVerificationError('Missing exp claim');
		}
		if (now > exp + clockToleranceSecs) {
			throw new TokenExpiredError(exp);
		}

		const nbf = payload['nbf'];
		if (typeof nbf === 'number' && now + clockToleranceSecs < nbf) {
			throw new TokenVerificationError('Token is not yet valid', 'TOKEN_NOT_BEFORE');
		}

		return payload;
	}

	function requireSecretKeyHeaders(): Record<string, string> {
		if (!config.secretKey) {
			throw new AuthSdkError(
				'MISSING_SECRET_KEY',
				'secretKey is required for Backend Sync & Management API calls'
			);
		}
		return {
			authorization: `Bearer ${config.secretKey}`,
			'content-type': 'application/json'
		};
	}

	async function listUsers(params: ListUsersParams = {}): Promise<PaginatedBackendUsers> {
		const qs = new URLSearchParams();
		if (params.updatedSince) {
			const iso =
				params.updatedSince instanceof Date
					? params.updatedSince.toISOString()
					: params.updatedSince;
			qs.set('updated_since', iso);
		}
		if (params.cursor) {
			qs.set('cursor', params.cursor);
		}
		if (typeof params.limit === 'number') {
			qs.set('limit', String(params.limit));
		}
		const suffix = qs.toString() ? `?${qs.toString()}` : '';
		const res = await fetchWithPolicy(fetchImpl, `${issuerUrl}/v1/backend/users${suffix}`, {
			method: 'GET',
			headers: requireSecretKeyHeaders(),
			timeoutMs,
			maxRetries: 2,
			idempotent: true
		});
		return parseJsonResponse<PaginatedBackendUsers>(res);
	}

	async function* iterateUsersUpdatedSince(
		updatedSince?: string | Date,
		options: { pageSize?: number } = {}
	): AsyncGenerator<BackendUserSyncItem, void, unknown> {
		let cursor: string | undefined;
		const limit = options.pageSize ?? 100;
		while (true) {
			const page = await listUsers({ updatedSince, cursor, limit });
			for (const item of page.data) {
				yield item;
			}
			if (!page.has_more || !page.next_cursor) {
				break;
			}
			cursor = page.next_cursor;
		}
	}

	return {
		verifyAccessToken: async (token, options = {}) => {
			const payload = await verifyJwtCore(token, 'at+jwt', options.nowSecs);
			return payload as unknown as TenantAccessTokenClaims;
		},

		verifyIdToken: async (token, options = {}) => {
			const payload = await verifyJwtCore(token, 'JWT', options.nowSecs);
			if (options.expectedNonce && payload['nonce'] !== options.expectedNonce) {
				throw new TokenVerificationError('ID token nonce mismatch', 'INVALID_NONCE');
			}
			return payload as unknown as IdTokenClaims;
		},

		getJwks: fetchJwks,
		listUsers,
		iterateUsersUpdatedSince,

		getUserByAppUserId: async (appUserId) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/users/${encodeURIComponent(appUserId)}`,
				{
					method: 'GET',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					maxRetries: 2,
					idempotent: true
				}
			);
			return parseJsonResponse<BackendUserSyncItem>(res);
		},

		getUserBySub: async (sub) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/users/by-sub/${encodeURIComponent(sub)}`,
				{
					method: 'GET',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					maxRetries: 2,
					idempotent: true
				}
			);
			return parseJsonResponse<BackendUserSyncItem>(res);
		},

		updateUserMetadata: async (appUserId, input) => {
			const payload: Record<string, unknown> = {};
			if (input.externalId !== undefined) payload['external_id'] = input.externalId;
			if (input.publicMetadata !== undefined) payload['public_metadata'] = input.publicMetadata;
			if (input.privateMetadata !== undefined) payload['private_metadata'] = input.privateMetadata;

			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/users/${encodeURIComponent(appUserId)}/metadata`,
				{
					method: 'PATCH',
					headers: requireSecretKeyHeaders(),
					body: JSON.stringify(payload),
					timeoutMs,
					idempotent: false
				}
			);
			return parseJsonResponse<BackendUserSyncItem>(res);
		},

		suspendUser: async (appUserId, reason) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/users/${encodeURIComponent(appUserId)}/suspend`,
				{
					method: 'POST',
					headers: requireSecretKeyHeaders(),
					body: JSON.stringify(reason ? { reason } : {}),
					timeoutMs,
					idempotent: false
				}
			);
			return parseJsonResponse<BackendUserSyncItem>(res);
		},

		restoreUser: async (appUserId) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/users/${encodeURIComponent(appUserId)}/restore`,
				{
					method: 'POST',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					idempotent: false
				}
			);
			return parseJsonResponse<BackendUserSyncItem>(res);
		},

		getSession: async (sessionId) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/sessions/${encodeURIComponent(sessionId)}`,
				{
					method: 'GET',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					maxRetries: 2,
					idempotent: true
				}
			);
			return parseJsonResponse<BackendSessionStatus>(res);
		},

		revokeSession: async (sessionId) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/sessions/${encodeURIComponent(sessionId)}/revoke`,
				{
					method: 'POST',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					idempotent: false
				}
			);
			return parseJsonResponse<BackendSessionStatus>(res);
		},

		revokeAllUserSessions: async (appUserId) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/users/${encodeURIComponent(appUserId)}/sessions/revoke-all`,
				{
					method: 'POST',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					idempotent: false
				}
			);
			return parseJsonResponse<{ revoked_sessions: number }>(res);
		},

		listEvents: async (params = {}) => {
			const qs = new URLSearchParams();
			if (params.afterEventId) qs.set('after_event_id', params.afterEventId);
			if (params.types && params.types.length > 0) qs.set('types', params.types.join(','));
			if (typeof params.limit === 'number') qs.set('limit', String(params.limit));
			const suffix = qs.toString() ? `?${qs.toString()}` : '';
			const res = await fetchWithPolicy(fetchImpl, `${issuerUrl}/v1/backend/events${suffix}`, {
				method: 'GET',
				headers: requireSecretKeyHeaders(),
				timeoutMs,
				maxRetries: 2,
				idempotent: true
			});
			return parseJsonResponse<PaginatedBackendEvents>(res);
		},

		listOrganizations: async (params = {}) => {
			const qs = new URLSearchParams();
			if (params.updatedSince) {
				const iso =
					params.updatedSince instanceof Date
						? params.updatedSince.toISOString()
						: params.updatedSince;
				qs.set('updated_since', iso);
			}
			if (params.cursor) qs.set('cursor', params.cursor);
			if (params.q) qs.set('q', params.q);
			if (typeof params.limit === 'number') qs.set('limit', String(params.limit));
			const suffix = qs.toString() ? `?${qs.toString()}` : '';
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/organizations${suffix}`,
				{
					method: 'GET',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					maxRetries: 2,
					idempotent: true
				}
			);
			return parseJsonResponse<PaginatedBackendOrganizations>(res);
		},

		iterateOrganizationsUpdatedSince: async function* (updatedSince, options = {}) {
			const pageSize = options.pageSize ?? 100;
			let cursor: string | undefined;
			while (true) {
				const qs = new URLSearchParams();
				if (updatedSince) {
					const iso = updatedSince instanceof Date ? updatedSince.toISOString() : updatedSince;
					qs.set('updated_since', iso);
				}
				if (cursor) qs.set('cursor', cursor);
				qs.set('limit', String(pageSize));
				const res = await fetchWithPolicy(
					fetchImpl,
					`${issuerUrl}/v1/backend/organizations?${qs.toString()}`,
					{
						method: 'GET',
						headers: requireSecretKeyHeaders(),
						timeoutMs,
						maxRetries: 2,
						idempotent: true
					}
				);
				const page = await parseJsonResponse<PaginatedBackendOrganizations>(res);
				for (const item of page.data) {
					yield item;
				}
				if (!page.has_more || !page.next_cursor) {
					break;
				}
				cursor = page.next_cursor;
			}
		},

		getOrganization: async (orgId) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/organizations/${encodeURIComponent(orgId)}`,
				{
					method: 'GET',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					maxRetries: 2,
					idempotent: true
				}
			);
			return parseJsonResponse<OrganizationView>(res);
		},

		createOrganization: async (input) => {
			const res = await fetchWithPolicy(fetchImpl, `${issuerUrl}/v1/backend/organizations`, {
				method: 'POST',
				headers: requireSecretKeyHeaders(),
				body: JSON.stringify({
					name: input.name,
					slug: input.slug,
					logo_url: input.logoUrl ?? null,
					metadata: input.metadata,
					creator_app_user_id: input.creatorAppUserId
				}),
				timeoutMs,
				idempotent: false
			});
			return parseJsonResponse<OrganizationView>(res);
		},

		updateOrganization: async (orgId, input) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/organizations/${encodeURIComponent(orgId)}`,
				{
					method: 'PATCH',
					headers: requireSecretKeyHeaders(),
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
		},

		deleteOrganization: async (orgId) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/organizations/${encodeURIComponent(orgId)}`,
				{
					method: 'DELETE',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					idempotent: false
				}
			);
			if (!res.ok) {
				await parseJsonResponse<unknown>(res);
			}
		},

		listOrganizationMembers: async (orgId) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/organizations/${encodeURIComponent(orgId)}/members`,
				{
					method: 'GET',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					maxRetries: 2,
					idempotent: true
				}
			);
			return parseJsonResponse<OrganizationMemberView[]>(res);
		},

		addOrganizationMember: async (orgId, input) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/organizations/${encodeURIComponent(orgId)}/members`,
				{
					method: 'POST',
					headers: requireSecretKeyHeaders(),
					body: JSON.stringify({
						application_user_id: input.applicationUserId,
						role_slug: input.roleSlug
					}),
					timeoutMs,
					idempotent: false
				}
			);
			return parseJsonResponse<OrganizationMemberView>(res);
		},

		updateOrganizationMemberRole: async (orgId, memberId, roleSlug) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/organizations/${encodeURIComponent(orgId)}/members/${encodeURIComponent(memberId)}`,
				{
					method: 'PATCH',
					headers: requireSecretKeyHeaders(),
					body: JSON.stringify({ role_slug: roleSlug }),
					timeoutMs,
					idempotent: false
				}
			);
			return parseJsonResponse<OrganizationMemberView>(res);
		},

		removeOrganizationMember: async (orgId, memberId) => {
			const res = await fetchWithPolicy(
				fetchImpl,
				`${issuerUrl}/v1/backend/organizations/${encodeURIComponent(orgId)}/members/${encodeURIComponent(memberId)}`,
				{
					method: 'DELETE',
					headers: requireSecretKeyHeaders(),
					timeoutMs,
					idempotent: false
				}
			);
			if (!res.ok) {
				await parseJsonResponse<unknown>(res);
			}
		},

		listPermissions: async () => {
			const res = await fetchWithPolicy(fetchImpl, `${issuerUrl}/v1/backend/permissions`, {
				method: 'GET',
				headers: requireSecretKeyHeaders(),
				timeoutMs,
				maxRetries: 2,
				idempotent: true
			});
			return parseJsonResponse<EnvironmentPermissionView[]>(res);
		},

		listRoles: async () => {
			const res = await fetchWithPolicy(fetchImpl, `${issuerUrl}/v1/backend/roles`, {
				method: 'GET',
				headers: requireSecretKeyHeaders(),
				timeoutMs,
				maxRetries: 2,
				idempotent: true
			});
			return parseJsonResponse<EnvironmentRoleView[]>(res);
		}
	};
}
