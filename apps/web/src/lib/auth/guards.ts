import type { AuthUser } from '$lib/api/types';
import { canAccess } from './session';

export interface RouteGuardResult {
	allowed: boolean;
	redirectUrl?: string;
	reason?: 'unauthenticated' | 'forbidden' | 'already_authenticated';
}

export function evaluateRouteAccess(
	user: AuthUser | null | undefined,
	pathname: string
): RouteGuardResult {
	const normalized = pathname.toLowerCase();

	const isAuthRoute =
		normalized === '/login' ||
		normalized === '/register' ||
		normalized.startsWith('/login/') ||
		normalized.startsWith('/register/');

	if (user && isAuthRoute) {
		return {
			allowed: false,
			redirectUrl: '/dashboard',
			reason: 'already_authenticated'
		};
	}

	const isProtectedRoute =
		normalized.startsWith('/dashboard') ||
		normalized.startsWith('/settings') ||
		normalized.startsWith('/admin');

	if (!user && isProtectedRoute) {
		const redirectParam = encodeURIComponent(pathname);
		return {
			allowed: false,
			redirectUrl: `/login?redirect=${redirectParam}`,
			reason: 'unauthenticated'
		};
	}

	if (user && normalized.startsWith('/admin') && !canAccess(user, pathname)) {
		return {
			allowed: false,
			redirectUrl: '/dashboard',
			reason: 'forbidden'
		};
	}

	return { allowed: true };
}
