import { createAuthServer } from '@secure-auth/core/server';
import { elysiarSyncStore, type ElysiarSyncStore } from '../src/lib/db/sync.js';

export async function reconcileElysiarUsers(
	options: {
		issuerUrl: string;
		audience: string;
		secretKey: string;
		updatedSince?: string;
		store?: ElysiarSyncStore;
	}
): Promise<{ syncedCount: number; lastUpdatedAt: string | null }> {
	const store = options.store ?? elysiarSyncStore;
	const server = createAuthServer({
		issuerUrl: options.issuerUrl,
		audience: options.audience,
		secretKey: options.secretKey
	});

	let syncedCount = 0;
	let latest: string | null = store.lastReconciledAt;

	for await (const user of server.iterateUsersUpdatedSince(
		options.updatedSince ?? store.lastReconciledAt ?? undefined,
		{ pageSize: 100 }
	)) {
		store.upsertFromBackendItem(user);
		syncedCount += 1;
		if (!latest || user.updated_at > latest) {
			latest = user.updated_at;
		}
	}

	store.lastReconciledAt = latest;
	return { syncedCount, lastUpdatedAt: latest };
}
