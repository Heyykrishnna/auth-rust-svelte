import {
	AuthSdkError,
	fetchWithPolicy,
	parseJsonResponse,
	type BackendUserSyncItem,
	type FetchLike,
	type OidcTokenResponse,
	type TenantAccessTokenClaims
} from '@secure-auth/core';
import { createAuthServer, type AuthServer } from '@secure-auth/core/server';
import {
	createWebhookRouter,
	type WebhookHandlersMap
} from '@secure-auth/core/webhooks';
import {
	DEFAULT_ACCESS_TOKEN_COOKIE,
	DEFAULT_PKCE_COOKIE,
	DEFAULT_REFRESH_TOKEN_COOKIE,
	extractTokenFromRequest,
	type NextRequestLike
} from './middleware.js';

export interface NextCookiesReadLike {
	get(name: string): { value: string } | undefined;
}

export type AuthSource =
	| NextRequestLike
	| NextCookiesReadLike
	| (() => Promise<NextCookiesReadLike> | NextCookiesReadLike);

export interface NextAuthServerConfig {
	issuerUrl: string;
	audience: string;
	secretKey?: string;
	publishableKey?: string;
	accessTokenCookieName?: string;
	fetch?: FetchLike;
}

export interface NextAuthResult {
	isAuthenticated: boolean;
	userId: string | null;
	appUserId: string | null;
	sessionId: string | null;
	orgId: string | null;
	roles: string[];
	permissions: string[];
	version: number | null;
	claims: TenantAccessTokenClaims | null;
	hasPermission(permission: string): boolean;
	hasRole(role: string): boolean;
}

export interface NextCallbackRouteConfig {
	issuerUrl: string;
	clientId: string;
	redirectUri: string;
	accessTokenCookieName?: string;
	refreshTokenCookieName?: string;
	pkceCookieName?: string;
	cookieSecure?: boolean;
	defaultRedirectTo?: string;
	fetch?: FetchLike;
}

export interface NextWebhookRouteConfig {
	secret: string | string[];
	toleranceSecs?: number;
	isEventProcessed?: (eventId: string) => Promise<boolean> | boolean;
	markEventProcessed?: (eventId: string, eventType: string) => Promise<void> | void;
	handlers: WebhookHandlersMap;
}

function emptyNextAuthResult(): NextAuthResult {
	return {
		isAuthenticated: false,
		userId: null,
		appUserId: null,
		sessionId: null,
		orgId: null,
		roles: [],
		permissions: [],
		version: null,
		claims: null,
		hasPermission: () => false,
		hasRole: () => false
	};
}

function claimsToNextAuthResult(claims: TenantAccessTokenClaims): NextAuthResult {
	const roles = [...claims.roles];
	const permissions = [...claims.permissions];
	return {
		isAuthenticated: true,
		userId: claims.sub,
		appUserId: claims.app_user_id,
		sessionId: claims.sid,
		orgId: claims.org_id ?? null,
		roles,
		permissions,
		version: claims.ver,
		claims,
		hasPermission: (permission: string) => permissions.includes(permission),
		hasRole: (role: string) => roles.includes(role)
	};
}

async function resolveAccessToken(source: AuthSource, cookieName: string): Promise<string | null> {
	if (typeof source === 'function') {
		const cookies = await source();
		return cookies.get(cookieName)?.value ?? null;
	}
	if ('headers' in source && source.headers instanceof Headers) {
		return extractTokenFromRequest(source, cookieName);
	}
	if ('get' in source && typeof source.get === 'function') {
		return source.get(cookieName)?.value ?? null;
	}
	return null;
}

