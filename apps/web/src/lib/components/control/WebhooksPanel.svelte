<script lang="ts">
	import SectionHeader from './SectionHeader.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import SelectionRow from '$lib/components/SelectionRow.svelte';
	import Button from '$lib/components/Button.svelte';
	import DeleteButton from '$lib/components/DeleteButton.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import { showOneTimeSecret } from '$lib/stores/control.svelte';
	import type {
		EnvironmentKind,
		WebhookDeliveryDto,
		WebhookEndpointViewDto
	} from '$lib/api/types';
	import { formatRelativeTimestamp } from '$lib/control/permissions';

	interface Props {
		appId: string;
		envKind: EnvironmentKind;
		canMutate: boolean;
	}

	let { appId, envKind, canMutate }: Props = $props();

	const ALL_EVENTS = [
		'user.created',
		'user.updated',
		'user.deleted',
		'user.suspended',
		'user.email_verified',
		'membership.created',
		'membership.updated',
		'membership.revoked',
		'session.created',
		'session.revoked',
		'role.assigned',
		'role.revoked',
		'webhook.test'
	];

	let endpoints = $state<WebhookEndpointViewDto[]>([]);
	let loading = $state(false);
	let selectedEndpointId = $state<string | null>(null);
	let deliveries = $state<WebhookDeliveryDto[]>([]);
	let deliveriesLoading = $state(false);
	let deliveryFilter = $state<'all' | 'succeeded' | 'retrying' | 'failed' | 'dead_letter'>('all');
	let expandedDeliveryId = $state<string | null>(null);

	let newUrl = $state('');
	let newDescription = $state('');
	let selectedEvents = $state<string[]>(['user.created', 'user.updated', 'session.revoked']);
	let creating = $state(false);

	let replayFrom = $state('');
	let replayTo = $state('');
	let replaying = $state(false);

	const filteredDeliveries = $derived(
		deliveryFilter === 'all'
			? deliveries
			: deliveries.filter((d) => d.status === deliveryFilter)
	);

	async function loadEndpoints() {
		loading = true;
		try {
			const list = await controlApi.listWebhooks(appId, envKind);
			endpoints = list;
			if (!selectedEndpointId || !list.some((e) => e.id === selectedEndpointId)) {
				selectedEndpointId = list[0]?.id ?? null;
			}
			if (selectedEndpointId) {
				await loadDeliveries(selectedEndpointId);
			} else {
				deliveries = [];
			}
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load webhooks', 'error');
		} finally {
			loading = false;
		}
	}

	async function loadDeliveries(endpointId: string) {
		deliveriesLoading = true;
		try {
			deliveries = await controlApi.listWebhookDeliveries(appId, envKind, endpointId, 50);
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load deliveries', 'error');
		} finally {
			deliveriesLoading = false;
		}
	}

	$effect(() => {
		const _app = appId;
		const _env = envKind;
		void _app;
		void _env;
		loadEndpoints();
	});

	function toggleSubscribedEvent(evt: string) {
		if (selectedEvents.includes(evt)) {
			selectedEvents = selectedEvents.filter((e) => e !== evt);
		} else {
			selectedEvents = [...selectedEvents, evt];
		}
	}

	async function handleCreateEndpoint(e: SubmitEvent) {
		e.preventDefault();
		if (!canMutate) return;
		creating = true;
		try {
			const created = await controlApi.createWebhook(appId, envKind, {
				url: newUrl.trim(),
				description: newDescription.trim() || undefined,
				subscribed_events: selectedEvents
			});
			newUrl = '';
			newDescription = '';
			showOneTimeSecret({
				title: 'Webhook Signing Secret Created',
				secretKey: created.signing_secret
			});
			addToast('Webhook endpoint registered', 'success');
			await loadEndpoints();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to register webhook', 'error');
		} finally {
			creating = false;
		}
	}

	async function handleTogglePause(ep: WebhookEndpointViewDto) {
		if (!canMutate) return;
		const nextStatus = ep.status === 'active' ? 'paused' : 'active';
		try {
			await controlApi.updateWebhook(appId, envKind, ep.id, { status: nextStatus });
			addToast(`Endpoint ${nextStatus}`, 'info');
			await loadEndpoints();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to update endpoint', 'error');
		}
	}

	async function handleRotateSecret(ep: WebhookEndpointViewDto) {
		if (!canMutate) return;
		try {
			const rotated = await controlApi.rotateWebhookSecret(appId, envKind, ep.id, 24);
			showOneTimeSecret({
				title: 'Rotated Webhook Signing Secret (24h Grace Window)',
				secretKey: rotated.signing_secret
			});
			addToast('Signing secret rotated with 24h dual-secret window', 'success');
			await loadEndpoints();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to rotate secret', 'error');
		}
	}

	async function handleTestPing(ep: WebhookEndpointViewDto) {
		if (!canMutate) return;
		try {
			await controlApi.sendTestWebhook(appId, envKind, ep.id);
			addToast('Queued webhook.test delivery', 'success');
			await loadDeliveries(ep.id);
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to queue test event', 'error');
		}
	}

	async function handleDelete(ep: WebhookEndpointViewDto) {
		if (!canMutate) return;
		try {
			await controlApi.deleteWebhook(appId, envKind, ep.id);
			addToast('Webhook endpoint deleted', 'info');
			await loadEndpoints();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to delete endpoint', 'error');
		}
	}

	async function handleReplayOne(delivery: WebhookDeliveryDto) {
		if (!canMutate || !selectedEndpointId) return;
		try {
			await controlApi.replayWebhookDelivery(appId, envKind, selectedEndpointId, delivery.id);
			addToast('Delivery queued for replay', 'success');
			await loadDeliveries(selectedEndpointId);
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Replay failed', 'error');
		}
	}

	async function handleReplayRange(e: SubmitEvent) {
		e.preventDefault();
		if (!canMutate || !selectedEndpointId || !replayFrom || !replayTo) return;
		replaying = true;
		try {
			const res = await controlApi.replayWebhookRange(appId, envKind, selectedEndpointId, {
				from: new Date(replayFrom).toISOString(),
				to: new Date(replayTo).toISOString()
			});
			addToast(`Replayed ${res.replayed_count} event(s)`, 'success');
			await loadDeliveries(selectedEndpointId);
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Range replay failed', 'error');
		} finally {
			replaying = false;
		}
	}
