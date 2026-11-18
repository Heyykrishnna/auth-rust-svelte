import assert from 'node:assert/strict';
import test from 'node:test';
import {
	bytesToBase64Url,
	createAuthServer,
	shouldApplyVersion,
	signWebhookPayload,
	syncUserFromClaims,
	utf8ToBytes,
	WEBHOOK_EVENT_ID_HEADER,
	WEBHOOK_SIGNATURE_HEADER,
	WEBHOOK_TIMESTAMP_HEADER
} from '@secure-auth/core';
import {
	createAuthHandle,
	createWebhookEndpointHandler,
	serializeClientAuthState
} from '@secure-auth/svelte/server';
import { authMiddleware, INTERNAL_AUTH_HEADERS } from '../dist/middleware.js';
import {
	createNextAuthServer,
	createWebhookRouteHandler,
	protect
} from '../dist/server.js';

async function createIdpKey(kid = 'idp-es256-1') {
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

async function mintAccessJwt(idpKey, claims) {
	const header = { alg: 'ES256', typ: 'at+jwt', kid: idpKey.kid };
	const encHeader = bytesToBase64Url(utf8ToBytes(JSON.stringify(header)));
	const encPayload = bytesToBase64Url(utf8ToBytes(JSON.stringify(claims)));
	const signingInput = `${encHeader}.${encPayload}`;
	const sig = await crypto.subtle.sign(
		{ name: 'ECDSA', hash: 'SHA-256' },
		idpKey.privateKey,
		utf8ToBytes(signingInput)
	);
	return `${signingInput}.${bytesToBase64Url(new Uint8Array(sig))}`;
}

function createLocalDbProjection() {
	const users = new Map();
	const processed = new Set();
	return {
		users,
		processed,
		upsertClaims(claims) {
			const proj = syncUserFromClaims(claims);
			const existing = users.get(proj.sub);
			if (existing && !shouldApplyVersion(existing.version, proj.identityVersion)) {
				return existing;
			}
			const next = {
				sub: proj.sub,
				appUserId: proj.appUserId,
				roles: proj.roles,
				permissions: proj.permissions,
				version: proj.identityVersion
			};
			users.set(proj.sub, next);
			return next;
		},
		applyWebhook(event) {
			const existing = users.get(event.aggregate_id);
			if (existing && !shouldApplyVersion(existing.version, event.aggregate_version)) {
				return false;
			}
			users.set(event.aggregate_id, {
				sub: event.aggregate_id,
				appUserId: String(event.data.app_user_id ?? existing?.appUserId ?? event.aggregate_id),
				email: typeof event.data.email === 'string' ? event.data.email : existing?.email,
				roles: Array.isArray(event.data.roles) ? event.data.roles : (existing?.roles ?? []),
				permissions: existing?.permissions ?? [],
				version: event.aggregate_version
			});
			return true;
		}
	};
}

test('dual-app SSO & DB sync: Ferrum SvelteKit and Elysiar Next.js isolate audiences, sync webhooks, and reconcile', async () => {
	const idpKey = await createIdpKey();
	const issuerUrl = 'https://idp.example.test';
	const now = Math.floor(Date.now() / 1000);
	const sharedSub = 'usr_shared_sso_001';

	const ferrumToken = await mintAccessJwt(idpKey, {
		iss: issuerUrl,
		sub: sharedSub,
		aud: 'ferrum',
		exp: now + 600,
		iat: now,
		jti: 'jti_f1',
		sid: 'sid_ferrum_1',
		app_id: 'app_ferrum',
		app_user_id: 'au_ferrum_1',
		roles: ['member'],
		permissions: ['workspace:read'],
		ver: 1
	});

	const elysiarToken = await mintAccessJwt(idpKey, {
		iss: issuerUrl,
		sub: sharedSub,
		aud: 'elysiar',
		exp: now + 600,
		iat: now,
		jti: 'jti_e1',
		sid: 'sid_elysiar_1',
		app_id: 'app_elysiar',
		app_user_id: 'au_elysiar_1',
		roles: ['owner'],
		permissions: ['billing:write'],
		ver: 2
	});

	const jwksFetch = async (input) => {
		const url = String(input);
		if (url.endsWith('/.well-known/jwks.json')) {
			return new Response(JSON.stringify({ keys: [idpKey.jwk] }), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			});
		}
		if (url.includes('/v1/backend/users')) {
			return new Response(
				JSON.stringify({
					data: [
						{
							sub: sharedSub,
							app_user_id: 'au_elysiar_1',
							email: 'shared@example.test',
							email_verified: true,
							display_name: 'Shared User',
							avatar_url: null,
							status: 'active',
							roles: ['owner'],
							public_metadata: {},
							private_metadata: {},
							external_id: null,
							version: 3,
							updated_at: '2026-01-05T00:00:00Z'
						}
					],
					next_cursor: null,
					has_more: false
				}),
				{ status: 200, headers: { 'content-type': 'application/json' } }
			);
		}
		throw new Error(`Unexpected URL: ${url}`);
	};

	const ferrumDb = createLocalDbProjection();
	const elysiarDb = createLocalDbProjection();

	const ferrumHandle = createAuthHandle({
		issuerUrl,
		audience: 'ferrum',
		publishableKey: 'pub-ferrum',
		protectedRoutes: ['/dashboard'],
		fetch: jwksFetch
	});

	const ferrumCookies = new Map([['sa_access_token', ferrumToken]]);
	const ferrumEvent = {
		request: new Request('https://ferrum.example.test/dashboard'),
		url: new URL('https://ferrum.example.test/dashboard'),
		cookies: {
			get: (k) => ferrumCookies.get(k),
			set: (k, v) => ferrumCookies.set(k, v),
			delete: (k) => ferrumCookies.delete(k)
		},
		locals: {}
	};

	const ferrumRes = await ferrumHandle({
		event: ferrumEvent,
		resolve: (ev) => {
			ferrumDb.upsertClaims(ev.locals.auth.claims);
			return new Response(JSON.stringify(serializeClientAuthState(ev.locals)), { status: 200 });
		}
	});
	assert.equal(ferrumRes.status, 200);
	assert.equal(ferrumDb.users.get(sharedSub)?.appUserId, 'au_ferrum_1');
	assert.equal(ferrumDb.users.get(sharedSub)?.version, 1);

	const ferrumWebhookSecret = 'ferrum-sync-webhook-key';
	const ferrumWebhookPost = createWebhookEndpointHandler({
		secret: ferrumWebhookSecret,
		isEventProcessed: (id) => ferrumDb.processed.has(id),
		markEventProcessed: (id) => ferrumDb.processed.add(id),
		handlers: {
			'user.updated': (ev) => {
				ferrumDb.applyWebhook(ev);
			}
		}
	});

	const ferrumUpdatePayload = JSON.stringify({
		spec_version: '1.0',
		event_id: 'evt_ferrum_v2',
		event_type: 'user.updated',
		occurred_at: '2026-01-02T00:00:00Z',
		aggregate_type: 'user',
		aggregate_id: sharedSub,
		aggregate_version: 2,
		data: { app_user_id: 'au_ferrum_1', email: 'shared@example.test', roles: ['admin'] }
	});
	const ferrumSig = await signWebhookPayload(ferrumWebhookSecret, now, ferrumUpdatePayload);
	const ferrumWhRes = await ferrumWebhookPost({
		request: new Request('https://ferrum.example.test/api/webhooks/auth', {
			method: 'POST',
			headers: {
				'content-type': 'application/json',
				[WEBHOOK_EVENT_ID_HEADER]: 'evt_ferrum_v2',
				[WEBHOOK_TIMESTAMP_HEADER]: String(now),
				[WEBHOOK_SIGNATURE_HEADER]: ferrumSig
			},
			body: ferrumUpdatePayload
		})
	});
	assert.equal(ferrumWhRes.status, 200);
	assert.deepEqual(ferrumDb.users.get(sharedSub)?.roles, ['admin']);
	assert.equal(ferrumDb.users.get(sharedSub)?.version, 2);

	const elysiarMiddleware = authMiddleware({
		issuerUrl,
		audience: 'elysiar',
		protectedRoutes: ['/dashboard'],
		loginPath: '/login',
		fetch: jwksFetch
	});

	const crossAudienceAttempt = await elysiarMiddleware(
		new Request('https://elysiar.example.test/dashboard', {
			headers: { cookie: `sa_access_token=${encodeURIComponent(ferrumToken)}` }
		})
	);
	assert.equal(crossAudienceAttempt.status, 302);

	const elysiarOk = await elysiarMiddleware(
		new Request('https://elysiar.example.test/dashboard', {
			headers: { cookie: `sa_access_token=${encodeURIComponent(elysiarToken)}` }
		})
	);
	assert.equal(elysiarOk.status, 200);
	assert.equal(
		elysiarOk.headers.get(`x-middleware-request-${INTERNAL_AUTH_HEADERS.userId}`),
		sharedSub
	);

	const elysiarServer = createNextAuthServer({
		issuerUrl,
		audience: 'elysiar',
		secretKey: 'elysiar-backend-sync-credential',
		fetch: jwksFetch
	});
	const elysiarAuth = protect(
		await elysiarServer.auth({
			get: (k) => (k === 'sa_access_token' ? { value: elysiarToken } : undefined)
		}),
		{ permission: 'billing:write' }
	);
	elysiarDb.upsertClaims(elysiarAuth.claims);
	assert.equal(elysiarDb.users.get(sharedSub)?.version, 2);

	const elysiarWebhookSecret = 'elysiar-sync-webhook-key';
	const elysiarWebhookPost = createWebhookRouteHandler({
		secret: elysiarWebhookSecret,
		isEventProcessed: (id) => elysiarDb.processed.has(id),
		markEventProcessed: (id) => elysiarDb.processed.add(id),
		handlers: {
			'user.updated': (ev) => {
				elysiarDb.applyWebhook(ev);
			}
		}
	});
	const stalePayload = JSON.stringify({
		spec_version: '1.0',
		event_id: 'evt_elysiar_stale_v1',
		event_type: 'user.updated',
		occurred_at: '2026-01-01T00:00:00Z',
		aggregate_type: 'user',
		aggregate_id: sharedSub,
		aggregate_version: 1,
		data: { app_user_id: 'au_elysiar_1', roles: ['stale_role'] }
	});
	const staleSig = await signWebhookPayload(elysiarWebhookSecret, now, stalePayload);
	const staleRes = await elysiarWebhookPost(
		new Request('https://elysiar.example.test/api/webhooks/auth', {
			method: 'POST',
			headers: {
				'content-type': 'application/json',
				[WEBHOOK_EVENT_ID_HEADER]: 'evt_elysiar_stale_v1',
				[WEBHOOK_TIMESTAMP_HEADER]: String(now),
				[WEBHOOK_SIGNATURE_HEADER]: staleSig
			},
			body: stalePayload
		})
	);
	assert.equal(staleRes.status, 200);
	assert.deepEqual(elysiarDb.users.get(sharedSub)?.roles, ['owner']);

	const backendClient = createAuthServer({
		issuerUrl,
		audience: 'elysiar',
		secretKey: 'elysiar-backend-sync-credential',
		fetch: jwksFetch
	});
	for await (const item of backendClient.iterateUsersUpdatedSince('2026-01-01T00:00:00Z')) {
		const existing = elysiarDb.users.get(item.sub);
		if (!existing || shouldApplyVersion(existing.version, item.version)) {
			elysiarDb.users.set(item.sub, {
				sub: item.sub,
				appUserId: item.app_user_id,
				email: item.email,
				roles: item.roles,
				permissions: existing?.permissions ?? [],
				version: item.version
			});
		}
	}
	assert.equal(elysiarDb.users.get(sharedSub)?.version, 3);
	assert.equal(elysiarDb.users.get(sharedSub)?.email, 'shared@example.test');
});

