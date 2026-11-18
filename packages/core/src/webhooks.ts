import { constantTimeEqual, hmacSha256Hex, utf8ToBytes } from './encoding.js';
import { WebhookSignatureError } from './errors.js';
import type {
	IdTokenClaims,
	JitUserProjection,
	TenantAccessTokenClaims,
	WebhookEventEnvelope,
	WebhookEventType
} from './types.js';

export type {
	IdTokenClaims,
	JitUserProjection,
	TenantAccessTokenClaims,
	WebhookEventEnvelope,
	WebhookEventType
};

export const WEBHOOK_EVENT_ID_HEADER = 'x-auth-event-id';
export const WEBHOOK_TIMESTAMP_HEADER = 'x-auth-timestamp';
export const WEBHOOK_SIGNATURE_HEADER = 'x-auth-signature';
export const DEFAULT_WEBHOOK_TOLERANCE_SECS = 300;

export type HeaderSource = Headers | Record<string, string | string[] | undefined>;

export interface VerifyWebhookSignatureInput {
	payload: string | Uint8Array;
	headers: HeaderSource;
	secret: string | string[];
	toleranceSecs?: number;
	nowSecs?: number;
}

export type WebhookEventHandler<TData = Record<string, unknown>> = (
	event: WebhookEventEnvelope<TData>
) => Promise<void> | void;

export type WebhookHandlersMap = Partial<Record<WebhookEventType, WebhookEventHandler>> & {
	onEvent?: WebhookEventHandler;
};

export interface WebhookRouterConfig {
	secret: string | string[];
	toleranceSecs?: number;
	isEventProcessed?: (eventId: string) => Promise<boolean> | boolean;
	markEventProcessed?: (eventId: string, eventType: string) => Promise<void> | void;
	handlers: WebhookHandlersMap;
}

export interface WebhookDispatchResult {
	status: 'processed' | 'duplicate' | 'ignored';
	eventId: string;
	eventType: string;
}

function readHeader(headers: HeaderSource, name: string): string | undefined {
	if (typeof Headers !== 'undefined' && headers instanceof Headers) {
		return headers.get(name) ?? undefined;
	}
	const target = name.toLowerCase();
	for (const [key, val] of Object.entries(headers)) {
		if (key.toLowerCase() === target) {
			return Array.isArray(val) ? val[0] : val;
		}
	}
	return undefined;
}

export async function signWebhookPayload(
	secret: string,
	timestampSecs: number,
	rawBody: string | Uint8Array
): Promise<string> {
	const bodyBytes = typeof rawBody === 'string' ? utf8ToBytes(rawBody) : rawBody;
	const prefixBytes = utf8ToBytes(`${timestampSecs}.`);
	const combined = new Uint8Array(prefixBytes.length + bodyBytes.length);
	combined.set(prefixBytes, 0);
	combined.set(bodyBytes, prefixBytes.length);
	const digestHex = await hmacSha256Hex(secret, combined);
	return `v1=${digestHex}`;
}

