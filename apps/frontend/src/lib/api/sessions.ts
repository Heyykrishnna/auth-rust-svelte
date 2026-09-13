import { apiClient, type RequestOptions } from './client';
import type { SessionItem } from './types';

export const sessionsApi = {
	listSessions: (options?: RequestOptions) =>
		options
			? apiClient.get<SessionItem[]>('/api/sessions', options)
			: apiClient.get<SessionItem[]>('/api/sessions'),

	revokeSession: (sessionId: string, options?: RequestOptions) =>
		options
			? apiClient.delete<void>(`/api/sessions/${sessionId}`, options)
			: apiClient.delete<void>(`/api/sessions/${sessionId}`)
};
