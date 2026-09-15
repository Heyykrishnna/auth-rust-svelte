import type { AuthTokens, AuthUser } from '$lib/api/types';
import { authApi } from '$lib/api/auth';
import { isAdmin } from '$lib/auth/session';

let _user = $state<AuthUser | null>(null);
let _loading = $state(false);
let _initialized = $state(false);

export const authStore = {
	get user() {
		return _user;
	},
	get loading() {
		return _loading;
	},
	get initialized() {
		return _initialized;
	},
	get isAuthenticated() {
		return _user !== null;
	},
	get isAdmin() {
		return isAdmin(_user);
	},
	get roles() {
		return _user?.roles || [];
	},
	get permissions() {
		return _user?.permissions || [];
	}
};

export function syncServerUser(user: AuthUser | null | undefined) {
	if (user !== undefined) {
		_user = user;
		_initialized = true;
	}
}

export async function initAuth() {
	if (typeof window === 'undefined') return;

	if (!_user) {
		_loading = true;
	}

	try {
		const user = await authApi.me();
		_user = user;
	} catch {
		try {
			await authApi.refresh();
			const user = await authApi.me();
			_user = user;
		} catch {
			_user = null;
		}
	} finally {
		_loading = false;
		_initialized = true;
	}
}

export function setAuth(user: AuthUser, _tokens?: AuthTokens) {
	_user = user;
	_initialized = true;
}

export function clearAuth() {
	_user = null;
}

export async function tryRefresh(): Promise<boolean> {
	try {
		await authApi.refresh();
		const user = await authApi.me();
		_user = user;
		return true;
	} catch {
		clearAuth();
		return false;
	}
}

export async function logout() {
	_loading = true;
	try {
		await authApi.logout();
	} finally {
		clearAuth();
		_loading = false;
	}
}
