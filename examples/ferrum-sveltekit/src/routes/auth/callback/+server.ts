import { createOidcCallbackHandler } from '@secure-auth/svelte/server';

export const GET = createOidcCallbackHandler({
	issuerUrl: process.env['AUTH_ISSUER_URL'] ?? 'http://localhost:8080',
	clientId: process.env['AUTH_CLIENT_ID'] ?? '',
	redirectUri: process.env['AUTH_REDIRECT_URI'] ?? 'http://localhost:5174/auth/callback',
	defaultRedirectTo: '/dashboard'
});

