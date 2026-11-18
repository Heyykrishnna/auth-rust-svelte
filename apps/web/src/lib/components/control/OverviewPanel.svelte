<script lang="ts">
	import SectionHeader from './SectionHeader.svelte';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import type { EnvironmentKind, EnvironmentOverviewDto } from '$lib/api/types';
	import { computeSuccessRate, formatRelativeTimestamp } from '$lib/control/permissions';

	interface Props {
		overview: EnvironmentOverviewDto | null;
		loading: boolean;
		appName: string;
		appSlug: string;
		environmentKind: EnvironmentKind;
		onRefresh: () => void;
	}

	let { overview, loading, appName, appSlug, environmentKind, onRefresh }: Props = $props();
	let snippetTab = $state<'sveltekit' | 'nextjs' | 'oidc'>('sveltekit');

	const webhookRate24h = $derived(
		overview && overview.webhook_deliveries_succeeded_24h + overview.webhook_deliveries_failed_24h > 0
			? `${computeSuccessRate(overview.webhook_deliveries_succeeded_24h, overview.webhook_deliveries_failed_24h)}%`
			: '—'
	);

	const snippetText = $derived.by(() => {
		const issuer = overview?.issuer_url ?? 'http://localhost:8080';
		const pk = overview?.publishable_key ?? 'REPLACE_WITH_PUBLISHABLE_KEY';
		if (snippetTab === 'sveltekit') {
			return `import { createAuthHandle } from '@secure-auth/svelte/server';\n\nexport const handle = createAuthHandle({\n  issuerUrl: '${issuer}',\n  audience: '${appSlug}',\n  publishableKey: '${pk}',\n  protectedRoutes: ['/dashboard']\n});`;
		}
		if (snippetTab === 'nextjs') {
			return `import { authMiddleware } from '@secure-auth/next/middleware';\n\nexport const middleware = authMiddleware({\n  issuerUrl: '${issuer}',\n  audience: '${appSlug}',\n  publishableKey: '${pk}',\n  protectedRoutes: ['/dashboard']\n});`;
		}
		return `ISSUER_URL=${issuer}\nJWKS_URI=${overview?.jwks_uri ?? `${issuer}/.well-known/jwks.json`}\nAUDIENCE=${appSlug}\nPUBLISHABLE_KEY=${pk}`;
	});

	async function copyValue(value: string, label: string) {
		try {
			await navigator.clipboard.writeText(value);
			addToast(`${label} copied`, 'success');
		} catch {
			addToast('Copy failed', 'warning');
		}
	}
</script>

