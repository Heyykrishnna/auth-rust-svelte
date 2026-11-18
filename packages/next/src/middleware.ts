import {
	createAuthServer,
	type AuthServer,
	type FetchLike,
	type TenantAccessTokenClaims
} from '@secure-auth/core/server';

export const DEFAULT_ACCESS_TOKEN_COOKIE = 'sa_access_token';
export const DEFAULT_REFRESH_TOKEN_COOKIE = 'sa_refresh_token';
export const DEFAULT_PKCE_COOKIE = 'sa_oidc_pkce';

export const INTERNAL_AUTH_HEADERS = {
	userId: 'x-sa-user-id',
	appUserId: 'x-sa-app-user-id',
	sessionId: 'x-sa-session-id',
	orgId: 'x-sa-org-id',
	roles: 'x-sa-roles',
	permissions: 'x-sa-permissions',
	version: 'x-sa-ver'
} as const;

export interface NextRequestLike {
	url: string;
	method: string;
	headers: Headers;
	nextUrl?: URL;
	cookies?: {
		get(name: string): { value: string } | undefined;
	};
}

export interface MiddlewareAuthContext {
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

export interface AuthMiddlewareConfig {
	issuerUrl: string;
	audience: string;
	publishableKey?: string;
	accessTokenCookieName?: string;
	protectedRoutes?: string[];
	publicRoutes?: string[];
	loginPath?: string;
	jwksTtlMs?: number;
	fetch?: FetchLike;
	afterAuth?: (
		auth: MiddlewareAuthContext,
		request: NextRequestLike
	) => Promise<Response | void> | Response | void;
}

function parseCookieHeader(cookieHeader: string | null): Map<string, string> {
	const map = new Map<string, string>();
	if (!cookieHeader) return map;
	for (const segment of cookieHeader.split(';')) {
		const idx = segment.indexOf('=');
		if (idx <= 0) continue;
		const key = segment.slice(0, idx).trim();
		const rawVal = segment.slice(idx + 1).trim();
		if (key.length === 0) continue;
		try {
			map.set(key, decodeURIComponent(rawVal));
		} catch {
			map.set(key, rawVal);
		}
	}
	return map;
}

export function extractTokenFromRequest(
	request: NextRequestLike,
	cookieName: string = DEFAULT_ACCESS_TOKEN_COOKIE
): string | null {
	const authHeader = request.headers.get('authorization');
	if (authHeader && authHeader.startsWith('Bearer ')) {
		const bearer = authHeader.slice('Bearer '.length).trim();
		if (bearer.length > 0) return bearer;
	}

	const fromNextCookies = request.cookies?.get(cookieName)?.value;
	if (fromNextCookies && fromNextCookies.length > 0) {
		return fromNextCookies;
	}

	const parsed = parseCookieHeader(request.headers.get('cookie'));
	const fromHeader = parsed.get(cookieName);
	return fromHeader && fromHeader.length > 0 ? fromHeader : null;
}

function matchesRoutePattern(pathname: string, patterns: string[]): boolean {
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

function isProtectedPath(
	pathname: string,
	protectedRoutes: string[] | undefined,
	publicRoutes: string[] | undefined
): boolean {
	if (publicRoutes && matchesRoutePattern(pathname, publicRoutes)) {
		return false;
	}
	if (protectedRoutes && protectedRoutes.length > 0) {
		return matchesRoutePattern(pathname, protectedRoutes);
	}
	return false;
}

function emptyContext(): MiddlewareAuthContext {
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

function buildNextResponseHeaders(
	request: NextRequestLike,
	auth: MiddlewareAuthContext
): Headers {
	const forwarded = new Headers(request.headers);
	for (const headerName of Object.values(INTERNAL_AUTH_HEADERS)) {
		forwarded.delete(headerName);
	}
	if (auth.isAuthenticated && auth.userId && auth.appUserId && auth.sessionId) {
		forwarded.set(INTERNAL_AUTH_HEADERS.userId, auth.userId);
		forwarded.set(INTERNAL_AUTH_HEADERS.appUserId, auth.appUserId);
		forwarded.set(INTERNAL_AUTH_HEADERS.sessionId, auth.sessionId);
		if (auth.orgId) {
			forwarded.set(INTERNAL_AUTH_HEADERS.orgId, auth.orgId);
		}
		forwarded.set(INTERNAL_AUTH_HEADERS.roles, auth.roles.join(','));
		forwarded.set(INTERNAL_AUTH_HEADERS.permissions, auth.permissions.join(','));
		if (auth.version !== null) {
			forwarded.set(INTERNAL_AUTH_HEADERS.version, String(auth.version));
		}
	}

	const responseHeaders = new Headers();
	responseHeaders.set('x-middleware-next', '1');
	const overrideKeys: string[] = [];
	forwarded.forEach((value, key) => {
		overrideKeys.push(key);
		responseHeaders.set(`x-middleware-request-${key}`, value);
	});
	responseHeaders.set('x-middleware-override-headers', overrideKeys.join(','));
	return responseHeaders;
}

export function authMiddleware(config: AuthMiddlewareConfig) {
	const cookieName = config.accessTokenCookieName ?? DEFAULT_ACCESS_TOKEN_COOKIE;
	const loginPath = config.loginPath ?? '/login';

	const server: AuthServer = createAuthServer({
		issuerUrl: config.issuerUrl,
		audience: config.audience,
		publishableKey: config.publishableKey,
		jwksTtlMs: config.jwksTtlMs,
		fetch: config.fetch
	});

	return async function middleware(request: NextRequestLike): Promise<Response> {
		const url = request.nextUrl ?? new URL(request.url);
		let auth = emptyContext();

		const token = extractTokenFromRequest(request, cookieName);
		if (token) {
			try {
				const claims = await server.verifyAccessToken(token);
				auth = {
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
			} catch {
				auth = emptyContext();
			}
		}

		if (config.afterAuth) {
			const custom = await config.afterAuth(auth, request);
			if (custom) {
				return custom;
			}
		}

		if (
			!auth.isAuthenticated &&
			isProtectedPath(url.pathname, config.protectedRoutes, config.publicRoutes)
		) {
			const redirectUrl = new URL(loginPath, url.origin);
			redirectUrl.searchParams.set('next', `${url.pathname}${url.search}`);
			return new Response(null, {
				status: 302,
				headers: { location: redirectUrl.toString() }
			});
		}

		const responseHeaders = buildNextResponseHeaders(request, auth);
		return new Response(null, {
			status: 200,
			headers: responseHeaders
		});
	};
}
