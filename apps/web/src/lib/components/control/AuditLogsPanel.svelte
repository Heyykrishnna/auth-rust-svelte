<script lang="ts">
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

<section class="panel-stack" aria-label="Audit log explorer">
	<div class="section-header">
		<div>
			<h1 class="page-title">Audit Logs</h1>
			<p class="page-subtitle">Immutable environment-scoped trail of authentication, credential, webhook, and role mutations.</p>
		</div>
		<div class="export-group">
			<Button variant="outline" size="sm" disabled={logs.length === 0} onclick={handleExportCsv}>Export CSV</Button>
			<Button variant="outline" size="sm" disabled={logs.length === 0} onclick={handleExportJson}>Export JSON</Button>
		</div>
	</div>

	<form
		class="filter-bar"
		onsubmit={(e) => {
			e.preventDefault();
			offset = 0;
			loadAuditLogs();
		}}
	>
		<input type="search" bind:value={queryText} placeholder="Search event, actor ID, resource, metadata..." aria-label="Search audit logs" />
		<select bind:value={actorType} aria-label="Actor type">
			<option value="all">All actors</option>
			<option value="developer">Developer</option>
			<option value="user">User</option>
			<option value="system">System</option>
		</select>
		<input type="text" bind:value={resourceType} placeholder="Resource type" aria-label="Resource type" />
		<input type="datetime-local" bind:value={fromDate} aria-label="From date" />
		<input type="datetime-local" bind:value={toDate} aria-label="To date" />
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
	.panel-stack { display: flex; flex-direction: column; gap: 1.25rem; }
	.section-header { display: flex; align-items: flex-end; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
	.page-title { font-size: 1.75rem; font-weight: 600; color: #ffffff; margin: 0; }
	.page-subtitle { font-size: 0.85rem; color: #71717a; margin: 0.25rem 0 0; }
	.export-group { display: flex; gap: 0.5rem; }
	.filter-bar { display: flex; gap: 0.55rem; flex-wrap: wrap; align-items: center; }
	.filter-bar input, .filter-bar select { background-color: #15171b; border: 1px solid #23262e; border-radius: 6px; padding: 0.48rem 0.7rem; color: #f4f4f5; font-size: 0.8rem; outline: none; }
	.filter-bar input[type='search'] { flex: 1; min-width: 220px; }
	.table-wrap { background-color: #15171b; border: 1px solid #23262e; border-radius: 12px; overflow-x: auto; }
	.data-table { width: 100%; border-collapse: collapse; text-align: left; }
	.data-table th { font-size: 0.72rem; color: #71717a; padding: 0.75rem 1rem; border-bottom: 1px solid #23262e; }
	.data-table td { padding: 0.8rem 1rem; border-bottom: 1px solid #1f2228; font-size: 0.82rem; color: #d4d4d8; }
	.mono { font-family: var(--font-mono); font-size: 0.76rem; }
	.strong { color: #ffffff; }
	.muted { color: #71717a; font-size: 0.78rem; }
	.row-actions { text-align: right; }
	.mini-btn { background: transparent; border: 1px solid #2e333d; color: #d4d4d8; border-radius: 4px; padding: 0.22rem 0.55rem; font-size: 0.72rem; cursor: pointer; }
	.meta-row td { background-color: #0b0c0e; }
	.meta-row pre { margin: 0; font-size: 0.74rem; color: #a1a1aa; white-space: pre-wrap; }
	.empty-surface { padding: 2.5rem; text-align: center; color: #71717a; background-color: #15171b; border: 1px solid #23262e; border-radius: 12px; font-size: 0.85rem; }
	.pagination-row { display: flex; align-items: center; justify-content: space-between; }
	.page-btns { display: flex; gap: 0.5rem; }
</style>

