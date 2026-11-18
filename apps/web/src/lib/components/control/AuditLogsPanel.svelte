<script lang="ts">
	import SectionHeader from './SectionHeader.svelte';
	import Dropdown from '$lib/components/Dropdown.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import type { AuditLogEntryDto, EnvironmentKind } from '$lib/api/types';
	import { exportAuditLogsToCsv, formatRelativeTimestamp } from '$lib/control/permissions';

	interface Props {
		appId: string;
		envKind: EnvironmentKind;
	}

	let { appId, envKind }: Props = $props();

	let logs = $state<AuditLogEntryDto[]>([]);
	let total = $state(0);
	let limit = $state(30);
	let offset = $state(0);
	let loading = $state(false);

	let queryText = $state('');
	let actorType = $state('all');
	let resourceType = $state('');
	let fromDate = $state('');
	let toDate = $state('');
	let expandedId = $state<string | null>(null);

	async function loadAuditLogs() {
		loading = true;
		try {
			const res = await controlApi.listAuditLogs(appId, envKind, {
				q: queryText.trim() || undefined,
				actor_type: actorType !== 'all' ? actorType : undefined,
				resource_type: resourceType.trim() || undefined,
				from: fromDate ? new Date(fromDate).toISOString() : undefined,
				to: toDate ? new Date(toDate).toISOString() : undefined,
				limit,
				offset
			});
			logs = res.logs;
			total = res.total;
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load audit logs', 'error');
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const _app = appId;
		const _env = envKind;
		const _off = offset;
		const _actor = actorType;
		void _app;
		void _env;
		void _off;
		void _actor;
		loadAuditLogs();
	});

	function triggerDownload(content: string, filename: string, mime: string) {
		const blob = new Blob([content], { type: mime });
		const url = URL.createObjectURL(blob);
		const anchor = document.createElement('a');
		anchor.href = url;
		anchor.download = filename;
		anchor.click();
		URL.revokeObjectURL(url);
	}

	function handleExportCsv() {
		triggerDownload(
			exportAuditLogsToCsv(logs),
			`audit-logs-${envKind}.csv`,
			'text/csv;charset=utf-8'
		);
		addToast('Exported audit logs as CSV', 'success');
	}

	function handleExportJson() {
		triggerDownload(
			JSON.stringify(logs, null, 2),
			`audit-logs-${envKind}.json`,
			'application/json'
		);
		addToast('Exported audit logs as JSON', 'success');
	}
</script>

