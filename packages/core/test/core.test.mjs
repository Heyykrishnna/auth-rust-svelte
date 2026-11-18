import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import {
	AuthApiError,
	AuthSdkError,
	RateLimitError,
	TokenExpiredError,
	TokenVerificationError,
	WebhookSignatureError,
	bytesToBase64Url,
	utf8ToBytes
} from '../dist/index.js';
import { createAuthClient } from '../dist/client.js';
import { createAuthServer } from '../dist/server.js';
import {
	WEBHOOK_EVENT_ID_HEADER,
	WEBHOOK_SIGNATURE_HEADER,
	WEBHOOK_TIMESTAMP_HEADER,
	createWebhookRouter,
	shouldApplyVersion,
	signWebhookPayload,
	syncUserFromClaims,
	verifyWebhookSignature
} from '../dist/webhooks.js';

async function generateEs256SigningKey(kid) {
	const keyPair = await crypto.subtle.generateKey(
		{ name: 'ECDSA', namedCurve: 'P-256' },
		true,
		['sign', 'verify']
	);
	const publicJwk = await crypto.subtle.exportKey('jwk', keyPair.publicKey);
	return {
		kid,
		privateKey: keyPair.privateKey,
		jwk: {
			kty: 'EC',
			crv: 'P-256',
			use: 'sig',
			alg: 'ES256',
			kid,
			x: publicJwk.x,
			y: publicJwk.y
		}
	};
}

async function signJwt(signingKey, payload, headerOverrides = {}) {
	const header = {
		alg: 'ES256',
		typ: 'at+jwt',
		kid: signingKey.kid,
		...headerOverrides
	};
	const encodedHeader = bytesToBase64Url(utf8ToBytes(JSON.stringify(header)));
	const encodedPayload = bytesToBase64Url(utf8ToBytes(JSON.stringify(payload)));
	const signingInput = `${encodedHeader}.${encodedPayload}`;
	const sigBuffer = await crypto.subtle.sign(
		{ name: 'ECDSA', hash: 'SHA-256' },
		signingKey.privateKey,
		utf8ToBytes(signingInput)
	);
	const encodedSig = bytesToBase64Url(new Uint8Array(sigBuffer));
	return `${signingInput}.${encodedSig}`;
}

test('verifyAccessToken verifies ES256 tokens and handles JWKS key rotation on kid miss', async () => {
	const keyV1 = await generateEs256SigningKey('kid-v1');
	const keyV2 = await generateEs256SigningKey('kid-v2');
	let activeKeys = [keyV1.jwk];
	let jwksFetches = 0;

	const mockFetch = async (input) => {
		const url = String(input);
		if (url.endsWith('/.well-known/jwks.json')) {
			jwksFetches += 1;
			return new Response(JSON.stringify({ keys: activeKeys }), {
				status: 200,
				headers: { 'content-type': 'application/json', etag: `"v${jwksFetches}"` }
			});
		}
		throw new Error(`Unexpected fetch URL: ${url}`);
	};

	const server = createAuthServer({
		issuerUrl: 'https://auth.example.test',
		audience: 'ferrum',
		minJwksRefreshIntervalMs: 0,
		fetch: mockFetch
	});

	const now = Math.floor(Date.now() / 1000);
	const baseClaims = {
		iss: 'https://auth.example.test',
		sub: 'usr_101',
		aud: 'ferrum',
		exp: now + 300,
		iat: now - 5,
		jti: 'jti_1',
		sid: 'sid_1',
		app_id: 'app_ferrum',
		app_user_id: 'au_101',
		roles: ['member'],
		permissions: ['posts:read'],
		ver: 1
	};

	const tokenV1 = await signJwt(keyV1, baseClaims);
	const verifiedV1 = await server.verifyAccessToken(tokenV1);
	assert.equal(verifiedV1.sub, 'usr_101');
	assert.deepEqual(verifiedV1.roles, ['member']);
	assert.equal(jwksFetches, 1);

	const verifiedV1Cached = await server.verifyAccessToken(tokenV1);
	assert.equal(verifiedV1Cached.sub, 'usr_101');
	assert.equal(jwksFetches, 1);

	activeKeys = [keyV1.jwk, keyV2.jwk];
	const tokenV2 = await signJwt(keyV2, { ...baseClaims, ver: 2 });
	const verifiedV2 = await server.verifyAccessToken(tokenV2);
	assert.equal(verifiedV2.ver, 2);
	assert.equal(jwksFetches, 2);
});

