import type { AuthUser } from '$lib/api/types';

export function hasRole(user: AuthUser | null | undefined, role: string): boolean {
	if (!user || !user.roles) return false;
	const normalized = role.toLowerCase();
	return user.roles.some((r) => r.toLowerCase() === normalized);
}

export function hasPermission(user: AuthUser | null | undefined, permission: string): boolean {
	if (!user) return false;
	if (isAdmin(user)) return true;
	if (!user.permissions) return false;
	const normalized = permission.toLowerCase();
	return user.permissions.some((p) => p.toLowerCase() === normalized);
}

export function isAdmin(user: AuthUser | null | undefined): boolean {
	return hasRole(user, 'admin') || hasRole(user, 'administrator') || hasRole(user, 'superadmin');
}

export function canAccess(user: AuthUser | null | undefined, path: string): boolean {
	const normalizedPath = path.toLowerCase();

	if (
		normalizedPath === '/' ||
		normalizedPath.startsWith('/login') ||
		normalizedPath.startsWith('/register') ||
		normalizedPath.startsWith('/auth')
	) {
		return true;
	}

	if (!user) {
		return false;
	}

	if (normalizedPath.startsWith('/admin')) {
		return isAdmin(user) || hasPermission(user, 'users:read');
	}

	return true;
}

export function getUserInitials(user: AuthUser | null | undefined): string {
	if (!user) return '?';
	const name = user.display_name?.trim() || user.email?.trim() || '';
	if (!name) return '?';

	const parts = name.split(/\s+/).filter(Boolean);
	if (parts.length >= 2) {
		return `${parts[0][0]}${parts[1][0]}`.toUpperCase();
	}
	return name.substring(0, 2).toUpperCase();
}

export function formatUserDisplayName(user: AuthUser | null | undefined): string {
	if (!user) return 'Anonymous';
	return user.display_name || user.email.split('@')[0] || 'User';
}