<section class="grid min-w-0 grid-cols-1 gap-5 sm:gap-6" aria-label="Audit log explorer">
	<SectionHeader title="Audit Logs" description="Immutable environment-scoped trail of authentication, credential, webhook, and role mutations.">
		<div class="export-group">
			<Button variant="outline" size="sm" disabled={logs.length === 0} onclick={handleExportCsv}>Export CSV</Button>
			<Button variant="outline" size="sm" disabled={logs.length === 0} onclick={handleExportJson}>Export JSON</Button>
		</div>
	</SectionHeader>

	<form
		class="flex min-w-0 flex-wrap items-center gap-2 [&>button]:h-9 [&>input]:h-9 [&>input]:max-w-full"
		onsubmit={(e) => {
			e.preventDefault();
			offset = 0;
			loadAuditLogs();
		}}
	>
		<input class="control-input" type="search" bind:value={queryText} placeholder="Search event, actor ID, resource, metadata..." aria-label="Search audit logs" />
		<Dropdown
			label="Actor type"
			bind:value={actorType}
			options={[
				{ value: 'all', label: 'All actors' },
				{ value: 'developer', label: 'Developer' },
				{ value: 'user', label: 'User' },
				{ value: 'system', label: 'System' }
			]}
			/>
		<input class="control-input" type="text" bind:value={resourceType} placeholder="Resource type" aria-label="Resource type" />
		<input class="control-input" type="datetime-local" bind:value={fromDate} aria-label="From date" />
		<input class="control-input" type="datetime-local" bind:value={toDate} aria-label="To date" />
		<Button type="submit" variant="primary" size="sm">Apply</Button>
	</form>

	{#if loading && logs.length === 0}
		<div class="empty-surface">Loading audit events...</div>
	{:else if logs.length === 0}
		<div class="empty-surface">No audit events match the active filters.</div>
	{:else}
		<div class="table-wrap">
			<table class="data-table">
				<thead>
					<tr>
						<th>Event</th>
						<th>Actor</th>
						<th>Resource</th>
						<th>IP</th>
						<th>Time</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each logs as log}
						<tr>
							<td class="mono strong">{log.event}</td>
							<td><Badge variant="neutral" size="sm">{log.actor_type}</Badge></td>
							<td class="mono">{log.resource_type ?? '—'}</td>
							<td class="mono muted">{log.ip_address ?? '—'}</td>
							<td class="muted">{formatRelativeTimestamp(log.created_at)}</td>
							<td class="row-actions">
								<button
									type="button"
									class="mini-btn"
									onclick={() => (expandedId = expandedId === log.id ? null : log.id)}
								>
									{expandedId === log.id ? 'Hide' : 'Metadata'}
								</button>
							</td>
						</tr>
						{#if expandedId === log.id}
							<tr class="meta-row">
								<td colspan="6">
									<pre>{JSON.stringify({ id: log.id, actor_id: log.actor_id, resource_id: log.resource_id, created_at: log.created_at, metadata: log.metadata }, null, 2)}</pre>
								</td>
							</tr>
						{/if}
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	<div class="pagination-row">
		<span class="muted">Showing {total === 0 ? 0 : offset + 1}–{Math.min(offset + limit, total)} of {total}</span>
		<div class="page-btns">
			<Button variant="outline" size="sm" disabled={offset === 0} onclick={() => (offset = Math.max(0, offset - limit))}>Previous</Button>
			<Button variant="outline" size="sm" disabled={offset + limit >= total} onclick={() => (offset = offset + limit)}>Next</Button>
		</div>
	</div>
</section>

<style>
	.export-group { display: flex; gap: 0.5rem; }
	.control-input { background-color: var(--color-brand-surface-card); border: 1px solid var(--color-brand-border); border-radius: 6px; padding: 0.48rem 0.7rem; color: var(--color-brand-text-primary); font-size: 0.8rem; outline: none; }
	.control-input[type='search'] { flex: 1; min-width: 220px; }
	.table-wrap { background-color: var(--color-brand-surface-card); border: 1px solid var(--color-brand-border); border-radius: 12px; overflow-x: auto; }
	.data-table { width: 100%; border-collapse: collapse; text-align: left; }
	.data-table th { font-size: 0.72rem; color: var(--color-brand-text-secondary); padding: 0.75rem 1rem; border-bottom: 1px solid var(--color-brand-border); }
	.data-table td { padding: 0.8rem 1rem; border-bottom: 1px solid var(--color-brand-border); font-size: 0.82rem; color: #d4d4d4; }
	.mono { font-family: var(--font-mono); font-size: 0.76rem; }
	.strong { color: #ffffff; }
	.muted { color: var(--color-brand-text-secondary); font-size: 0.78rem; }
	.row-actions { text-align: right; }
	.mini-btn { background: transparent; border: 1px solid var(--color-brand-border-hover); color: #d4d4d4; border-radius: 4px; padding: 0.22rem 0.55rem; font-size: 0.72rem; cursor: pointer; }
	.meta-row td { background-color: var(--color-brand-surface); }
	.meta-row pre { margin: 0; font-size: 0.74rem; color: var(--color-brand-text-secondary); white-space: pre-wrap; }
	.empty-surface { padding: 2.5rem; text-align: center; color: var(--color-brand-text-secondary); background-color: var(--color-brand-surface-card); border: 1px solid var(--color-brand-border); border-radius: 12px; font-size: 0.85rem; }
	.pagination-row { display: flex; align-items: center; justify-content: space-between; }
	.page-btns { display: flex; gap: 0.5rem; }
</style>

