import { apiClient, type RequestOptions } from './client';
import type { PaginationQuery, UpdateProfileRequest, UserProfile } from './types';

export const usersApi = {
	getMe: (options?: RequestOptions) =>
		options
			? apiClient.get<UserProfile>('/api/users/me', options)
			: apiClient.get<UserProfile>('/api/users/me'),

	updateProfile: (data: UpdateProfileRequest, options?: RequestOptions) =>
		options
			? apiClient.put<UserProfile>('/api/users/profile', data, options)
			: apiClient.put<UserProfile>('/api/users/profile', data),

	listUsers: (query?: PaginationQuery, options?: RequestOptions) => {
		const searchParams = new URLSearchParams();
		if (query?.limit !== undefined) searchParams.set('limit', String(query.limit));
		if (query?.offset !== undefined) searchParams.set('offset', String(query.offset));
		const queryStr = searchParams.toString();
		const path = queryStr ? `/api/users?${queryStr}` : '/api/users';
		return options
			? apiClient.get<UserProfile[]>(path, options)
			: apiClient.get<UserProfile[]>(path);
	},

	deleteUser: (userId: string, options?: RequestOptions) =>
		options
			? apiClient.delete<void>(`/api/users/${userId}`, options)
			: apiClient.delete<void>(`/api/users/${userId}`)
};