export async function verifyWebhookSignature(
	input: VerifyWebhookSignatureInput
): Promise<{ timestamp: number; eventId: string | null }> {
	const tolerance = input.toleranceSecs ?? DEFAULT_WEBHOOK_TOLERANCE_SECS;
	const rawTs = readHeader(input.headers, WEBHOOK_TIMESTAMP_HEADER);
	const rawSig = readHeader(input.headers, WEBHOOK_SIGNATURE_HEADER);
	const eventId = readHeader(input.headers, WEBHOOK_EVENT_ID_HEADER) ?? null;

	if (!rawTs || !/^-?\d+$/.test(rawTs.trim())) {
		throw new WebhookSignatureError('Missing or invalid X-Auth-Timestamp header', 'MISSING_TIMESTAMP');
	}
	const timestamp = Number.parseInt(rawTs.trim(), 10);
	const now = input.nowSecs ?? Math.floor(Date.now() / 1000);
	if (Math.abs(now - timestamp) > tolerance) {
		throw new WebhookSignatureError(
			'Webhook timestamp outside allowed tolerance window',
			'TIMESTAMP_EXPIRED'
		);
	}

	if (!rawSig) {
		throw new WebhookSignatureError('Missing X-Auth-Signature header', 'MISSING_SIGNATURE');
	}
	const presented = rawSig
		.split(',')
		.map((s) => s.trim())
		.filter((s) => s.startsWith('v1='));
	if (presented.length === 0) {
		throw new WebhookSignatureError('No v1 webhook signature found', 'MISSING_SIGNATURE');
	}

	const secrets = Array.isArray(input.secret) ? input.secret : [input.secret];
	for (const sec of secrets) {
		if (!sec) continue;
		const expected = await signWebhookPayload(sec, timestamp, input.payload);
		const expectedBytes = utf8ToBytes(expected);
		for (const candidate of presented) {
			if (constantTimeEqual(utf8ToBytes(candidate), expectedBytes)) {
				return { timestamp, eventId };
			}
		}
	}

	throw new WebhookSignatureError('Webhook signature verification failed');
}

export function shouldApplyVersion(
	localVersion: number | null | undefined,
	incomingVersion: number
): boolean {
	if (localVersion === null || localVersion === undefined) {
		return true;
	}
	return incomingVersion >= localVersion;
}

export function syncUserFromClaims(
	claims: TenantAccessTokenClaims | IdTokenClaims
): JitUserProjection {
	if ('email' in claims) {
		return {
			sub: claims.sub,
			appUserId: claims.app_user_id,
			email: claims.email,
			emailVerified: claims.email_verified,
			displayName: claims.name,
			avatarUrl: claims.picture ?? null,
			roles: [],
			permissions: [],
			identityVersion: claims.ver
		};
	}

	return {
		sub: claims.sub,
		appUserId: claims.app_user_id,
		roles: [...claims.roles],
		permissions: [...claims.permissions],
		identityVersion: claims.ver
	};
}

export function createWebhookRouter(config: WebhookRouterConfig) {
	return {
		async handle(input: {
			payload: string | Uint8Array;
			headers: HeaderSource;
			nowSecs?: number;
		}): Promise<WebhookDispatchResult> {
			await verifyWebhookSignature({
				payload: input.payload,
				headers: input.headers,
				secret: config.secret,
				toleranceSecs: config.toleranceSecs,
				nowSecs: input.nowSecs
			});

			const rawText =
				typeof input.payload === 'string'
					? input.payload
					: new TextDecoder().decode(input.payload);
			let envelope: WebhookEventEnvelope;
			try {
				envelope = JSON.parse(rawText) as WebhookEventEnvelope;
			} catch {
				throw new WebhookSignatureError('Invalid webhook JSON payload', 'INVALID_PAYLOAD');
			}

			if (!envelope.event_id || !envelope.event_type) {
				throw new WebhookSignatureError(
					'Webhook envelope missing event_id or event_type',
					'INVALID_PAYLOAD'
				);
			}

			if (config.isEventProcessed && (await config.isEventProcessed(envelope.event_id))) {
				return {
					status: 'duplicate',
					eventId: envelope.event_id,
					eventType: envelope.event_type
				};
			}

			const typedHandler = config.handlers[envelope.event_type as WebhookEventType];
			const fallbackHandler = config.handlers.onEvent;
			if (!typedHandler && !fallbackHandler) {
				if (config.markEventProcessed) {
					await config.markEventProcessed(envelope.event_id, envelope.event_type);
				}
				return {
					status: 'ignored',
					eventId: envelope.event_id,
					eventType: envelope.event_type
				};
			}

			if (typedHandler) {
				await typedHandler(envelope);
			}
			if (fallbackHandler) {
				await fallbackHandler(envelope);
			}
			if (config.markEventProcessed) {
				await config.markEventProcessed(envelope.event_id, envelope.event_type);
			}

			return {
				status: 'processed',
				eventId: envelope.event_id,
				eventType: envelope.event_type
			};
		}
	};
}
