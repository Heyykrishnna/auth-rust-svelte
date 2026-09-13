import type { Handle } from '@sveltejs/kit';
import { redirect } from '@sveltejs/kit';
import type { AuthTokens, AuthUser } from '$lib/api/types';
import { evaluateRouteAccess } from '$lib/auth/guards';

const API_INTERNAL_URL =
	(typeof process !== 'undefined' && process.env.PUBLIC_API_BASE_URL) ||
	'http://127.0.0.1:8080';

export const handle: Handle = async ({ event, resolve }) => {
	const sessionToken = event.cookies.get('session_token');
	const refreshToken = event.cookies.get('refresh_token');

	event.locals.user = null;

	if (sessionToken) {
		try {
			const res = await event.fetch(`${API_INTERNAL_URL}/api/auth/me`, {
				headers: {
					Authorization: `Bearer ${sessionToken}`,
					Cookie: event.request.headers.get('cookie') || ''
				}
			});

			if (res.ok) {
				const user = (await res.json()) as AuthUser;
				event.locals.user = user;
			} else if (res.status === 401 && refreshToken) {
				const refreshRes = await event.fetch(`${API_INTERNAL_URL}/api/auth/refresh`, {
					method: 'POST',
					headers: {
						'Content-Type': 'application/json',
						Cookie: event.request.headers.get('cookie') || ''
					},
					body: JSON.stringify({ refresh_token: refreshToken })
				});

				if (refreshRes.ok) {
					const tokens = (await refreshRes.json()) as AuthTokens;
					if (tokens.access_token) {
						const retryUserRes = await event.fetch(`${API_INTERNAL_URL}/api/auth/me`, {
							headers: {
								Authorization: `Bearer ${tokens.access_token}`
							}
						});
						if (retryUserRes.ok) {
							event.locals.user = (await retryUserRes.json()) as AuthUser;
						}
					}
				}
			}
		} catch {
			event.locals.user = null;
		}
	}

	const guard = evaluateRouteAccess(event.locals.user, event.url.pathname);
	if (!guard.allowed && guard.redirectUrl) {
		throw redirect(303, guard.redirectUrl);
	}

	return resolve(event);
};