test('verifyAccessToken rejects expired, wrong-aud, wrong-iss, alg confusion, and tampered JWTs', async () => {
	const key = await generateEs256SigningKey('kid-sec');
	const server = createAuthServer({
		issuerUrl: 'https://auth.example.test',
		audience: 'ferrum',
		fetch: async () =>
			new Response(JSON.stringify({ keys: [key.jwk] }), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			})
	});

	const now = Math.floor(Date.now() / 1000);
	const validClaims = {
		iss: 'https://auth.example.test',
		sub: 'usr_1',
		aud: 'ferrum',
		exp: now + 300,
		iat: now - 5,
		jti: 'jti_sec',
		sid: 'sid_sec',
		app_id: 'app_1',
		app_user_id: 'au_1',
		roles: [],
		permissions: [],
		ver: 1
	};

	const expired = await signJwt(key, { ...validClaims, exp: now - 120 });
	await assert.rejects(() => server.verifyAccessToken(expired), TokenExpiredError);

	const wrongAud = await signJwt(key, { ...validClaims, aud: 'elysiar' });
	await assert.rejects(() => server.verifyAccessToken(wrongAud), (err) => {
		assert.ok(err instanceof TokenVerificationError);
		assert.equal(err.code, 'INVALID_AUDIENCE');
		return true;
	});

	const wrongIss = await signJwt(key, { ...validClaims, iss: 'https://evil.example.test' });
	await assert.rejects(() => server.verifyAccessToken(wrongIss), (err) => {
		assert.ok(err instanceof TokenVerificationError);
		assert.equal(err.code, 'INVALID_ISSUER');
		return true;
	});

	const wrongAlg = await signJwt(key, validClaims, { alg: 'HS256' });
	await assert.rejects(() => server.verifyAccessToken(wrongAlg), (err) => {
		assert.ok(err instanceof TokenVerificationError);
		assert.equal(err.code, 'INVALID_ALG');
		return true;
	});

	const good = await signJwt(key, validClaims);
	const parts = good.split('.');
	const tamperedPayload = bytesToBase64Url(
		utf8ToBytes(JSON.stringify({ ...validClaims, roles: ['super_admin'] }))
	);
	const tampered = `${parts[0]}.${tamperedPayload}.${parts[2]}`;
	await assert.rejects(() => server.verifyAccessToken(tampered), (err) => {
		assert.ok(err instanceof TokenVerificationError);
		assert.equal(err.code, 'INVALID_SIGNATURE');
		return true;
	});

	const idToken = await signJwt(
		key,
		{
			iss: 'https://auth.example.test',
			sub: 'usr_1',
			aud: 'ferrum',
			exp: now + 300,
			iat: now,
			nonce: 'nonce-abc',
			email: 'alice@example.test',
			email_verified: true
		},
		{ typ: 'JWT' }
	);
	const verifiedId = await server.verifyIdToken(idToken, { expectedNonce: 'nonce-abc' });
	assert.equal(verifiedId.email, 'alice@example.test');
	await assert.rejects(
		() => server.verifyIdToken(idToken, { expectedNonce: 'nonce-wrong' }),
		(err) => err instanceof TokenVerificationError && err.code === 'INVALID_NONCE'
	);
});

