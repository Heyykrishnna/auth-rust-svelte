import { createWebhookRouteHandler } from '@secure-auth/next/server';
import { elysiarSyncStore } from '../../../../lib/db/sync.js';

const activeSecrets = [
	process.env['AUTH_WEBHOOK_SECRET'] ?? '',
	process.env['AUTH_WEBHOOK_SECRET_PREVIOUS'] ?? ''
].filter((value) => value.length > 0);

export const POST = createWebhookRouteHandler({
	secret: activeSecrets,
	isEventProcessed: (eventId) => elysiarSyncStore.isEventProcessed(eventId),
	markEventProcessed: (eventId) => {
		elysiarSyncStore.markEventProcessed(eventId);
	},
	handlers: {
		'user.created': (event) => elysiarSyncStore.applyWebhookEvent(event),
		'user.updated': (event) => elysiarSyncStore.applyWebhookEvent(event),
		'user.deleted': (event) => elysiarSyncStore.applyWebhookEvent(event),
		'user.suspended': (event) => elysiarSyncStore.applyWebhookEvent(event),
		'membership.updated': (event) => elysiarSyncStore.applyWebhookEvent(event),
		'role.assigned': (event) => elysiarSyncStore.applyWebhookEvent(event),
		'role.revoked': (event) => elysiarSyncStore.applyWebhookEvent(event)
	}
});
