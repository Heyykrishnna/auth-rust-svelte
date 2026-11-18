import {
	AuthSdkError,
	TokenExpiredError,
	fetchWithPolicy,
	parseJsonResponse,
	type ClientTokenRefreshResponse,
	type FetchLike,
	type OidcTokenResponse,
	type TenantAccessTokenClaims
} from '@secure-auth/core';
import { createAuthServer, type AuthServer } from '@secure-auth/core/server';
import {
	createWebhookRouter,
	type WebhookHandlersMap
} from '@secure-auth/core/webhooks';
import type { PublicAuthState } from './index.js';

export interface CookieSerializeOptions {
	path?: string;
	httpOnly?: boolean;
	secure?: boolean;
	sameSite?: 'lax' | 'strict' | 'none';
	maxAge?: number;
}

export interface SvelteKitCookiesLike {
	get(name: string): string | undefined;
	set(name: string, value: string, opts: CookieSerializeOptions): void;
	delete(name: string, opts: CookieSerializeOptions): void;
}

export interface SvelteKitAuthLocals {
	isAuthenticated: boolean;
	userId: string | null;
	appUserId: string | null;
	sessionId: string | null;
	orgId: string | null;
	roles: string[];
	permissions: string[];
	version: number | null;
	claims: TenantAccessTokenClaims | null;
}

export interface SvelteKitRequestEventLike {
	request: Request;
	url: URL;
	cookies: SvelteKitCookiesLike;
	locals: { auth?: SvelteKitAuthLocals } & Record<string, unknown>;
}

export interface AuthHandleConfig {
	publishableKey: string;
	secretKey?: string;
	issuerUrl: string;
	audience: string;
	accessTokenCookieName?: string;
	refreshTokenCookieName?: string;
	cookieSecure?: boolean;
	refreshOnExpiry?: boolean;
	protectedRoutes?: string[];
	loginPath?: string;
	fetch?: FetchLike;
}