test('createAuthClient performs PKCE S256 redirect flow and single-flight refresh deduplication', async () => {
	let refreshCalls = 0;
	let capturedTokenBody = '';
	const key = await generateEs256SigningKey('kid-oidc');
	let capturedNonce = '';

	const mockFetch = async (input, init) => {
		const url = String(input);
		if (url.endsWith('/oauth2/token')) {
			capturedTokenBody = String(init?.body ?? '');
			const now = Math.floor(Date.now() / 1000);
			const idToken = await signJwt(
				key,
				{
					iss: 'https://auth.example.test',
					sub: 'usr_oidc',
					aud: 'client_ferrum',
					exp: now + 600,
					iat: now,
					nonce: capturedNonce,
					email: 'sso@example.test',
					email_verified: true
				},
				{ typ: 'JWT' }
			);
			const accessToken = await signJwt(key, {
				iss: 'https://auth.example.test',
				sub: 'usr_oidc',
				aud: 'ferrum',
				exp: now + 900,
				iat: now,
				jti: 'jti_oidc_1',
				sid: 'sid_oidc_1',
				app_id: 'app_ferrum',
				app_user_id: 'au_oidc_1',
				roles: ['member'],
				permissions: ['posts:read'],
				ver: 1
			});
			return new Response(
				JSON.stringify({
					access_token: accessToken,
					token_type: 'Bearer',
					expires_in: 900,
					scope: 'openid profile email',
					refresh_token: 'refresh-oidc-1',
					id_token: idToken
				}),
				{ status: 200, headers: { 'content-type': 'application/json' } }
			);
		}
		if (url.endsWith('/v1/client/auth/refresh')) {
			refreshCalls += 1;
			await new Promise((resolve) => setTimeout(resolve, 15));
			return new Response(
				JSON.stringify({
					access_token: `access-rotated-${refreshCalls}`,
					refresh_token: `refresh-rotated-${refreshCalls}`,
					token_type: 'Bearer',
					expires_in: 900
				}),
				{ status: 200, headers: { 'content-type': 'application/json' } }
			);
		}
		if (url.endsWith('/v1/client/auth/sign-in')) {
			return new Response(
				JSON.stringify({
					error: {
						code: 'RATE_LIMITED',
						message: 'Too many requests',
						request_id: 'req_429'
					}
				}),
				{
					status: 429,
					headers: { 'content-type': 'application/json', 'retry-after': '42' }
				}
			);
		}
		throw new Error(`Unexpected URL: ${url}`);
	};

	const client = createAuthClient({
		issuerUrl: 'https://auth.example.test',
		publishableKey: 'pub-key-ferrum-test',
		clientId: 'client_ferrum',
		redirectUri: 'https://ferrum.example.test/auth/callback',
		fetch: mockFetch
	});

	const authorizeUrl = new URL(await client.signInWithRedirect());
	assert.equal(authorizeUrl.pathname, '/oauth2/authorize');
	assert.equal(authorizeUrl.searchParams.get('code_challenge_method'), 'S256');
	assert.ok((authorizeUrl.searchParams.get('code_challenge') ?? '').length >= 43);
	const state = authorizeUrl.searchParams.get('state');
	capturedNonce = authorizeUrl.searchParams.get('nonce') ?? '';
	assert.ok(state && capturedNonce);

	const tokens = await client.handleRedirectCallback(
		`https://ferrum.example.test/auth/callback?code=auth_code_123&state=${encodeURIComponent(state)}`
	);
	assert.equal(tokens.access_token.split('.').length, 3);
	assert.ok(capturedTokenBody.includes('code_verifier='));

	const concurrent = await Promise.all([
		client.refreshSession(),
		client.refreshSession(),
		client.refreshSession(),
		client.refreshSession(),
		client.refreshSession()
	]);
	assert.equal(refreshCalls, 1);
	for (const result of concurrent) {
		assert.equal(result.access_token, 'access-rotated-1');
	}

	await assert.rejects(
		() => client.signInWithPassword({ email: 'a@example.test', password: 'Password123!' }),
		(err) => err instanceof RateLimitError && err.retryAfterSecs === 42
	);
});

test('createAuthServer Backend Sync API paginates iterateUsersUpdatedSince and requires secretKey', async () => {
	const pages = [
		{
			data: [
				{
					sub: 'u_1',
					app_user_id: 'au_1',
					email: 'one@example.test',
					email_verified: true,
					display_name: 'One',
					avatar_url: null,
					status: 'active',
					roles: ['member'],
					external_id: null,
					public_metadata: {},
					private_metadata: {},
					version: 1,
					updated_at: '2026-01-02T00:00:00Z'
				}
			],
			next_cursor: 'cursor_2',
			has_more: true
		},
		{
			data: [
				{
					sub: 'u_2',
					app_user_id: 'au_2',
					email: 'two@example.test',
					email_verified: true,
					display_name: 'Two',
					avatar_url: null,
					status: 'active',
					roles: ['admin'],
					external_id: 'ext_2',
					public_metadata: { plan: 'pro' },
					private_metadata: {},
					version: 3,
					updated_at: '2026-01-03T00:00:00Z'
				}
			],
			next_cursor: null,
			has_more: false
		}
	];

	const serverNoSecret = createAuthServer({
		issuerUrl: 'https://auth.example.test',
		audience: 'ferrum'
	});
	await assert.rejects(() => serverNoSecret.listUsers(), AuthSdkError);

	let pageIdx = 0;
	const server = createAuthServer({
		issuerUrl: 'https://auth.example.test',
		audience: 'ferrum',
		secretKey: 'backend-test-credential-value',
		fetch: async (input, init) => {
			const headers = new Headers(init?.headers);
			assert.equal(headers.get('authorization'), 'Bearer backend-test-credential-value');
			const url = String(input);
			if (url.includes('/v1/backend/users')) {
				const body = pages[pageIdx++];
				return new Response(JSON.stringify(body), {
					status: 200,
					headers: { 'content-type': 'application/json' }
				});
			}
			return new Response(
				JSON.stringify({ error: { code: 'NOT_FOUND', message: 'Missing resource' } }),
				{ status: 404, headers: { 'content-type': 'application/json' } }
			);
		}
	});

	const collected = [];
	for await (const user of server.iterateUsersUpdatedSince('2026-01-01T00:00:00Z', {
		pageSize: 1
	})) {
		collected.push(user.app_user_id);
	}
	assert.deepEqual(collected, ['au_1', 'au_2']);
	await assert.rejects(() => server.getSession('missing'), AuthApiError);
});

