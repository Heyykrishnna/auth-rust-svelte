import { describe, it, expect } from 'vitest';
import { evaluateRouteAccess } from './guards';
import type { AuthUser } from '$lib/api/types';

describe('evaluateRouteAccess', () => {
	const regularUser: AuthUser = {
		id: 'u1',
		email: 'member@example.com',
		display_name: 'Member',
		avatar_url: null,
		email_verified: true,
		status: 'active',
		created_at: '2026-01-01T00:00:00Z',
		roles: ['user']
	};

	const adminUser: AuthUser = {
		id: 'u2',
		email: 'admin@example.com',
		display_name: 'Admin',
		avatar_url: null,
		email_verified: true,
		status: 'active',
		created_at: '2026-01-01T00:00:00Z',
		roles: ['admin']
	};

	it('allows public routes for unauthenticated users', () => {
		const result = evaluateRouteAccess(null, '/');
		expect(result.allowed).toBe(true);
	});

	it('redirects unauthenticated users attempting to access protected dashboard', () => {
		const result = evaluateRouteAccess(null, '/dashboard');
		expect(result.allowed).toBe(false);
		expect(result.redirectUrl).toBe('/login?redirect=%2Fdashboard');
		expect(result.reason).toBe('unauthenticated');
	});

	it('redirects unauthenticated users attempting to access settings', () => {
		const result = evaluateRouteAccess(null, '/settings');
		expect(result.allowed).toBe(false);
		expect(result.redirectUrl).toBe('/login?redirect=%2Fsettings');
	});

	it('redirects unauthenticated users attempting to access admin', () => {
		const result = evaluateRouteAccess(null, '/admin');
		expect(result.allowed).toBe(false);
		expect(result.redirectUrl).toBe('/login?redirect=%2Fadmin');
	});

	it('redirects authenticated users away from /login to /dashboard', () => {
		const result = evaluateRouteAccess(regularUser, '/login');
		expect(result.allowed).toBe(false);
		expect(result.redirectUrl).toBe('/dashboard');
		expect(result.reason).toBe('already_authenticated');
	});

	it('redirects authenticated users away from /register to /dashboard', () => {
		const result = evaluateRouteAccess(regularUser, '/register');
		expect(result.allowed).toBe(false);
		expect(result.redirectUrl).toBe('/dashboard');
	});

	it('blocks standard authenticated user from /admin', () => {
		const result = evaluateRouteAccess(regularUser, '/admin');
		expect(result.allowed).toBe(false);
		expect(result.redirectUrl).toBe('/dashboard');
		expect(result.reason).toBe('forbidden');
	});

	it('permits admin user to access /admin', () => {
		const result = evaluateRouteAccess(adminUser, '/admin');
		expect(result.allowed).toBe(true);
	});
});
