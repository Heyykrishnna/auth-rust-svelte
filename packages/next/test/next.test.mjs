import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { AuthSdkError, bytesToBase64Url, utf8ToBytes } from '@secure-auth/core';
import {
	WEBHOOK_EVENT_ID_HEADER,
	WEBHOOK_SIGNATURE_HEADER,
	WEBHOOK_TIMESTAMP_HEADER,
	signWebhookPayload
} from '@secure-auth/core/webhooks';
import { INTERNAL_AUTH_HEADERS, authMiddleware } from '../dist/middleware.js';
import {
	createCallbackRouteHandler,
	createNextAuthServer,
	createWebhookRouteHandler,
	protect
} from '../dist/server.js';

async function createSigningKey(kid = 'kid-next') {
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

test('Edge authMiddleware verifies ES256 JWTs, strips spoofed headers, and redirects protected routes', async () => {
	const key = await createSigningKey();
	const now = Math.floor(Date.now() / 1000);
	const validToken = await signAccessToken(key, {
		iss: 'https://auth.example.test',
		sub: 'usr_next_1',
		aud: 'elysiar',
		exp: now + 300,
		iat: now - 5,
		jti: 'jti_nx_1',
		sid: 'sid_nx_1',
		app_id: 'app_elysiar',
		app_user_id: 'au_nx_1',
		roles: ['owner'],
		permissions: ['billing:write'],
		ver: 5
	});

	const middleware = authMiddleware({
		issuerUrl: 'https://auth.example.test',
		audience: 'elysiar',
		protectedRoutes: ['/dashboard', '/billing'],
		publicRoutes: ['/login', '/api/webhooks/auth'],
		loginPath: '/login',
		fetch: async () =>
			new Response(JSON.stringify({ keys: [key.jwk] }), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			})
	});

	const unauthProtected = await middleware(
		new Request('https://elysiar.example.test/dashboard?view=overview', {
			headers: {
				[INTERNAL_AUTH_HEADERS.userId]: 'spoofed_user'
			}
		})
	);
	assert.equal(unauthProtected.status, 302);
	assert.equal(
		unauthProtected.headers.get('location'),
		'https://elysiar.example.test/login?next=%2Fdashboard%3Fview%3Doverview'
	);

	const publicWithSpoof = await middleware(
		new Request('https://elysiar.example.test/login', {
			headers: {
				[INTERNAL_AUTH_HEADERS.userId]: 'spoofed_user'
			}
		})
	);
	assert.equal(publicWithSpoof.status, 200);
	assert.equal(
		publicWithSpoof.headers.get(`x-middleware-request-${INTERNAL_AUTH_HEADERS.userId}`),
		null
	);

	const authedRes = await middleware(
		new Request('https://elysiar.example.test/dashboard', {
			headers: {
				cookie: `sa_access_token=${encodeURIComponent(validToken)}`,
				[INTERNAL_AUTH_HEADERS.userId]: 'spoofed_user'
			}
		})
	);
	assert.equal(authedRes.status, 200);
	assert.equal(authedRes.headers.get('x-middleware-next'), '1');
	assert.equal(
		authedRes.headers.get(`x-middleware-request-${INTERNAL_AUTH_HEADERS.userId}`),
		'usr_next_1'
	);
	assert.equal(
		authedRes.headers.get(`x-middleware-request-${INTERNAL_AUTH_HEADERS.permissions}`),
		'billing:write'
	);
});

