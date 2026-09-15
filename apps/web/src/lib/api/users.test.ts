import { describe, it, expect, vi, beforeEach } from 'vitest';
import { usersApi } from './users';
import { apiClient } from './client';

vi.mock('./client', () => ({
	apiClient: {
		get: vi.fn(),
		put: vi.fn(),
		delete: vi.fn()
	}
}));

describe('usersApi', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('calls getMe with /api/users/me', async () => {
		await usersApi.getMe();
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/me');
	});

	it('calls updateProfile with expected payload', async () => {
		const payload = { display_name: 'Updated Name', avatar_url: null };
		await usersApi.updateProfile(payload);
		expect(apiClient.put).toHaveBeenCalledWith('/api/users/profile', payload);
	});

	it('calls listUsers with pagination query parameters', async () => {
		await usersApi.listUsers({ limit: 10, offset: 20 });
		expect(apiClient.get).toHaveBeenCalledWith('/api/users?limit=10&offset=20');
	});

	it('calls deleteUser with user id parameter', async () => {
		const userId = '123e4567-e89b-12d3-a456-426614174000';
		await usersApi.deleteUser(userId);
		expect(apiClient.delete).toHaveBeenCalledWith(`/api/users/${userId}`);
	});
});
