import { authMiddleware } from '@secure-auth/next/middleware';

export const middleware = authMiddleware({
	issuerUrl: process.env['AUTH_ISSUER_URL'] ?? 'http://localhost:8080',
	audience: process.env['AUTH_AUDIENCE'] ?? 'elysiar',
	publishableKey: process.env['AUTH_PUBLISHABLE_KEY'],
	protectedRoutes: ['/dashboard', '/billing'],
	publicRoutes: ['/', '/login', '/auth/callback', '/api/webhooks/auth'],
	loginPath: '/login'
});

export const config = {
	matcher: ['/dashboard/:path*', '/billing/:path*']
};

