import { describe, it, expect } from 'vitest';
import {
	hasRole,
	hasPermission,
	isAdmin,
	canAccess,
	getUserInitials,
	formatUserDisplayName
} from './session';
import type { AuthUser } from '$lib/api/types';

describe('auth session helpers', () => {
	const standardUser: AuthUser = {
		id: 'u1',
		email: 'user@example.com',
		display_name: 'Regular User',
		avatar_url: null,
		email_verified: true,
		status: 'active',
		created_at: '2026-01-01T00:00:00Z',
		roles: ['user'],
		permissions: ['profile:read', 'profile:write']
	};

	const adminUser: AuthUser = {
		id: 'u2',
		email: 'admin@example.com',
		display_name: 'Admin Boss',
		avatar_url: null,
		email_verified: true,
		status: 'active',
		created_at: '2026-01-01T00:00:00Z',
		roles: ['admin', 'user'],
		permissions: ['users:read', 'users:delete']
	};

	it('detects user role correctly', () => {
		expect(hasRole(standardUser, 'user')).toBe(true);
		expect(hasRole(standardUser, 'admin')).toBe(false);
		expect(hasRole(adminUser, 'admin')).toBe(true);
		expect(hasRole(null, 'admin')).toBe(false);
	});

	it('detects admin privileges correctly', () => {
		expect(isAdmin(adminUser)).toBe(true);
		expect(isAdmin(standardUser)).toBe(false);
		expect(isAdmin(null)).toBe(false);
	});

	it('detects permissions with admin inheritance', () => {
		expect(hasPermission(standardUser, 'profile:read')).toBe(true);
		expect(hasPermission(standardUser, 'users:delete')).toBe(false);
		expect(hasPermission(adminUser, 'users:delete')).toBe(true);
		expect(hasPermission(adminUser, 'custom:permission')).toBe(true);
	});

	it('evaluates route clearance properly', () => {
		expect(canAccess(null, '/')).toBe(true);
		expect(canAccess(null, '/login')).toBe(true);

		expect(canAccess(null, '/dashboard')).toBe(false);
		expect(canAccess(standardUser, '/dashboard')).toBe(true);
		expect(canAccess(standardUser, '/settings')).toBe(true);

		expect(canAccess(standardUser, '/admin')).toBe(false);
		expect(canAccess(adminUser, '/admin')).toBe(true);
	});

	it('extracts user initials and display names cleanly', () => {
		expect(getUserInitials(standardUser)).toBe('RU');
		expect(getUserInitials({ ...standardUser, display_name: 'Single' })).toBe('SI');
		expect(formatUserDisplayName(standardUser)).toBe('Regular User');
		expect(formatUserDisplayName(null)).toBe('Anonymous');
	});
});
