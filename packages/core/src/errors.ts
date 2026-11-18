export class AuthSdkError extends Error {
	readonly code: string;

	constructor(code: string, message: string) {
		super(message);
		this.name = new.target.name;
		this.code = code;
	}
}

export class AuthApiError extends AuthSdkError {
	readonly status: number;
	readonly requestId: string | null;
	readonly details: unknown;

	constructor(
		status: number,
		code: string,
		message: string,
		requestId: string | null = null,
		details: unknown = null
	) {
		super(code, message);
		this.status = status;
		this.requestId = requestId;
		this.details = details;
	}
}

export class TokenExpiredError extends AuthSdkError {
	readonly expiredAt: number;

	constructor(expiredAt: number, message = 'Token has expired') {
		super('TOKEN_EXPIRED', message);
		this.expiredAt = expiredAt;
	}
}

export class TokenVerificationError extends AuthSdkError {
	constructor(message: string, code = 'INVALID_TOKEN') {
		super(code, message);
	}
}

export class RateLimitError extends AuthApiError {
	readonly retryAfterSecs: number | null;

	constructor(
		message: string,
		retryAfterSecs: number | null = null,
		requestId: string | null = null
	) {
		super(429, 'TOO_MANY_REQUESTS', message, requestId, null);
		this.retryAfterSecs = retryAfterSecs;
	}
}

export class NetworkError extends AuthSdkError {
	override readonly cause: unknown;

	constructor(message: string, cause?: unknown) {
		super('NETWORK_ERROR', message);
		this.cause = cause;
	}
}

export class WebhookSignatureError extends AuthSdkError {
	constructor(message: string, code = 'INVALID_WEBHOOK_SIGNATURE') {
		super(code, message);
	}
}

