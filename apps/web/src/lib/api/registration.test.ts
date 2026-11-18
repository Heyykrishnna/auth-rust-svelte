import { describe, expect, it, vi } from 'vitest';
import { authApi } from './auth';

const email = 'register@example.test';

function registrationTransport() {
	let cookieToken: string | undefined;
	return vi.fn<typeof fetch>(async (input, init) => {
		const url = new URL(String(input), 'http://localhost');
		if (url.pathname === '/api/auth/csrf') {
			cookieToken = 'fresh-csrf-token';
			return Response.json({ csrf_token: cookieToken });
		}
		const submittedToken = new Headers(init?.headers).get('x-csrf-token');
		if (!cookieToken || submittedToken !== cookieToken || init?.credentials !== 'include') {
			return Response.json(
				{ error: { code: 'CSRF_TOKEN_INVALID', message: 'Missing or invalid CSRF token' } },
				{ status: 403 }
			);
		}
		return Response.json({ message: 'Registration request accepted' });
	});
}

describe('registration with a fresh session', () => {
	it.each([
		{ path: '/api/auth/register/verify', payload: { email, code: '123456' } },
		{ path: '/api/auth/register/resend-otp', payload: { email } }
	])('provides the CSRF cookie and header before calling $path', async ({ path, payload }) => {
		const customFetch = registrationTransport();
		const options = { customFetch };
		if (payload.code) {
			await authApi.verifyRegisterOtp({ email, code: payload.code }, options);
		} else {
			await authApi.resendRegisterOtp(payload, options);
		}

		const [bootstrap, submission] = customFetch.mock.calls;
		expect(customFetch).toHaveBeenCalledTimes(2);
		expect(new URL(String(bootstrap[0]), 'http://localhost').pathname).toBe('/api/auth/csrf');
		expect(bootstrap[1]?.credentials).toBe('include');
		expect(new URL(String(submission[0]), 'http://localhost').pathname).toBe(path);
		expect(submission[1]?.method).toBe('POST');
		expect(submission[1]?.body).toBe(JSON.stringify(payload));
	});
});