</script>

<section class="grid min-w-0 grid-cols-1 gap-5 sm:gap-6" aria-label="Webhooks and sync operations">
	<SectionHeader title="Webhooks &amp; Sync" description="Manage signed webhook endpoints, rotate HMAC secrets, inspect delivery attempts, and replay dead-letter queues." />

	<div class="surface-box task-surface">
		<h2 class="box-title">Register Webhook Endpoint</h2>
		<form class="create-form" onsubmit={handleCreateEndpoint}>
			<div class="grid-2">
				<label class="field">
					<span>Endpoint URL</span>
					<input type="url" bind:value={newUrl} placeholder="https://app.example.com/api/webhooks/auth" required disabled={!canMutate} />
				</label>
				<label class="field">
					<span>Description</span>
					<input type="text" bind:value={newDescription} placeholder="Production DB sync consumer" disabled={!canMutate} />
				</label>
			</div>

			<div class="events-selector">
				{#each ALL_EVENTS as evt}
					<SelectionRow label={evt} checked={selectedEvents.includes(evt)} disabled={!canMutate} onchange={() => toggleSubscribedEvent(evt)} />
				{/each}
			</div>

			<div class="form-actions">
				<Button type="submit" size="sm" loading={creating} disabled={!canMutate || creating}>
					Add Endpoint
				</Button>
			</div>
		</form>
	</div>

	<div class="surface-box task-surface">
		<h2 class="box-title">Configured Endpoints ({endpoints.length})</h2>
		{#if loading && endpoints.length === 0}
			<p class="muted">Loading webhook endpoints...</p>
		{:else if endpoints.length === 0}
			<p class="muted">No webhook endpoints registered in this environment.</p>
		{:else}
			<div class="endpoint-list">
				{#each endpoints as ep}
					<div class="endpoint-card task-surface" class:selected={selectedEndpointId === ep.id}>
						<div class="ep-top">
							<button
								type="button"
								class="ep-select"
								onclick={() => {
									selectedEndpointId = ep.id;
									loadDeliveries(ep.id);
								}}
							>
								<code>{ep.url}</code>
								<span>{ep.description || 'No description'}</span>
							</button>
							<div class="ep-badges">
								<Badge variant={ep.status === 'active' ? 'success' : ep.status === 'paused' ? 'warning' : 'danger'} size="sm" dot>
									{ep.status}
								</Badge>
								{#if ep.previous_secret_expires_at}
									<Badge variant="warning" size="sm">Dual-secret grace active</Badge>
								{/if}
								{#if ep.consecutive_failures > 0}
									<Badge variant="danger" size="sm">{ep.consecutive_failures} failures</Badge>
								{/if}
							</div>
						</div>

						<div class="ep-events">
							{#each ep.subscribed_events as evt}
								<span class="mini-tag">{evt}</span>
							{/each}
						</div>

						<div class="ep-actions">
							<Button size="sm" disabled={!canMutate} onclick={() => handleTestPing(ep)}>Send Test Event</Button>
							<Button size="sm" disabled={!canMutate} onclick={() => handleRotateSecret(ep)}>Rotate Secret</Button>
							<Button size="sm" disabled={!canMutate} onclick={() => handleTogglePause(ep)}>
								{ep.status === 'active' ? 'Pause' : 'Resume'}
							</Button>
							<DeleteButton label="Delete webhook" disabled={!canMutate} onclick={() => handleDelete(ep)} />
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	{#if selectedEndpointId}
		<div class="surface-box task-surface">
			<div class="box-header">
				<div>
					<h2 class="box-title">Delivery Log &amp; Recovery</h2>
					<p class="box-sub">Inspect delivery status codes, latencies, and replay failed or dead-letter events.</p>
				</div>

				<div class="filter-pills">
					{#each ['all', 'succeeded', 'retrying', 'failed', 'dead_letter'] as st}
						<button
							type="button"
							class:active={deliveryFilter === st}
							onclick={() => (deliveryFilter = st as typeof deliveryFilter)}
						>
							{st}
						</button>
					{/each}
				</div>
			</div>

			<form class="replay-bar" onsubmit={handleReplayRange}>
				<label class="field">
					<span>Bulk Replay From</span>
					<input type="datetime-local" bind:value={replayFrom} disabled={!canMutate} required />
				</label>
				<label class="field">
					<span>Bulk Replay To</span>
					<input type="datetime-local" bind:value={replayTo} disabled={!canMutate} required />
				</label>
				<Button type="submit" size="sm" loading={replaying} disabled={!canMutate || replaying}>
					Replay Time Window
				</Button>
			</form>

			{#if deliveriesLoading}
				<p class="muted">Loading deliveries...</p>
			{:else if filteredDeliveries.length === 0}
				<p class="muted">No deliveries match the current filter.</p>
			{:else}
				<div class="table-wrap">
					<table class="data-table">
						<thead>
							<tr>
								<th>Status</th>
								<th>HTTP</th>
								<th>Attempt</th>
								<th>Latency</th>
								<th>Event ID</th>
								<th>Time</th>
								<th></th>
							</tr>
						</thead>
						<tbody>
							{#each filteredDeliveries as d}
								<tr>
									<td>
										<Badge
											variant={d.status === 'succeeded' ? 'success' : d.status === 'dead_letter' || d.status === 'failed' ? 'danger' : 'warning'}
											size="sm"
											dot
										>
											{d.status}
										</Badge>
									</td>
									<td class="mono">{d.response_status ?? '—'}</td>
									<td class="mono">#{d.attempt_number}</td>
									<td class="mono">{d.latency_ms !== null ? `${d.latency_ms}ms` : '—'}</td>
									<td class="mono">{d.outbox_event_id.slice(0, 8)}...</td>
									<td class="muted">{formatRelativeTimestamp(d.created_at)}</td>
									<td class="row-actions">
										<button
											type="button"
											class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold"
											onclick={() => (expandedDeliveryId = expandedDeliveryId === d.id ? null : d.id)}
										>
											{expandedDeliveryId === d.id ? 'Hide' : 'Inspect'}
										</button>
										<button
											type="button"
											class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold"
											disabled={!canMutate}
											onclick={() => handleReplayOne(d)}
										>
											Replay
										</button>
									</td>
								</tr>
								{#if expandedDeliveryId === d.id}
									<tr class="inspect-row">
										<td colspan="7">
											<div class="inspect-grid">
												<div>
													<span class="inspect-label">Request Headers</span>
													<pre>{JSON.stringify(d.request_headers ?? {}, null, 2)}</pre>
												</div>
												<div>
													<span class="inspect-label">Response Body Excerpt</span>
													<pre>{d.response_body_excerpt || '(empty response)'}</pre>
												</div>
											</div>
										</td>
									</tr>
								{/if}
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</div>
	{/if}
</section>

<style>
	.surface-box { padding: 1.35rem; display: flex; flex-direction: column; gap: 1rem; }
	.box-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
	.box-title { font-size: 0.98rem; font-weight: 600; color: var(--color-brand-text-primary); margin: 0; }
	.box-sub { font-size: 0.78rem; color: var(--color-brand-text-secondary); margin: 0.2rem 0 0; }
	.create-form { display: flex; flex-direction: column; gap: 0.9rem; }
	.grid-2 { display: grid; grid-template-columns: 1.4fr 1fr; gap: 0.85rem; }
	.field { display: flex; flex-direction: column; gap: 0.3rem; }
	.field span { font-size: 0.74rem; color: var(--color-brand-text-secondary); }
	.field input { background-color: var(--color-brand-surface); border: 1px solid var(--color-brand-border); border-radius: 6px; padding: 0.55rem 0.75rem; color: var(--color-brand-text-primary); font-size: 0.84rem; outline: none; }
	.events-selector { display: flex; gap: 0.4rem; flex-wrap: wrap; }
	.form-actions { display: flex; justify-content: flex-end; }
	.endpoint-list { display: flex; flex-direction: column; gap: 0.75rem; }
	.endpoint-card { padding: 1rem; display: flex; flex-direction: column; gap: 0.75rem; }
	.endpoint-card.selected { border-color: #ffffff; }
	.ep-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
	.ep-select { background: transparent; border: none; text-align: left; cursor: pointer; display: flex; flex-direction: column; gap: 0.2rem; padding: 0; }
	.ep-select code { color: #ffffff; font-size: 0.84rem; }
	.ep-select span { color: var(--color-brand-text-secondary); font-size: 0.76rem; }
	.ep-badges { display: flex; gap: 0.4rem; flex-wrap: wrap; }
	.ep-events { display: flex; gap: 0.35rem; flex-wrap: wrap; }
	.mini-tag { font-family: var(--font-mono); font-size: 0.68rem; color: var(--color-brand-text-secondary); background-color: var(--color-brand-surface-card); border: 1px solid var(--color-brand-border); padding: 0.15rem 0.45rem; border-radius: 4px; }
	.ep-actions { display: flex; gap: 0.5rem; flex-wrap: wrap; }
	.filter-pills { display: inline-flex; gap: 0.25rem; background-color: var(--color-brand-surface); border: 1px solid var(--color-brand-border); border-radius: 6px; padding: 2px; }
	.filter-pills button { background: transparent; border: none; color: var(--color-brand-text-secondary); font-size: 0.74rem; padding: 0.28rem 0.55rem; border-radius: 4px; cursor: pointer; text-transform: capitalize; }
	.filter-pills button.active { background-color: var(--color-brand-border); color: #ffffff; }
	.replay-bar { display: flex; align-items: flex-end; gap: 0.75rem; flex-wrap: wrap; padding: 0.85rem; background-color: var(--color-brand-surface); border: 1px solid var(--color-brand-border); border-radius: 8px; }
	.table-wrap { overflow-x: auto; border: 1px solid var(--color-brand-border); border-radius: 8px; }
	.data-table { width: 100%; border-collapse: collapse; text-align: left; }
	.data-table th { font-size: 0.72rem; color: var(--color-brand-text-secondary); padding: 0.7rem 0.9rem; border-bottom: 1px solid var(--color-brand-border); background-color: var(--color-brand-surface); }
	.data-table td { padding: 0.75rem 0.9rem; border-bottom: 1px solid var(--color-brand-border); font-size: 0.82rem; color: #d4d4d4; }
	.mono { font-family: var(--font-mono); font-size: 0.76rem; }
	.muted { color: var(--color-brand-text-secondary); font-size: 0.8rem; }
	.row-actions { display: flex; gap: 0.4rem; justify-content: flex-end; }


	.inspect-row td { background-color: var(--color-brand-surface); }
	.inspect-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
	.inspect-label { display: block; font-size: 0.7rem; color: var(--color-brand-text-secondary); margin-bottom: 0.35rem; text-transform: uppercase; }
	.inspect-grid pre { margin: 0; font-size: 0.74rem; color: #d4d4d4; white-space: pre-wrap; word-break: break-word; max-height: 180px; overflow-y: auto; }
	@media (max-width: 900px) {
		.grid-2, .inspect-grid { grid-template-columns: 1fr; }
	}
</style>