function formatSetCookie(
	name: string,
	value: string,
	options: {
		httpOnly?: boolean;
		secure?: boolean;
		sameSite?: 'Lax' | 'Strict' | 'None';
		path?: string;
		maxAge?: number;
	}
): string {
	const parts = [`${name}=${encodeURIComponent(value)}`];
	parts.push(`Path=${options.path ?? '/'}`);
	if (options.httpOnly ?? true) parts.push('HttpOnly');
	if (options.secure) parts.push('Secure');
	parts.push(`SameSite=${options.sameSite ?? 'Lax'}`);
	if (typeof options.maxAge === 'number') {
		parts.push(`Max-Age=${Math.max(0, Math.floor(options.maxAge))}`);
	}
	return parts.join('; ');
}

function parseCookiesFromHeader(header: string | null): Map<string, string> {
	const map = new Map<string, string>();
	if (!header) return map;
	for (const part of header.split(';')) {
		const idx = part.indexOf('=');
		if (idx <= 0) continue;
		const key = part.slice(0, idx).trim();
		const val = part.slice(idx + 1).trim();
		if (key.length === 0) continue;
		try {
			map.set(key, decodeURIComponent(val));
		} catch {
			map.set(key, val);
		}
	}
	return map;
}

export function protect(
	authState: NextAuthResult,
	requirements: { permission?: string; role?: string } = {}
): NextAuthResult {
	if (!authState.isAuthenticated) {
		throw new AuthSdkError('UNAUTHORIZED', 'Authentication required');
	}
	if (requirements.permission && !authState.hasPermission(requirements.permission)) {
		throw new AuthSdkError(
			'FORBIDDEN',
			`Missing required permission '${requirements.permission}'`
		);
	}
	if (requirements.role && !authState.hasRole(requirements.role)) {
		throw new AuthSdkError('FORBIDDEN', `Missing required role '${requirements.role}'`);
	}
	return authState;
}

export function createCallbackRouteHandler(config: NextCallbackRouteConfig) {
	const issuerUrl = config.issuerUrl.replace(/\/+$/, '');
	const accessCookie = config.accessTokenCookieName ?? DEFAULT_ACCESS_TOKEN_COOKIE;
	const refreshCookie = config.refreshTokenCookieName ?? DEFAULT_REFRESH_TOKEN_COOKIE;
	const pkceCookie = config.pkceCookieName ?? DEFAULT_PKCE_COOKIE;
	const cookieSecure = config.cookieSecure ?? issuerUrl.startsWith('https://');
	const fetchImpl = config.fetch ?? globalThis.fetch.bind(globalThis);

	return async function GET(request: Request): Promise<Response> {
		const url = new URL(request.url);
		const oauthError = url.searchParams.get('error');
		if (oauthError) {
			return new Response(
				JSON.stringify({
					error: oauthError,
					error_description: url.searchParams.get('error_description')
				}),
				{ status: 400, headers: { 'content-type': 'application/json' } }
			);
		}

		const code = url.searchParams.get('code');
		const state = url.searchParams.get('state');
		const cookies = parseCookiesFromHeader(request.headers.get('cookie'));
		const rawPkce = cookies.get(pkceCookie);

		if (!code || !state || !rawPkce) {
			return new Response(JSON.stringify({ error: 'invalid_callback_state' }), {
				status: 400,
				headers: { 'content-type': 'application/json' }
			});
		}

		let stored: { state?: string; codeVerifier?: string; redirectTo?: string };
		try {
			stored = JSON.parse(rawPkce) as {
				state?: string;
				codeVerifier?: string;
				redirectTo?: string;
			};
		} catch {
			return new Response(JSON.stringify({ error: 'corrupted_pkce_cookie' }), {
				status: 400,
				headers: { 'content-type': 'application/json' }
			});
		}

		if (!stored.codeVerifier || stored.state !== state) {
			return new Response(JSON.stringify({ error: 'state_mismatch' }), {
				status: 400,
				headers: { 'content-type': 'application/json' }
			});
		}

		const form = new URLSearchParams({
			grant_type: 'authorization_code',
			client_id: config.clientId,
			code,
			redirect_uri: config.redirectUri,
			code_verifier: stored.codeVerifier
		});

		const tokenRes = await fetchWithPolicy(fetchImpl, `${issuerUrl}/oauth2/token`, {
			method: 'POST',
			headers: { 'content-type': 'application/x-www-form-urlencoded' },
			body: form.toString(),
			idempotent: false
		});
		if (!tokenRes.ok) {
			return new Response(JSON.stringify({ error: 'token_exchange_failed' }), {
				status: 401,
				headers: { 'content-type': 'application/json' }
			});
		}

		const tokens = await parseJsonResponse<OidcTokenResponse>(tokenRes);
		const target =
			stored.redirectTo &&
			stored.redirectTo.startsWith('/') &&
			!stored.redirectTo.startsWith('//')
				? stored.redirectTo
				: (config.defaultRedirectTo ?? '/');

		const headers = new Headers({ location: target });
		headers.append(
			'set-cookie',
			formatSetCookie(pkceCookie, '', { httpOnly: true, secure: cookieSecure, maxAge: 0 })
		);
		headers.append(
			'set-cookie',
			formatSetCookie(accessCookie, tokens.access_token, {
				httpOnly: true,
				secure: cookieSecure,
				sameSite: 'Lax',
				maxAge: tokens.expires_in
			})
		);
		if (tokens.refresh_token) {
			headers.append(
				'set-cookie',
				formatSetCookie(refreshCookie, tokens.refresh_token, {
					httpOnly: true,
					secure: cookieSecure,
					sameSite: 'Lax',
					maxAge: 60 * 60 * 24 * 7
				})
			);
		}

		return new Response(null, { status: 302, headers });
	};
}

