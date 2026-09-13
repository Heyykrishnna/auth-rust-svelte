import { describe, it, expect, vi, beforeEach } from 'vitest';
import { sessionsApi } from './sessions';
import { apiClient } from './client';

vi.mock('./client', () => ({
	apiClient: {
		get: vi.fn(),
		delete: vi.fn()
	}
}));

describe('sessionsApi', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('calls listSessions with /api/sessions', async () => {
		await sessionsApi.listSessions();
		expect(apiClient.get).toHaveBeenCalledWith('/api/sessions');
	});

	it('calls revokeSession with session id in URL', async () => {
		const sessionId = 'session-uuid-1234';
		await sessionsApi.revokeSession(sessionId);
		expect(apiClient.delete).toHaveBeenCalledWith(`/api/sessions/${sessionId}`);
	});
});
