import { describe, it, expect, vi } from 'vitest';
import { authApi } from './auth';
import { apiClient } from './client';

vi.mock('./client', () => ({
	apiClient: {
		post: vi.fn(),
		get: vi.fn()
	}
}));

describe('authApi', () => {
	it('calls register with expected endpoint and payload', async () => {
		const payload = {
			email: 'test@example.com',
			password: 'password123',
			display_name: 'Test User'
		};

		await authApi.register(payload);
		expect(apiClient.post).toHaveBeenCalledWith('/auth/register', payload);
	});

	it('calls login with expected endpoint and payload', async () => {
		const payload = {
			email: 'test@example.com',
			password: 'password123'
		};

		await authApi.login(payload);
		expect(apiClient.post).toHaveBeenCalledWith('/auth/login', payload);
	});

	it('calls logout with token header', async () => {
		await authApi.logout('sample-token');
		expect(apiClient.post).toHaveBeenCalledWith('/auth/logout', {}, { token: 'sample-token' });
	});

	it('calls me with token header', async () => {
		await authApi.me('sample-token');
		expect(apiClient.get).toHaveBeenCalledWith('/auth/me', { token: 'sample-token' });
	});

	it('calls health endpoint', async () => {
		await authApi.health();
		expect(apiClient.get).toHaveBeenCalledWith('/health');
	});
});
