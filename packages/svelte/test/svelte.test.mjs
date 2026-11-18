import assert from 'node:assert/strict';
import test from 'node:test';
import { AuthSdkError, bytesToBase64Url, utf8ToBytes } from '@secure-auth/core';
import {
	WEBHOOK_EVENT_ID_HEADER,
	WEBHOOK_SIGNATURE_HEADER,
	WEBHOOK_TIMESTAMP_HEADER,
	signWebhookPayload
} from '@secure-auth/core/webhooks';
import {
	canAccessRoute,
	createSvelteAuthContext,
	useAuth,
	useOrganization,
	useOrganizationList,
	usePermissions,
	useSession,
	useUser
} from '../dist/index.js';
import {
	createAuthHandle,
	createOidcCallbackHandler,
	createWebhookEndpointHandler,
	requireAuth,
	requireOrgPermission,
	requirePermission,
	serializeClientAuthState
} from '../dist/server.js';

async function createSigningKey(kid = 'kid-svelte') {
	const pair = await crypto.subtle.generateKey(
		{ name: 'ECDSA', namedCurve: 'P-256' },
		true,
		['sign', 'verify']
	);
	const jwk = await crypto.subtle.exportKey('jwk', pair.publicKey);
	return {
		kid,
		privateKey: pair.privateKey,
		jwk: { kty: 'EC', crv: 'P-256', use: 'sig', alg: 'ES256', kid, x: jwk.x, y: jwk.y }
	};
}

async function signAccessToken(key, claims) {
	const header = { alg: 'ES256', typ: 'at+jwt', kid: key.kid };
	const encHeader = bytesToBase64Url(utf8ToBytes(JSON.stringify(header)));
	const encPayload = bytesToBase64Url(utf8ToBytes(JSON.stringify(claims)));
	const signingInput = `${encHeader}.${encPayload}`;
	const sig = await crypto.subtle.sign(
		{ name: 'ECDSA', hash: 'SHA-256' },
		key.privateKey,
		utf8ToBytes(signingInput)
	);
	return `${signingInput}.${bytesToBase64Url(new Uint8Array(sig))}`;
}

function createMemoryCookies(initial = {}) {
	const map = new Map(Object.entries(initial));
	const setCalls = [];
	return {
		map,
		setCalls,
		get: (name) => map.get(name),
		set: (name, value, opts) => {
			map.set(name, value);
			setCalls.push({ name, value, opts });
		},
		delete: (name) => {
			map.delete(name);
		}
	};
}

test('createAuthHandle verifies SSR JWT, transparently refreshes expired tokens, protects routes, and leaks zero secrets', async () => {
	const key = await createSigningKey();
	const now = Math.floor(Date.now() / 1000);
	const baseClaims = {
		iss: 'https://auth.example.test',
		sub: 'usr_svelte_1',
		aud: 'ferrum',
		exp: now + 300,
		iat: now - 5,
		jti: 'jti_sv',
		sid: 'sid_sv_1',
		app_id: 'app_ferrum',
		app_user_id: 'au_sv_1',
		roles: ['editor'],
		permissions: ['articles:write'],
		ver: 2
	};

	const freshToken = await signAccessToken(key, baseClaims);
	const expiredToken = await signAccessToken(key, { ...baseClaims, exp: now - 120 });
	const secretKey = 'backend-secret-never-serialized-12345';
	const refreshSecret = 'refresh-token-never-serialized-67890';

	const handle = createAuthHandle({
		issuerUrl: 'https://auth.example.test',
		audience: 'ferrum',
		publishableKey: 'pub-ferrum',
		secretKey,
		protectedRoutes: ['/dashboard'],
		loginPath: '/login',
		fetch: async (input) => {
			const url = String(input);
			if (url.endsWith('/.well-known/jwks.json')) {
				return new Response(JSON.stringify({ keys: [key.jwk] }), {
					status: 200,
					headers: { 'content-type': 'application/json' }
				});
			}
			if (url.endsWith('/v1/client/auth/refresh')) {
				return new Response(
					JSON.stringify({
						access_token: freshToken,
						refresh_token: 'rotated-refresh-token-value',
						token_type: 'Bearer',
						expires_in: 900
					}),
					{ status: 200, headers: { 'content-type': 'application/json' } }
				);
			}
			throw new Error(`Unexpected URL: ${url}`);
		}
	});

	const unauthCookies = createMemoryCookies();
	const unauthEvent = {
		request: new Request('https://ferrum.example.test/dashboard?tab=billing'),
		url: new URL('https://ferrum.example.test/dashboard?tab=billing'),
		cookies: unauthCookies,
		locals: {}
	};
	const redirectRes = await handle({
		event: unauthEvent,
		resolve: () => new Response('should not render')
	});
	assert.equal(redirectRes.status, 302);
	assert.equal(
		redirectRes.headers.get('location'),
		'/login?next=%2Fdashboard%3Ftab%3Dbilling'
	);

	const expiredCookies = createMemoryCookies({
		sa_access_token: expiredToken,
		sa_refresh_token: refreshSecret
	});
	const refreshEvent = {
		request: new Request('https://ferrum.example.test/dashboard'),
		url: new URL('https://ferrum.example.test/dashboard'),
		cookies: expiredCookies,
		locals: {}
	};
	const okRes = await handle({
		event: refreshEvent,
		resolve: (ev) => {
			const verified = requirePermission(ev.locals, 'articles:write');
			assert.equal(verified.userId, 'usr_svelte_1');
			const serialized = serializeClientAuthState(ev.locals);
			const html = `<script>window.__AUTH__=${JSON.stringify(serialized)}</script>`;
			return new Response(html, { status: 200, headers: { 'content-type': 'text/html' } });
		}
	});
	assert.equal(okRes.status, 200);
	const body = await okRes.text();
	assert.equal(body.includes(secretKey), false);
	assert.equal(body.includes(refreshSecret), false);
	assert.equal(body.includes('rotated-refresh-token-value'), false);
	assert.equal(body.includes(freshToken), false);
	assert.ok(expiredCookies.setCalls.every((c) => c.opts.httpOnly === true));

	assert.throws(() => requireAuth({}), AuthSdkError);
	assert.throws(
		() => requirePermission(refreshEvent.locals, 'billing:admin'),
		(err) => err instanceof AuthSdkError && err.code === 'FORBIDDEN'
	);
});