export interface OidcCallbackHandlerConfig {
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

export interface WebhookEndpointConfig {
	secret: string | string[];
	toleranceSecs?: number;
	isEventProcessed?: (eventId: string) => Promise<boolean> | boolean;
	markEventProcessed?: (eventId: string, eventType: string) => Promise<void> | void;
	handlers: WebhookHandlersMap;
}

const DEFAULT_ACCESS_COOKIE = 'sa_access_token';
const DEFAULT_REFRESH_COOKIE = 'sa_refresh_token';
const DEFAULT_PKCE_COOKIE = 'sa_oidc_pkce';

function emptyAuthLocals(): SvelteKitAuthLocals {
	return {
		isAuthenticated: false,
		userId: null,
		appUserId: null,
		sessionId: null,
		orgId: null,
		roles: [],
		permissions: [],
		version: null,
		claims: null
	};
}

function claimsToLocals(claims: TenantAccessTokenClaims): SvelteKitAuthLocals {
	return {
		isAuthenticated: true,
		userId: claims.sub,
		appUserId: claims.app_user_id,
		sessionId: claims.sid,
		orgId: claims.org_id ?? null,
		roles: [...claims.roles],
		permissions: [...claims.permissions],
		version: claims.ver,
		claims
	};
}

function extractBearerToken(req: Request): string | null {
	const header = req.headers.get('authorization');
	if (!header) return null;
	if (!header.startsWith('Bearer ')) return null;
	const token = header.slice('Bearer '.length).trim();
	return token.length > 0 ? token : null;
}

function matchesProtectedRoute(pathname: string, patterns: string[]): boolean {
	for (const pattern of patterns) {
		if (pattern.endsWith('*')) {
			const prefix = pattern.slice(0, -1);
			if (pathname.startsWith(prefix)) return true;
		} else if (pathname === pattern || pathname.startsWith(`${pattern}/`)) {
			return true;
		}
	}
	return false;
}

export function createAuthHandle(config: AuthHandleConfig) {
	const accessCookie = config.accessTokenCookieName ?? DEFAULT_ACCESS_COOKIE;
	const refreshCookie = config.refreshTokenCookieName ?? DEFAULT_REFRESH_COOKIE;
	const issuerUrl = config.issuerUrl.replace(/\/+$/, '');
	const fetchImpl = config.fetch ?? globalThis.fetch.bind(globalThis);
	const cookieSecure = config.cookieSecure ?? issuerUrl.startsWith('https://');
	const protectedRoutes = config.protectedRoutes ?? [];
	const loginPath = config.loginPath ?? '/login';

	const server: AuthServer = createAuthServer({
		issuerUrl,
		audience: config.audience,
		secretKey: config.secretKey,
		publishableKey: config.publishableKey,
		fetch: fetchImpl
	});

	async function tryServerRefresh(
		cookies: SvelteKitCookiesLike
	): Promise<TenantAccessTokenClaims | null> {
		const refreshToken = cookies.get(refreshCookie);
		if (!refreshToken) return null;

		const res = await fetchWithPolicy(fetchImpl, `${issuerUrl}/v1/client/auth/refresh`, {
			method: 'POST',
			headers: {
				'x-publishable-key': config.publishableKey,
				'content-type': 'application/json'
			},
			body: JSON.stringify({ refresh_token: refreshToken }),
			idempotent: false
		});
		if (!res.ok) {
			cookies.delete(accessCookie, { path: '/' });
			cookies.delete(refreshCookie, { path: '/' });
			return null;
		}

		const rotated = await parseJsonResponse<ClientTokenRefreshResponse>(res);
		const verified = await server.verifyAccessToken(rotated.access_token);
		cookies.set(accessCookie, rotated.access_token, {
			path: '/',
			httpOnly: true,
			secure: cookieSecure,
			sameSite: 'lax',
			maxAge: rotated.expires_in
		});
		cookies.set(refreshCookie, rotated.refresh_token, {
			path: '/',
			httpOnly: true,
			secure: cookieSecure,
			sameSite: 'lax',
			maxAge: 60 * 60 * 24 * 7
		});
		return verified;
	}

	return async function handle(input: {
		event: SvelteKitRequestEventLike;
		resolve: (event: SvelteKitRequestEventLike) => Promise<Response> | Response;
	}): Promise<Response> {
		const { event, resolve } = input;
		let authLocals = emptyAuthLocals();

		const token = extractBearerToken(event.request) ?? event.cookies.get(accessCookie) ?? null;
		if (token) {
			try {
				const claims = await server.verifyAccessToken(token);
				authLocals = claimsToLocals(claims);
			} catch (err) {
				if (err instanceof TokenExpiredError && (config.refreshOnExpiry ?? true)) {
					const refreshed = await tryServerRefresh(event.cookies).catch(() => null);
					if (refreshed) {
						authLocals = claimsToLocals(refreshed);
					}
				}
			}
		} else if (config.refreshOnExpiry ?? true) {
			const refreshed = await tryServerRefresh(event.cookies).catch(() => null);
			if (refreshed) {
				authLocals = claimsToLocals(refreshed);
			}
		}

		event.locals.auth = authLocals;

		if (!authLocals.isAuthenticated && matchesProtectedRoute(event.url.pathname, protectedRoutes)) {
			const redirectTarget = `${loginPath}?next=${encodeURIComponent(event.url.pathname + event.url.search)}`;
			return new Response(null, {
				status: 302,
				headers: { location: redirectTarget }
			});
		}

		return resolve(event);
	};
}

export function requireAuth(locals: { auth?: SvelteKitAuthLocals }): SvelteKitAuthLocals {
	if (!locals.auth || !locals.auth.isAuthenticated) {
		throw new AuthSdkError('UNAUTHORIZED', 'Authentication required');
	}
	return locals.auth;
}

export function requirePermission(
	locals: { auth?: SvelteKitAuthLocals },
	permission: string
): SvelteKitAuthLocals {
	const auth = requireAuth(locals);
	if (!auth.permissions.includes(permission)) {
		throw new AuthSdkError('FORBIDDEN', `Missing required permission '${permission}'`);
	}
	return auth;
}

export function serializeClientAuthState(locals: {
	auth?: SvelteKitAuthLocals;
}): PublicAuthState {
	const auth = locals.auth;
	if (!auth || !auth.isAuthenticated || !auth.userId || !auth.appUserId) {
		return {
			isLoaded: true,
			isSignedIn: false,
			user: null,
			sessionId: null,
			roles: [],
			permissions: []
		};
	}

	return {
		isLoaded: true,
		isSignedIn: true,
		user: null,
		sessionId: auth.sessionId,
		roles: [...auth.roles],
		permissions: [...auth.permissions]
	};
}

export function createOidcCallbackHandler(config: OidcCallbackHandlerConfig) {
	const issuerUrl = config.issuerUrl.replace(/\/+$/, '');
	const accessCookie = config.accessTokenCookieName ?? DEFAULT_ACCESS_COOKIE;
	const refreshCookie = config.refreshTokenCookieName ?? DEFAULT_REFRESH_COOKIE;
	const pkceCookie = config.pkceCookieName ?? DEFAULT_PKCE_COOKIE;
	const cookieSecure = config.cookieSecure ?? issuerUrl.startsWith('https://');
	const fetchImpl = config.fetch ?? globalThis.fetch.bind(globalThis);

	return async function GET(event: SvelteKitRequestEventLike): Promise<Response> {
		const error = event.url.searchParams.get('error');
		if (error) {
			return new Response(
				JSON.stringify({
					error,
					error_description: event.url.searchParams.get('error_description')
				}),
				{ status: 400, headers: { 'content-type': 'application/json' } }
			);
		}

		const code = event.url.searchParams.get('code');
		const state = event.url.searchParams.get('state');
		const rawPkce = event.cookies.get(pkceCookie);
		event.cookies.delete(pkceCookie, { path: '/' });

		if (!code || !state || !rawPkce) {
			return new Response(JSON.stringify({ error: 'invalid_callback_state' }), {
				status: 400,
				headers: { 'content-type': 'application/json' }
			});
		}

		let stored: { state?: string; codeVerifier?: string; redirectTo?: string };
		try {
			stored = JSON.parse(rawPkce) as { state?: string; codeVerifier?: string; redirectTo?: string };
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
		event.cookies.set(accessCookie, tokens.access_token, {
			path: '/',
			httpOnly: true,
			secure: cookieSecure,
			sameSite: 'lax',
			maxAge: tokens.expires_in
		});
		if (tokens.refresh_token) {
			event.cookies.set(refreshCookie, tokens.refresh_token, {
				path: '/',
				httpOnly: true,
				secure: cookieSecure,
				sameSite: 'lax',
				maxAge: 60 * 60 * 24 * 7
			});
		}

		const target =
			stored.redirectTo && stored.redirectTo.startsWith('/') && !stored.redirectTo.startsWith('//')
				? stored.redirectTo
				: (config.defaultRedirectTo ?? '/');

		return new Response(null, {
			status: 302,
			headers: { location: target }
		});
	};
}

export function createWebhookEndpointHandler(config: WebhookEndpointConfig) {
	const router = createWebhookRouter({
		secret: config.secret,
		toleranceSecs: config.toleranceSecs,
		isEventProcessed: config.isEventProcessed,
		markEventProcessed: config.markEventProcessed,
		handlers: config.handlers
	});

	return async function POST(event: { request: Request }): Promise<Response> {
		const rawBody = await event.request.text();
		try {
			const result = await router.handle({
				payload: rawBody,
				headers: event.request.headers
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