export function createWebhookRouteHandler(config: NextWebhookRouteConfig) {
	const router = createWebhookRouter({
		secret: config.secret,
		toleranceSecs: config.toleranceSecs,
		isEventProcessed: config.isEventProcessed,
		markEventProcessed: config.markEventProcessed,
		handlers: config.handlers
	});

	return async function POST(request: Request): Promise<Response> {
		const rawBody = await request.text();
		try {
			const result = await router.handle({
				payload: rawBody,
				headers: request.headers
			});
			return new Response(JSON.stringify(result), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			});
		} catch (err) {
			const message = err instanceof Error ? err.message : 'Webhook verification failed';
			return new Response(JSON.stringify({ error: message }), {
				status: 401,
				headers: { 'content-type': 'application/json' }
			});
		}
	};
}

export function createNextAuthServer(config: NextAuthServerConfig) {
	const cookieName = config.accessTokenCookieName ?? DEFAULT_ACCESS_TOKEN_COOKIE;
	const server: AuthServer = createAuthServer({
		issuerUrl: config.issuerUrl,
		audience: config.audience,
		secretKey: config.secretKey,
		publishableKey: config.publishableKey,
		fetch: config.fetch
	});

	async function auth(source: AuthSource): Promise<NextAuthResult> {
		const token = await resolveAccessToken(source, cookieName);
		if (!token) {
			return emptyNextAuthResult();
		}
		try {
			const claims = await server.verifyAccessToken(token);
			return claimsToNextAuthResult(claims);
		} catch {
			return emptyNextAuthResult();
		}
	}

	async function currentUser(source: AuthSource): Promise<BackendUserSyncItem | null> {
		const state = await auth(source);
		if (!state.isAuthenticated || !state.userId) {
			return null;
		}
		if (state.appUserId) {
			return server.getUserByAppUserId(state.appUserId);
		}
		return server.getUserBySub(state.userId);
	}

	return {
		server,
		auth,
		currentUser,
		protect,
		createCallbackRouteHandler,
		createWebhookRouteHandler
	};
}

export async function auth(
	config: NextAuthServerConfig,
	source: AuthSource
): Promise<NextAuthResult> {
	return createNextAuthServer(config).auth(source);
}

export async function currentUser(
	config: NextAuthServerConfig,
	source: AuthSource
): Promise<BackendUserSyncItem | null> {
	return createNextAuthServer(config).currentUser(source);
}