test('createOidcCallbackHandler prevents open redirects and createWebhookEndpointHandler verifies signatures', async () => {
	const cookies = createMemoryCookies({
		sa_oidc_pkce: JSON.stringify({
			state: 'st_ok',
			codeVerifier: 'verifier_value_12345',
			redirectTo: '//evil.example/phish'
		})
	});

	const callbackGet = createOidcCallbackHandler({
		issuerUrl: 'https://auth.example.test',
		clientId: 'client_ferrum',
		redirectUri: 'https://ferrum.example.test/auth/callback',
		defaultRedirectTo: '/dashboard',
		fetch: async () =>
			new Response(
				JSON.stringify({
					access_token: 'cb-access-token',
					refresh_token: 'cb-refresh-token',
					token_type: 'Bearer',
					expires_in: 600
				}),
				{ status: 200, headers: { 'content-type': 'application/json' } }
			)
	});

	const cbRes = await callbackGet({
		request: new Request('https://ferrum.example.test/auth/callback?code=c1&state=st_ok'),
		url: new URL('https://ferrum.example.test/auth/callback?code=c1&state=st_ok'),
		cookies,
		locals: {}
	});
	assert.equal(cbRes.status, 302);
	assert.equal(cbRes.headers.get('location'), '/dashboard');
	assert.equal(cookies.get('sa_access_token'), 'cb-access-token');

	const secret = 'svelte-webhook-secret-test';
	const processed = new Set();
	let count = 0;
	const webhookPost = createWebhookEndpointHandler({
		secret,
		isEventProcessed: (id) => processed.has(id),
		markEventProcessed: (id) => {
			processed.add(id);
		},
		handlers: {
			'user.created': () => {
				count += 1;
			}
		}
	});

	const ts = Math.floor(Date.now() / 1000);
	const raw = JSON.stringify({
		spec_version: '1.0',
		event_id: 'evt_sv_1',
		event_type: 'user.created',
		occurred_at: '2026-01-01T00:00:00Z',
		aggregate_type: 'user',
		aggregate_id: 'u_1',
		aggregate_version: 1,
		data: { sub: 'u_1', app_user_id: 'au_1' }
	});
	const sig = await signWebhookPayload(secret, ts, raw);

	const goodRes = await webhookPost({
		request: new Request('https://ferrum.example.test/api/webhooks/auth', {
			method: 'POST',
			headers: {
				'content-type': 'application/json',
				[WEBHOOK_EVENT_ID_HEADER]: 'evt_sv_1',
				[WEBHOOK_TIMESTAMP_HEADER]: String(ts),
				[WEBHOOK_SIGNATURE_HEADER]: sig
			},
			body: raw
		})
	});
	assert.equal(goodRes.status, 200);
	assert.equal(count, 1);

	const badRes = await webhookPost({
		request: new Request('https://ferrum.example.test/api/webhooks/auth', {
			method: 'POST',
			headers: {
				'content-type': 'application/json',
				[WEBHOOK_EVENT_ID_HEADER]: 'evt_sv_1',
				[WEBHOOK_TIMESTAMP_HEADER]: String(ts),
				[WEBHOOK_SIGNATURE_HEADER]: 'v1=deadbeef'
			},
			body: raw
		})
	});
	assert.equal(badRes.status, 401);
});

