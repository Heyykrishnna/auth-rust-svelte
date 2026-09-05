import type { AuthUser, AuthTokens } from '$lib/api/auth';
import { authApi } from '$lib/api/auth';

let _user = $state<AuthUser | null>(null);
let _loading = $state(false);
let _initialized = $state(false);

export const authStore = {
	get user() { return _user; },
	get loading() { return _loading; },
	get initialized() { return _initialized; },
	get isAuthenticated() { return _user !== null; }
};

export async function initAuth() {
	if (typeof window === 'undefined') return;

	_loading = true;
	try {
		// Session cookies are sent automatically with credentials: 'include'
		const user = await authApi.me();
		_user = user;
	} catch {
		// Attempt silent session refresh via HttpOnly refresh_token cookie
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
	// Session tokens are kept in HttpOnly, Secure, SameSite=Lax cookies by design.
	// Never persist auth tokens in browser localStorage.
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
