const API_BASE = import.meta.env.PUBLIC_API_BASE_URL || '';

export class ApiError extends Error {
	public code?: string;

	constructor(
		public status: number,
		message: string,
		public body?: unknown
	) {
		super(message);
		this.name = 'ApiError';

		if (typeof body === 'object' && body !== null && 'error' in body) {
			const err = (body as { error: unknown }).error;
			if (typeof err === 'object' && err !== null && 'code' in err) {
				this.code = String((err as { code: unknown }).code);
			}
		}
	}
}

export interface ApiResponse<T> {
	data?: T;
	error?: string;
}

async function getAuthToken(): Promise<string | null> {
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
		credentials: 'include',
		body: options.body ? JSON.stringify(options.body) : undefined
	});

	if (!response.ok) {
		let errorBody: unknown;
		try {
			errorBody = await response.json();
		} catch {
			errorBody = await response.text();
		}

		let message = `HTTP ${response.status} ${response.statusText}`;
		if (typeof errorBody === 'object' && errorBody !== null) {
			const record = errorBody as Record<string, unknown>;
			if (
				typeof record.error === 'object' &&
				record.error !== null &&
				'message' in (record.error as Record<string, unknown>)
			) {
				message = String((record.error as Record<string, unknown>).message);
			} else if (typeof record.error === 'string') {
				message = record.error;
			} else if (typeof record.message === 'string') {
				message = record.message;
			}
		} else if (typeof errorBody === 'string' && errorBody.trim().length > 0) {
			message = errorBody;
		}

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