<section class="grid min-w-0 grid-cols-1 gap-5 sm:gap-6" aria-label="Environment overview">
	<SectionHeader title="Overview" description={`${appName} · ${environmentKind === 'production' ? 'Production' : 'Development'}`}>
		<Button variant="outline" size="sm" onclick={onRefresh} disabled={loading}>
			{loading ? 'Refreshing...' : 'Refresh'}
		</Button>
	</SectionHeader>

	{#if loading && !overview}
		<div class="empty-surface">Loading environment telemetry...</div>
	{:else if !overview}
		<div class="empty-surface">Select an application and environment to view overview metrics.</div>
	{:else}
		<div class="grid min-w-0 gap-3 sm:grid-cols-2 xl:grid-cols-[1.2fr_1fr_1fr]">
			<article class="flex min-w-0 flex-col justify-between gap-5 rounded-xl bg-brand-lime-soft p-5 text-brand-lime-ink sm:col-span-2 xl:col-span-1">
				<h2 class="m-0 text-sm font-semibold text-brand-lime-ink">Users in this environment</h2>
				<div>
					<p class="m-0 font-heading text-5xl font-bold leading-none tracking-tight text-brand-lime-ink tabular-nums">{overview.total_users.toLocaleString()}</p>
					<p class="mt-3 text-sm text-brand-lime-ink">{overview.active_users} active · {overview.suspended_users} suspended · {overview.unverified_users} unverified</p>
				</div>
				<p class="m-0 border-t border-brand-lime-ink/15 pt-3 text-sm font-medium text-brand-lime-ink">{overview.active_sessions.toLocaleString()} active sessions</p>
			</article>

			<article class="grid min-w-0 content-start gap-3 rounded-xl border border-brand-border bg-brand-surface-card p-5">
				<h2 class="m-0 text-sm font-semibold text-brand-text-primary">Authentication</h2>
				<dl class="grid gap-2.5 border-t border-brand-border pt-3 text-sm">
					{#each [
						['Successful sign-ins · 24h', overview.auth_success_24h],
						['Failures and lockouts · 24h', overview.auth_failure_24h],
						['Successful sign-ins · 7d', overview.auth_success_7d],
						['New users · 7d', overview.new_users_7d]
					] as [label, value] (label)}
						<div class="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3">
							<dt class="text-brand-text-secondary">{label}</dt>
							<dd class="text-right font-medium text-brand-text-primary tabular-nums">{value.toLocaleString()}</dd>
						</div>
					{/each}
				</dl>
			</article>

			<article class="grid min-w-0 content-start gap-3 rounded-xl border border-brand-border bg-brand-surface-card p-5">
				<h2 class="m-0 text-sm font-semibold text-brand-text-primary">Webhooks &amp; API</h2>
				<dl class="grid gap-2.5 border-t border-brand-border pt-3 text-sm">
					{#each [
						['Webhook delivery rate · 24h', webhookRate24h],
						['Active webhook endpoints', overview.webhook_endpoints_active],
						['Failed deliveries · 24h', overview.webhook_deliveries_failed_24h],
						['Dead-letter deliveries', overview.webhook_dead_letters],
						['Active API credentials', overview.active_api_keys]
					] as [label, value] (label)}
						<div class="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3">
							<dt class="text-brand-text-secondary">{label}</dt>
							<dd class="text-right font-medium text-brand-text-primary tabular-nums">{value.toLocaleString()}</dd>
						</div>
					{/each}
				</dl>
			</article>
		</div>

		<div class="grid min-w-0 grid-cols-1 items-start gap-3 xl:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
			<div class="surface-box min-w-0">
				<div class="box-header">
					<div>
						<h2 class="box-title">Quick Integration</h2>
						<p class="box-sub">Environment endpoints and SDK bootstrap snippets</p>
					</div>
					<div class="seg-tabs" role="tablist" aria-label="SDK snippet language">
						<button type="button" role="tab" aria-selected={snippetTab === 'sveltekit'} class:active={snippetTab === 'sveltekit'} onclick={() => (snippetTab = 'sveltekit')}>SvelteKit</button>
						<button type="button" role="tab" aria-selected={snippetTab === 'nextjs'} class:active={snippetTab === 'nextjs'} onclick={() => (snippetTab = 'nextjs')}>Next.js</button>
						<button type="button" role="tab" aria-selected={snippetTab === 'oidc'} class:active={snippetTab === 'oidc'} onclick={() => (snippetTab = 'oidc')}>OIDC</button>
					</div>
				</div>

				<div class="kv-list">
					<div class="kv-row">
						<span class="kv-label">Issuer URL</span>
						<code class="kv-code">{overview.issuer_url}</code>
						<button type="button" class="copy-btn" onclick={() => copyValue(overview.issuer_url, 'Issuer URL')}>Copy</button>
					</div>
					<div class="kv-row">
						<span class="kv-label">JWKS URI</span>
						<code class="kv-code">{overview.jwks_uri}</code>
						<button type="button" class="copy-btn" onclick={() => copyValue(overview.jwks_uri, 'JWKS URI')}>Copy</button>
					</div>
					<div class="kv-row">
						<span class="kv-label">Publishable Key</span>
						<code class="kv-code">{overview.publishable_key ?? 'None active'}</code>
						{#if overview.publishable_key}
							<button type="button" class="copy-btn" onclick={() => copyValue(overview.publishable_key ?? '', 'Publishable key')}>Copy</button>
						{/if}
					</div>
				</div>

				<div class="code-panel">
					<pre><code>{snippetText}</code></pre>
					<button type="button" class="copy-snippet" onclick={() => copyValue(snippetText, 'Integration snippet')}>
						Copy snippet
					</button>
				</div>
			</div>

			<div class="surface-box min-w-0">
				<div class="box-header">
					<div>
						<h2 class="box-title">Recent Security &amp; Audit Feed</h2>
						<p class="box-sub">Latest 15 events in this environment</p>
					</div>
				</div>

				{#if overview.recent_events.length === 0}
					<p class="empty-inline">No audit events recorded for this environment yet.</p>
				{:else}
					<div class="feed-list">
						{#each overview.recent_events as ev}
							<div class="feed-row">
								<div class="feed-main">
									<span class="feed-event">{ev.event}</span>
									<span class="feed-meta">{ev.actor_type}{ev.resource_type ? ` · ${ev.resource_type}` : ''}</span>
								</div>
								<span class="feed-time">{formatRelativeTimestamp(ev.created_at)}</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	{/if}
</section>

<style>
	.surface-box {
		background-color: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 12px;
		padding: 1.35rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.box-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.box-title {
		font-size: 0.98rem;
		font-weight: 600;
		margin: 0;
		color: var(--color-brand-text-primary);
	}

	.box-sub {
		font-size: 0.78rem;
		color: var(--color-brand-text-secondary);
		margin: 0.2rem 0 0;
	}

	.seg-tabs {
		display: inline-flex;
		background-color: var(--color-brand-surface);
		border: 1px solid var(--color-brand-border);
		border-radius: 6px;
		padding: 2px;
	}

	.seg-tabs button {
		background: transparent;
		border: none;
		color: var(--color-brand-text-secondary);
		font-size: 0.75rem;
		font-weight: 500;
		padding: 0.3rem 0.65rem;
		border-radius: 4px;
		cursor: pointer;
	}

	.seg-tabs button.active {
		background-color: #ffffff;
		color: var(--color-brand-surface);
	}

	.kv-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.kv-row {
		display: grid;
		grid-template-columns: minmax(0, 112px) minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.75rem;
		padding: 0.55rem 0.75rem;
		background-color: var(--color-brand-surface);
		border: 1px solid var(--color-brand-border);
		border-radius: 6px;
	}

	.kv-label {
		font-size: 0.74rem;
		color: var(--color-brand-text-secondary);
	}

	.kv-code {
		font-size: 0.76rem;
		color: var(--color-brand-text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.copy-btn {
		background: transparent;
		border: 1px solid var(--color-brand-border-hover);
		color: #d4d4d4;
		font-size: 0.72rem;
		padding: 0.2rem 0.5rem;
		border-radius: 4px;
		cursor: pointer;
	}

	.copy-btn:hover {
		border-color: #ffffff;
		color: #ffffff;
	}

	.code-panel {
		position: relative;
		background-color: var(--color-brand-surface);
		border: 1px solid var(--color-brand-border);
		border-radius: 8px;
		padding: 0.95rem;
	}

	.code-panel pre {
		margin: 0;
		overflow-x: auto;
		color: #d4d4d4;
		font-size: 0.76rem;
		line-height: 1.55;
	}

	.copy-snippet {
		position: absolute;
		top: 0.6rem;
		right: 0.6rem;
		background-color: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border-hover);
		color: #d4d4d4;
		font-size: 0.7rem;
		padding: 0.25rem 0.55rem;
		border-radius: 4px;
		cursor: pointer;
	}

	.feed-list {
		display: flex;
		flex-direction: column;
	}

	.feed-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		padding: 0.6rem 0;
		border-bottom: 1px solid var(--color-brand-border);
	}

	.feed-row:last-child {
		border-bottom: none;
	}

	.feed-main {
		display: flex;
		flex-direction: column;
		gap: 0.12rem;
		min-width: 0;
	}

	.feed-event {
		font-family: var(--font-mono);
		font-size: 0.76rem;
		color: var(--color-brand-text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.feed-meta {
		font-size: 0.72rem;
		color: var(--color-brand-text-secondary);
	}

	.feed-time {
		font-size: 0.74rem;
		color: var(--color-brand-text-secondary);
		white-space: nowrap;
	}

	.empty-surface,
	.empty-inline {
		padding: 2rem;
		text-align: center;
		color: var(--color-brand-text-secondary);
		font-size: 0.85rem;
		background-color: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 12px;
	}

	.empty-inline {
		background: transparent;
		border: none;
		padding: 1.5rem 0;
	}
</style>

