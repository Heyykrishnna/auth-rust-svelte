import {
	createAuthHandle,
	type SvelteKitRequestEventLike
} from '@secure-auth/svelte/server';
import { ferrumSyncStore } from './lib/db/sync.js';

const baseHandle = createAuthHandle({
	issuerUrl: process.env['AUTH_ISSUER_URL'] ?? 'http://localhost:8080',
	audience: process.env['AUTH_AUDIENCE'] ?? 'ferrum',
	publishableKey: process.env['AUTH_PUBLISHABLE_KEY'] ?? '',
	secretKey: process.env['AUTH_SECRET_KEY'],
	protectedRoutes: ['/dashboard', '/settings'],
	loginPath: '/login'
});

export async function handle(input: {
	event: SvelteKitRequestEventLike;
	resolve: (event: SvelteKitRequestEventLike) => Promise<Response> | Response;
}): Promise<Response> {
	return baseHandle({
		event: input.event,
		resolve: async (event) => {
			if (event.locals.auth?.isAuthenticated && event.locals.auth.claims) {
				ferrumSyncStore.upsertFromClaims(event.locals.auth.claims);
			}
			return input.resolve(event);
		}
	});
}

