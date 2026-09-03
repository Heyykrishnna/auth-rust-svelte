import type { AuthUser, AuthTokens } from '$lib/api/auth';
import { authApi } from '$lib/api/auth';

let _user = $state<AuthUser | null>(null);
let _tokens = $state<AuthTokens | null>(null);
let _loading = $state(false);
let _initialized = $state(false);

export const authStore = {
	get user() { return _user; },
	get tokens() { return _tokens; },
	get loading() { return _loading; },
	get initialized() { return _initialized; },
	get isAuthenticated() { return _user !== null && _tokens !== null; },
	get accessToken() { return _tokens?.access_token ?? null; }
};

export function initAuth() {
	if (typeof localStorage === 'undefined') return;

	try {
		const stored = localStorage.getItem('auth_tokens');
		if (stored) {
			const tokens = JSON.parse(stored) as AuthTokens;
			_tokens = tokens;
			// Fetch user profile with stored token
			authApi.me(tokens.access_token)
				.then((user) => {
					_user = user;
				})
				.catch(() => {
					// Token invalid — try refresh
					tryRefresh(tokens.refresh_token);
				})
				.finally(() => {
					_initialized = true;
				});
		} else {
			_initialized = true;
		}
	} catch {
		_initialized = true;
	}
}

export function setAuth(user: AuthUser, tokens: AuthTokens) {
	_user = user;
	_tokens = tokens;
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('auth_tokens', JSON.stringify(tokens));
	}
}

export function clearAuth() {
	_user = null;
	_tokens = null;
	if (typeof localStorage !== 'undefined') {
		localStorage.removeItem('auth_tokens');
	}
}

export async function tryRefresh(refreshToken: string): Promise<boolean> {
	try {
		const newTokens = await authApi.refresh({ refresh_token: refreshToken });
		_tokens = newTokens;
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem('auth_tokens', JSON.stringify(newTokens));
		}
		// Re-fetch user
		const user = await authApi.me(newTokens.access_token);
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
		if (_tokens?.access_token) {
			await authApi.logout(_tokens.access_token);
		}
	} finally {
		clearAuth();
		_loading = false;
	}
}