test('webhooks verify HMAC-SHA256 with dual-secret rotation, reject tamper/replay, and guard versions', async () => {
	const primarySecret = 'webhook-signing-material-primary';
	const oldSecret = 'webhook-signing-material-previous';
	const now = Math.floor(Date.now() / 1000);
	const payload = JSON.stringify({
		spec_version: '1.0',
		event_id: 'evt_001',
		event_type: 'user.updated',
		occurred_at: '2026-01-01T00:00:00Z',
		aggregate_type: 'user',
		aggregate_id: 'u_1',
		aggregate_version: 2,
		data: { sub: 'u_1', app_user_id: 'au_1', version: 2, roles: ['editor'] }
	});

	const sigPrimary = await signWebhookPayload(primarySecret, now, payload);
	const verifiedPrimary = await verifyWebhookSignature({
		payload,
		headers: {
			[WEBHOOK_EVENT_ID_HEADER]: 'evt_001',
			[WEBHOOK_TIMESTAMP_HEADER]: String(now),
			[WEBHOOK_SIGNATURE_HEADER]: sigPrimary
		},
		secret: [primarySecret, oldSecret],
		nowSecs: now + 10
	});
	assert.equal(verifiedPrimary.eventId, 'evt_001');

	const sigOld = await signWebhookPayload(oldSecret, now, payload);
	const verifiedOld = await verifyWebhookSignature({
		payload,
		headers: {
			[WEBHOOK_EVENT_ID_HEADER]: 'evt_001',
			[WEBHOOK_TIMESTAMP_HEADER]: String(now),
			[WEBHOOK_SIGNATURE_HEADER]: sigOld
		},
		secret: [primarySecret, oldSecret],
		nowSecs: now + 10
	});
	assert.equal(verifiedOld.timestamp, now);

	await assert.rejects(
		() =>
			verifyWebhookSignature({
				payload: `${payload} `,
				headers: {
					[WEBHOOK_EVENT_ID_HEADER]: 'evt_001',
					[WEBHOOK_TIMESTAMP_HEADER]: String(now),
					[WEBHOOK_SIGNATURE_HEADER]: sigPrimary
				},
				secret: primarySecret,
				nowSecs: now + 10
			}),
		WebhookSignatureError
	);

	await assert.rejects(
		() =>
			verifyWebhookSignature({
				payload,
				headers: {
					[WEBHOOK_EVENT_ID_HEADER]: 'evt_001',
					[WEBHOOK_TIMESTAMP_HEADER]: String(now - 600),
					[WEBHOOK_SIGNATURE_HEADER]: sigPrimary
				},
				secret: primarySecret,
				toleranceSecs: 300,
				nowSecs: now
			}),
		WebhookSignatureError
	);

	const processed = new Set();
	let handledCount = 0;
	const router = createWebhookRouter({
		secret: [primarySecret, oldSecret],
		isEventProcessed: (id) => processed.has(id),
		markEventProcessed: (id) => {
			processed.add(id);
		},
		handlers: {
			'user.updated': () => {
				handledCount += 1;
			}
		}
	});

	const headers = {
		[WEBHOOK_EVENT_ID_HEADER]: 'evt_001',
		[WEBHOOK_TIMESTAMP_HEADER]: String(now),
		[WEBHOOK_SIGNATURE_HEADER]: sigPrimary
	};
	const first = await router.handle({ payload, headers });
	const duplicate = await router.handle({ payload, headers });
	assert.equal(first.status, 'processed');
	assert.equal(duplicate.status, 'duplicate');
	assert.equal(handledCount, 1);

	assert.equal(shouldApplyVersion(null, 1), true);
	assert.equal(shouldApplyVersion(2, 1), false);
	assert.equal(shouldApplyVersion(2, 2), true);
	assert.equal(shouldApplyVersion(2, 3), true);

	const projected = syncUserFromClaims({
		iss: 'https://auth.example.test',
		sub: 'u_9',
		aud: 'ferrum',
		exp: now + 300,
		iat: now,
		jti: 'jti_9',
		sid: 'sid_9',
		app_id: 'app_ferrum',
		app_user_id: 'au_9',
		roles: ['admin'],
		permissions: ['users:manage'],
		ver: 4
	});
	assert.equal(projected.sub, 'u_9');
	assert.equal(projected.identityVersion, 4);
});

test('Edge Runtime compliance: compiled core dist bundle imports zero Node built-ins', () => {
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