test('Svelte client context hooks and canAccessRoute enforce permissions reactively', () => {
	const ctx = createSvelteAuthContext(
		{
			issuerUrl: 'https://auth.example.test',
			publishableKey: 'pub-ferrum'
		},
		{
			isLoaded: true,
			isSignedIn: true,
			sessionId: 'sid_1',
			roles: ['member'],
			permissions: ['posts:read']
		}
	);
	assert.equal(useAuth(ctx), ctx);
	assert.equal(useUser(ctx), null);
	assert.deepEqual(useSession(ctx), { isSignedIn: true, sessionId: 'sid_1' });
	const perms = usePermissions(ctx);
	assert.equal(perms.hasPermission('posts:read'), true);
	assert.equal(perms.hasRole('admin'), false);
	assert.equal(canAccessRoute(ctx.getState(), { permission: 'posts:read' }), true);
	assert.equal(canAccessRoute(ctx.getState(), { permission: 'billing:write' }), false);
});

test('Phase 08: Svelte organization hooks and requireOrgPermission enforce org context boundaries', async () => {
	const ctx = createSvelteAuthContext(
		{
			issuerUrl: 'https://auth.example.test',
			publishableKey: 'pub-ferrum',
			fetch: async (input) => {
				const url = String(input);
				if (url.endsWith('/v1/client/organizations')) {
					return new Response(
						JSON.stringify([
							{
								organization: {
									id: 'org_acme',
									environment_id: 'env_1',
									slug: 'acme',
									name: 'Acme Inc',
									logo_url: null,
									status: 'active',
									metadata: {},
									version: 1,
									member_count: 2,
									created_at: '2026-01-01T00:00:00Z',
									updated_at: '2026-01-01T00:00:00Z'
								},
								member_id: 'mem_1',
								roles: ['org:owner'],
								status: 'active',
								joined_at: '2026-01-01T00:00:00Z'
							}
						]),
						{ status: 200, headers: { 'content-type': 'application/json' } }
					);
				}
				return new Response(
					JSON.stringify({
						access_token: 'org-jwt',
						token_type: 'Bearer',
						expires_in: 900,
						active_organization_id: 'org_acme',
						org_slug: 'acme',
						org_role: 'org:owner',
						org_permissions: ['org:billing:write']
					}),
					{ status: 200, headers: { 'content-type': 'application/json' } }
				);
			}
		},
		{
			isLoaded: true,
			isSignedIn: true,
			sessionId: 'sid_1',
			roles: ['user'],
			permissions: ['profile.read']
		}
	);
	ctx.client.setSession({ accessToken: 'tok_1' });
	await ctx.refreshOrganizations();
	await ctx.setActiveOrganization('org_acme');

	assert.equal(useOrganizationList(ctx).organizations.length, 1);
	const org = useOrganization(ctx);
	assert.equal(org.orgId, 'org_acme');
	assert.equal(org.orgSlug, 'acme');
	assert.equal(org.orgRole, 'org:owner');
	assert.equal(ctx.has({ permission: 'org:billing:write', orgId: 'org_acme' }), true);
	assert.equal(ctx.has({ permission: 'org:billing:write', orgId: 'org_other' }), false);

	const mockLocals = {
		auth: {
			isAuthenticated: true,
			userId: 'u_1',
			appUserId: 'au_1',
			sessionId: 'sid_1',
			orgId: 'org_acme',
			orgSlug: 'acme',
			orgRole: 'org:owner',
			orgPermissions: ['org:billing:write'],
			roles: ['user'],
			permissions: ['profile.read'],
			version: 1,
			claims: null,
			has: (c) => ctx.has(c)
		}
	};
	assert.equal(
		requireOrgPermission(mockLocals, { permission: 'org:billing:write', orgId: 'org_acme' }).orgId,
		'org_acme'
	);
	assert.throws(
		() => requireOrgPermission(mockLocals, { permission: 'org:billing:write', orgId: 'org_other' }),
		(err) => err instanceof AuthSdkError && err.code === 'FORBIDDEN'
	);
});
