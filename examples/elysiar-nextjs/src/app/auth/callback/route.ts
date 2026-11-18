import { createCallbackRouteHandler } from '@secure-auth/next/server';

export const GET = createCallbackRouteHandler({
	issuerUrl: process.env['AUTH_ISSUER_URL'] ?? 'http://localhost:8080',
	clientId: process.env['AUTH_CLIENT_ID'] ?? '',
	redirectUri: process.env['AUTH_REDIRECT_URI'] ?? 'http://localhost:3001/auth/callback',
	defaultRedirectTo: '/dashboard'
});

