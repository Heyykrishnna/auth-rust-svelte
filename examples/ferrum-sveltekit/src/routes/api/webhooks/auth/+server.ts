import { createWebhookEndpointHandler } from '@secure-auth/svelte/server';
import { ferrumSyncStore } from '../../../../lib/db/sync.js';

const activeSecrets = [
	process.env['AUTH_WEBHOOK_SECRET'] ?? '',
	process.env['AUTH_WEBHOOK_SECRET_PREVIOUS'] ?? ''
].filter((value) => value.length > 0);

export const POST = createWebhookEndpointHandler({
	secret: activeSecrets,
	isEventProcessed: (eventId) => ferrumSyncStore.isEventProcessed(eventId),
	markEventProcessed: (eventId) => {
		ferrumSyncStore.markEventProcessed(eventId);
	},
	handlers: {
		'user.created': (event) => ferrumSyncStore.applyWebhookEvent(event),
		'user.updated': (event) => ferrumSyncStore.applyWebhookEvent(event),
		'user.deleted': (event) => ferrumSyncStore.applyWebhookEvent(event),
		'user.suspended': (event) => ferrumSyncStore.applyWebhookEvent(event),
		'membership.updated': (event) => ferrumSyncStore.applyWebhookEvent(event),
		'role.assigned': (event) => ferrumSyncStore.applyWebhookEvent(event),
		'role.revoked': (event) => ferrumSyncStore.applyWebhookEvent(event)
	}
});
