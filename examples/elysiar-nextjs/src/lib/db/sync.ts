import {
	shouldApplyVersion,
	syncUserFromClaims,
	type BackendUserSyncItem,
	type TenantAccessTokenClaims,
	type WebhookEventEnvelope
} from '@secure-auth/core';

export interface ElysiarLocalUser {
	authUserId: string;
	appUserId: string;
	email: string | null;
	displayName: string | null;
	roles: string[];
	permissions: string[];
	status: 'active' | 'suspended' | 'deleted';
	authVersion: number;
	updatedAt: string;
}

export interface ElysiarSyncStore {
	users: Map<string, ElysiarLocalUser>;
	processedEvents: Set<string>;
	lastReconciledAt: string | null;
	isEventProcessed(eventId: string): boolean;
	markEventProcessed(eventId: string): void;
	upsertFromClaims(claims: TenantAccessTokenClaims): ElysiarLocalUser;
	applyWebhookEvent(event: WebhookEventEnvelope): void;
	upsertFromBackendItem(item: BackendUserSyncItem): ElysiarLocalUser;
}

export function createElysiarSyncStore(): ElysiarSyncStore {
	const users = new Map<string, ElysiarLocalUser>();
	const processedEvents = new Set<string>();
	let lastReconciledAt: string | null = null;

	function upsertUser(candidate: ElysiarLocalUser): ElysiarLocalUser {
		const existing = users.get(candidate.authUserId);
		if (existing && !shouldApplyVersion(existing.authVersion, candidate.authVersion)) {
			return existing;
		}
		users.set(candidate.authUserId, candidate);
		return candidate;
	}

	return {
		users,
		processedEvents,
		get lastReconciledAt() {
			return lastReconciledAt;
		},
		set lastReconciledAt(value: string | null) {
			lastReconciledAt = value;
		},
		isEventProcessed(eventId: string): boolean {
			return processedEvents.has(eventId);
		},
		markEventProcessed(eventId: string): void {
			processedEvents.add(eventId);
		},
		upsertFromClaims(claims: TenantAccessTokenClaims): ElysiarLocalUser {
			const projection = syncUserFromClaims(claims);
			const existing = users.get(projection.sub);
			return upsertUser({
				authUserId: projection.sub,
				appUserId: projection.appUserId,
				email: projection.email ?? existing?.email ?? null,
				displayName: projection.displayName ?? existing?.displayName ?? null,
				roles: projection.roles,
				permissions: projection.permissions,
				status: existing?.status ?? 'active',
				authVersion: projection.identityVersion,
				updatedAt: new Date().toISOString()
			});
		},
		applyWebhookEvent(event: WebhookEventEnvelope): void {
			const data = event.data;
			const authUserId =
				typeof data['sub'] === 'string'
					? data['sub']
					: typeof data['user_id'] === 'string'
						? data['user_id']
						: event.aggregate_id;
			const appUserId =
				typeof data['app_user_id'] === 'string' ? data['app_user_id'] : authUserId;
			if (!authUserId || !appUserId) {
				return;
			}
			const incomingVersion =
				typeof data['version'] === 'number' ? data['version'] : event.aggregate_version;
			const existing = users.get(authUserId);
			if (existing && !shouldApplyVersion(existing.authVersion, incomingVersion)) {
				return;
			}

			const status: ElysiarLocalUser['status'] =
				event.event_type === 'user.deleted'
					? 'deleted'
					: event.event_type === 'user.suspended'
						? 'suspended'
						: 'active';

			const roles = Array.isArray(data['roles'])
				? data['roles'].filter((v): v is string => typeof v === 'string')
				: (existing?.roles ?? []);
			const permissions = Array.isArray(data['permissions'])
				? data['permissions'].filter((v): v is string => typeof v === 'string')
				: (existing?.permissions ?? []);

			users.set(authUserId, {
				authUserId,
				appUserId,
				email: typeof data['email'] === 'string' ? data['email'] : (existing?.email ?? null),
				displayName:
					typeof data['display_name'] === 'string'
						? data['display_name']
						: (existing?.displayName ?? null),
				roles,
				permissions,
				status,
				authVersion: incomingVersion,
				updatedAt: event.occurred_at
			});
		},
		upsertFromBackendItem(item: BackendUserSyncItem): ElysiarLocalUser {
			const existing = users.get(item.sub);
			const status: ElysiarLocalUser['status'] =
				item.status === 'deleted'
					? 'deleted'
					: item.status === 'suspended'
						? 'suspended'
						: 'active';
			return upsertUser({
				authUserId: item.sub,
				appUserId: item.app_user_id,
				email: item.email,
				displayName: item.display_name,
				roles: [...item.roles],
				permissions: existing ? [...existing.permissions] : [],
				status,
				authVersion: item.version,
				updatedAt: item.updated_at
			});
		}
	};
}

export const elysiarSyncStore = createElysiarSyncStore();
