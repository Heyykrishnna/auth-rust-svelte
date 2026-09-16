import { describe, it, expect, vi, beforeEach } from 'vitest';
import { authApi } from './auth';
import { apiClient } from './client';

vi.mock('./client', () => ({
	apiClient: {
		post: vi.fn(),
		get: vi.fn()
	}
}));

describe('authApi', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('calls register with expected endpoint and payload', async () => {
		const payload = {
			email: 'test@example.com',
			password: 'password123',
			display_name: 'Test User'
		};

		await authApi.register(payload);
		expect(apiClient.post).toHaveBeenCalledWith('/api/auth/register', payload);
	});

	it('calls verifyRegisterOtp with expected endpoint and payload', async () => {
		const payload = {
			email: 'test@example.com',
			code: '123456'
		};

		await authApi.verifyRegisterOtp(payload);
		expect(apiClient.post).toHaveBeenCalledWith('/api/auth/register/verify', payload);
	});

	it('calls resendRegisterOtp with expected endpoint and payload', async () => {
		const payload = {
			email: 'test@example.com'
		};

		await authApi.resendRegisterOtp(payload);
		expect(apiClient.post).toHaveBeenCalledWith('/api/auth/register/resend-otp', payload);
	});


	it('calls login with expected endpoint and payload', async () => {
		const payload = {
			email: 'test@example.com',
			password: 'password123'
		};

		await authApi.login(payload);
		expect(apiClient.post).toHaveBeenCalledWith('/api/auth/login', payload);
	});

	it('calls logout with secure cookie support', async () => {
		await authApi.logout();
		expect(apiClient.post).toHaveBeenCalledWith('/api/auth/logout', {});
	});

	it('calls me with cookie support', async () => {
		await authApi.me();
		expect(apiClient.get).toHaveBeenCalledWith('/api/auth/me');
	});

	it('calls health endpoint', async () => {
		await authApi.health();
		expect(apiClient.get).toHaveBeenCalledWith('/health');
	});
});
