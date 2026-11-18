import { AuthApiError, NetworkError, RateLimitError } from './errors.js';
import type { ApiErrorBody } from './types.js';

export type FetchLike = typeof globalThis.fetch;

export interface HttpRequestOptions {
	method?: 'GET' | 'POST' | 'PATCH' | 'PUT' | 'DELETE';
	headers?: Record<string, string>;
	body?: string;
	timeoutMs?: number;
	maxRetries?: number;
	idempotent?: boolean;
}

export async function fetchWithPolicy(
	fetchImpl: FetchLike,
	url: string,
	options: HttpRequestOptions = {}
): Promise<Response> {
	const method = options.method ?? 'GET';
	const timeoutMs = options.timeoutMs ?? 10_000;
	const canRetry = options.idempotent ?? method === 'GET';
	const maxAttempts = canRetry ? Math.max(1, (options.maxRetries ?? 2) + 1) : 1;

	let lastError: unknown;
	for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
		const controller = new AbortController();
		const timer = setTimeout(() => controller.abort(), timeoutMs);
		try {
			const res = await fetchImpl(url, {
				method,
				headers: options.headers,
				body: options.body,
				signal: controller.signal
			});
			clearTimeout(timer);

			if (canRetry && attempt < maxAttempts && (res.status === 502 || res.status === 503 || res.status === 504)) {
				await delayMs(attempt * 150);
				continue;
			}
			return res;
		} catch (err) {
			clearTimeout(timer);
			lastError = err;
			if (attempt < maxAttempts) {
				await delayMs(attempt * 150);
				continue;
			}
		}
	}

	throw new NetworkError(
		lastError instanceof Error ? lastError.message : 'Network request failed',
		lastError
	);
}

export async function parseJsonResponse<T>(res: Response): Promise<T> {
	const text = await res.text();
	const parsed = text.length > 0 ? safeJsonParse(text) : undefined;

	if (!res.ok) {
		const errBody = (parsed && typeof parsed === 'object' ? parsed : {}) as ApiErrorBody;
		const code = errBody.error?.code ?? `HTTP_${res.status}`;
		const message = errBody.error?.message ?? (text || res.statusText || 'Request failed');
		const requestId = errBody.error?.request_id ?? res.headers.get('x-request-id');

		if (res.status === 429) {
			const retryHeader = res.headers.get('retry-after');
			const retryAfter = retryHeader ? Number.parseInt(retryHeader, 10) : null;
			throw new RateLimitError(
				message,
				Number.isFinite(retryAfter) ? retryAfter : null,
				requestId
			);
		}
		throw new AuthApiError(res.status, code, message, requestId, errBody.error?.details ?? null);
	}

	return parsed as T;
}

function safeJsonParse(text: string): unknown {
	try {
		return JSON.parse(text) as unknown;
	} catch {
		return undefined;
	}
}

function delayMs(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