test('Next.js App Router server helpers support auth(), currentUser(), protect(), OIDC callback, and webhooks', async () => {
	const key = await createSigningKey();
	const now = Math.floor(Date.now() / 1000);
	const token = await signAccessToken(key, {
		iss: 'https://auth.example.test',
		sub: 'usr_next_2',
		aud: 'elysiar',
		exp: now + 300,
		iat: now,
		jti: 'jti_nx_2',
		sid: 'sid_nx_2',
		app_id: 'app_elysiar',
		app_user_id: 'au_nx_2',
		roles: ['member'],
		permissions: ['projects:read'],
		ver: 1
	});

	const nextServer = createNextAuthServer({
		issuerUrl: 'https://auth.example.test',
		audience: 'elysiar',
		secretKey: 'next-backend-secret-value',
		fetch: async (input) => {
			const url = String(input);
			if (url.endsWith('/.well-known/jwks.json')) {
				return new Response(JSON.stringify({ keys: [key.jwk] }), {
					status: 200,
					headers: { 'content-type': 'application/json' }
				});
			}
			if (url.endsWith('/v1/backend/users/au_nx_2')) {
				return new Response(
					JSON.stringify({
						app_user_id: 'au_nx_2',
						user_id: 'usr_next_2',
						email: 'next@example.test',
						email_verified: true,
						display_name: 'Next User',
						status: 'active',
						roles: ['member'],
						permissions: ['projects:read'],
						external_id: null,
						public_metadata: {},
						private_metadata: {},
						version: 1,
						created_at: '2026-01-01T00:00:00Z',
						updated_at: '2026-01-01T00:00:00Z'
					}),
					{ status: 200, headers: { 'content-type': 'application/json' } }
				);
			}
			throw new Error(`Unexpected URL: ${url}`);
		}
	});

	const asyncCookies = async () => ({
		get: (name) => (name === 'sa_access_token' ? { value: token } : undefined)
	});

	const authState = await nextServer.auth(asyncCookies);
	assert.equal(authState.isAuthenticated, true);
	assert.equal(authState.userId, 'usr_next_2');
	assert.equal(protect(authState, { permission: 'projects:read' }).appUserId, 'au_nx_2');
	assert.throws(
		() => protect(authState, { permission: 'billing:write' }),
		(err) => err instanceof AuthSdkError && err.code === 'FORBIDDEN'
	);

	const profile = await nextServer.currentUser(asyncCookies);
	assert.equal(profile?.email, 'next@example.test');

	const callbackGet = createCallbackRouteHandler({
		issuerUrl: 'https://auth.example.test',
		clientId: 'client_elysiar',
		redirectUri: 'https://elysiar.example.test/auth/callback',
		defaultRedirectTo: '/dashboard',
		fetch: async () =>
			new Response(
				JSON.stringify({
					access_token: 'next-cb-access',
					refresh_token: 'next-cb-refresh',
					token_type: 'Bearer',
					expires_in: 900
				}),
				{ status: 200, headers: { 'content-type': 'application/json' } }
			)
	});

	const pkceCookieVal = encodeURIComponent(
		JSON.stringify({ state: 'st_next', codeVerifier: 'pkce_verifier_next', redirectTo: '/billing' })
	);
	const cbRes = await callbackGet(
		new Request('https://elysiar.example.test/auth/callback?code=code_nx&state=st_next', {
			headers: { cookie: `sa_oidc_pkce=${pkceCookieVal}` }
		})
	);
	assert.equal(cbRes.status, 302);
	assert.equal(cbRes.headers.get('location'), '/billing');
	const setCookie = cbRes.headers.get('set-cookie') ?? '';
	assert.ok(setCookie.includes('sa_access_token=next-cb-access'));
	assert.ok(setCookie.includes('HttpOnly'));

	const webhookSecret = 'next-webhook-secret-test';
	let received = 0;
	const webhookPost = createWebhookRouteHandler({
		secret: webhookSecret,
		handlers: {
			'role.assigned': () => {
				received += 1;
			}
		}
	});
	const body = JSON.stringify({
		spec_version: '1.0',
		event_id: 'evt_nx_1',
		event_type: 'role.assigned',
		occurred_at: '2026-01-01T00:00:00Z',
		aggregate_type: 'user',
		aggregate_id: 'usr_next_2',
		aggregate_version: 2,
		data: { sub: 'usr_next_2', roles: ['admin'] }
	});
	const sig = await signWebhookPayload(webhookSecret, now, body);
	const whRes = await webhookPost(
		new Request('https://elysiar.example.test/api/webhooks/auth', {
			method: 'POST',
			headers: {
				'content-type': 'application/json',
				[WEBHOOK_EVENT_ID_HEADER]: 'evt_nx_1',
				[WEBHOOK_TIMESTAMP_HEADER]: String(now),
				[WEBHOOK_SIGNATURE_HEADER]: sig
			},
			body
		})
	);
	assert.equal(whRes.status, 200);
	assert.equal(received, 1);
});

test('Phase 08: Next.js auth() and protect() verify org_id, org_role, and org_permissions claims', async () => {
	const key = await createSigningKey('kid-next-org');
	const now = Math.floor(Date.now() / 1000);
	const orgToken = await signAccessToken(key, {
		iss: 'https://auth.example.test',
		sub: 'usr_org_1',
		aud: 'elysiar',
		azp: 'pub-elysiar',
		app_user_id: 'au_org_1',
		sid: 'sid_org_1',
		org_id: 'org_alpha',
		org_slug: 'alpha',
		org_role: 'org:admin',
		org_permissions: ['org:billing:write'],
		roles: ['user'],
		permissions: ['profile.read'],
		amr: ['pwd'],
		acr: 'aal1',
		ver: 1,
		iat: now,
		exp: now + 600,
		jti: 'jti_org_1'
	});

	const nextServer = createNextAuthServer({
		issuerUrl: 'https://auth.example.test',
		audience: 'elysiar',
		fetch: async () =>
			new Response(JSON.stringify({ keys: [key.jwk] }), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			})
	});

	const state = await nextServer.auth({
		get: (name) => (name === 'sa_access_token' ? { value: orgToken } : undefined)
	});
	assert.equal(state.isAuthenticated, true);
	assert.equal(state.orgId, 'org_alpha');
	assert.equal(state.orgSlug, 'alpha');
	assert.equal(state.orgRole, 'org:admin');
	assert.equal(state.has({ permission: 'org:billing:write', orgId: 'org_alpha' }), true);
	assert.equal(state.has({ permission: 'org:billing:write', orgId: 'org_beta' }), false);

	assert.equal(
		protect(state, { permission: 'org:billing:write', orgId: 'org_alpha' }).orgId,
		'org_alpha'
	);
	assert.throws(
		() => protect(state, { permission: 'org:billing:write', orgId: 'org_beta' }),
		(err) => err instanceof AuthSdkError && err.code === 'FORBIDDEN'
	);
});

test('Edge Runtime compliance: compiled next dist bundle imports zero Node built-ins', () => {
	const bannedImportPattern =
		/(?:from|import)\s*\(?\s*['"](?:node:|fs|crypto|http|https|net|tls|path|stream|buffer|os|child_process|zlib|url|util)['"]/;
	const distDir = join(import.meta.dirname, '..', 'dist');
	const files = readdirSync(distDir).filter((f) => f.endsWith('.js'));
	assert.ok(files.length > 0, `Expected built JS files in ${distDir}`);
	for (const file of files) {
		const content = readFileSync(join(distDir, file), 'utf8');
		assert.equal(
			bannedImportPattern.test(content),
			false,
			`Node built-in import detected in ${join(distDir, file)}`
		);
	}
});
