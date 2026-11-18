import {
	createNextAuthServer,
	type AuthSource
} from '@secure-auth/next/server';
import { elysiarSyncStore, type ElysiarLocalUser } from '../../lib/db/sync.js';

const nextAuth = createNextAuthServer({
	issuerUrl: process.env['AUTH_ISSUER_URL'] ?? 'http://localhost:8080',
	audience: process.env['AUTH_AUDIENCE'] ?? 'elysiar',
	secretKey: process.env['AUTH_SECRET_KEY'],
	publishableKey: process.env['AUTH_PUBLISHABLE_KEY']
});

export async function loadDashboardView(source: AuthSource): Promise<{
	userId: string;
	roles: string[];
	permissions: string[];
	localProfile: ElysiarLocalUser | null;
}> {
	const state = nextAuth.protect(await nextAuth.auth(source));
	const localProfile = state.claims ? elysiarSyncStore.upsertFromClaims(state.claims) : null;
	return {
		userId: state.userId ?? '',
		roles: state.roles,
		permissions: state.permissions,
		localProfile
	};
}

