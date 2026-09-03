// ─── API Client ───────────────────────────────────────────────────────────────
// Type-safe HTTP client wrapping fetch with base URL, auth headers, and error handling

const API_BASE = import.meta.env.PUBLIC_API_BASE_URL || 'http://localhost:8080';

export class ApiError extends Error {
	constructor(
		public status: number,
		message: string,
		public body?: unknown
	) {
		super(message);
		this.name = 'ApiError';
	}
}

export interface ApiResponse<T> {
	data?: T;
	error?: string;
}

async function getAuthToken(): Promise<string | null> {
	if (typeof document !== 'undefined') {
		// Read from cookie (httpOnly cookie set by auth-api)
		// In SSR, this is handled server-side via request headers
		return null;
	}
	return null;
}

async function request<T>(
	method: string,
	path: string,
	options: {
		body?: unknown;
		token?: string;
		headers?: Record<string, string>;
	} = {}
): Promise<T> {
	const token = options.token ?? (await getAuthToken());

	const headers: Record<string, string> = {
		'Content-Type': 'application/json',
		...options.headers
	};

	if (token) {
		headers['Authorization'] = `Bearer ${token}`;
	}

	const response = await fetch(`${API_BASE}${path}`, {
		method,
		headers,
		credentials: 'include', // send cookies for session-based auth
		body: options.body ? JSON.stringify(options.body) : undefined
	});

	if (!response.ok) {
		let errorBody: unknown;
		try {
			errorBody = await response.json();
		} catch {
			errorBody = await response.text();
		}

		const message =
			typeof errorBody === 'object' && errorBody !== null && 'message' in errorBody
				? String((errorBody as { message: unknown }).message)
				: `HTTP ${response.status} ${response.statusText}`;

		throw new ApiError(response.status, message, errorBody);
	}

	if (response.status === 204) {
		return undefined as T;
	}

	return response.json() as Promise<T>;
}

export const apiClient = {
	get: <T>(path: string, options?: { token?: string }) => request<T>('GET', path, options),
	post: <T>(path: string, body: unknown, options?: { token?: string }) =>
		request<T>('POST', path, { body, ...options }),
	put: <T>(path: string, body: unknown, options?: { token?: string }) =>
		request<T>('PUT', path, { body, ...options }),
	patch: <T>(path: string, body: unknown, options?: { token?: string }) =>
		request<T>('PATCH', path, { body, ...options }),
	delete: <T>(path: string, options?: { token?: string }) => request<T>('DELETE', path, options)
};
