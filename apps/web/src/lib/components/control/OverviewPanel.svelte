<script lang="ts">
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import type { EnvironmentOverviewDto } from '$lib/api/types';
	import { computeSuccessRate, formatRelativeTimestamp } from '$lib/control/permissions';

	interface Props {
		overview: EnvironmentOverviewDto | null;
		loading: boolean;
		appName: string;
		appSlug: string;
		onRefresh: () => void;
	}

	let { overview, loading, appName, appSlug, onRefresh }: Props = $props();
	let snippetTab = $state<'sveltekit' | 'nextjs' | 'oidc'>('sveltekit');

	const authRate24h = $derived(
		overview ? computeSuccessRate(overview.auth_success_24h, overview.auth_failure_24h) : 100
	);
	const webhookRate24h = $derived(
		overview
			? computeSuccessRate(
					overview.webhook_deliveries_succeeded_24h,
					overview.webhook_deliveries_failed_24h
				)
			: 100
	);
	const activeUserShare = $derived(
		overview && overview.total_users > 0
			? Math.round((overview.active_users / overview.total_users) * 100)
			: 100
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

<section class="panel-stack" aria-label="Environment overview">
	<div class="section-header">
		<div>
			<h1 class="page-title">Overview</h1>
			<p class="page-subtitle">
				{appName} · {overview?.environment_kind === 'production' ? 'Production' : 'Development'} environment health and quick integration
			</p>
		</div>
		<Button variant="outline" size="sm" onclick={onRefresh} disabled={loading}>
			{loading ? 'Refreshing...' : 'Refresh'}
		</Button>
	</div>

	{#if loading && !overview}
		<div class="empty-surface">Loading environment telemetry...</div>
	{:else if !overview}
		<div class="empty-surface">Select an application and environment to view overview metrics.</div>
	{:else}
		<div class="bento-grid">
			<div class="kpi-highlight">
				<div class="kpi-top">
					<span class="kpi-kicker">Total users in environment</span>
					<span class="kpi-pill">{overview.active_sessions} active sessions</span>
				</div>
				<div class="kpi-hero">{overview.total_users.toLocaleString()}</div>
				<div class="kpi-foot">
					<span>{overview.active_users} active · {overview.suspended_users} suspended · {overview.unverified_users} unverified</span>
					<div class="meter-track meter-track-light">
						<div class="meter-fill meter-fill-dark" style={`width: ${activeUserShare}%`}></div>
					</div>
				</div>
			</div>

			<div class="kpi-card">
				<div class="kpi-card-head">
					<span class="card-kicker">Authentication health</span>
					<Badge variant={overview.auth_failure_24h > 10 ? 'warning' : 'success'} size="sm" dot>
						{authRate24h}% success
					</Badge>
				</div>
				<div class="metric-rows">
					<div class="metric-row">
						<span>24h sign-ins succeeded</span>
						<strong>{overview.auth_success_24h}</strong>
					</div>
					<div class="metric-row">
						<span>24h failures &amp; lockouts</span>
						<strong class:text-warn={overview.auth_failure_24h > 0}>{overview.auth_failure_24h}</strong>
					</div>
					<div class="metric-row">
						<span>7d sign-ins succeeded</span>
						<strong>{overview.auth_success_7d}</strong>
					</div>
					<div class="metric-row">
						<span>New users (7d)</span>
						<strong>+{overview.new_users_7d}</strong>
					</div>
				</div>
			</div>

			<div class="kpi-card">
				<div class="kpi-card-head">
					<span class="card-kicker">Sync &amp; API health</span>
					<Badge variant={overview.webhook_dead_letters > 0 ? 'danger' : 'neutral'} size="sm" dot>
						{overview.webhook_dead_letters > 0 ? `${overview.webhook_dead_letters} DLQ` : 'Healthy'}
					</Badge>
				</div>
				<div class="metric-rows">
					<div class="metric-row">
						<span>Webhook delivery rate (24h)</span>
						<strong>{webhookRate24h}%</strong>
					</div>
					<div class="metric-row">
						<span>Active webhook endpoints</span>
						<strong>{overview.webhook_endpoints_active}</strong>
					</div>
					<div class="metric-row">
						<span>Failed deliveries (24h)</span>
						<strong class:text-err={overview.webhook_deliveries_failed_24h > 0}>
							{overview.webhook_deliveries_failed_24h}
						</strong>
					</div>
					<div class="metric-row">
						<span>Active API credentials</span>
						<strong>{overview.active_api_keys}</strong>
					</div>
				</div>
			</div>
		</div>

		<div class="two-col">
			<div class="surface-box">
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

			<div class="surface-box">
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
	.panel-stack {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.section-header {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.page-title {
		font-size: 1.75rem;
		font-weight: 600;
		color: #ffffff;
		margin: 0;
	}

	.page-subtitle {
		font-size: 0.85rem;
		color: #71717a;
		margin: 0.25rem 0 0;
	}

	.bento-grid {
		display: grid;
		grid-template-columns: 1.35fr 1fr 1fr;
		gap: 1rem;
	}

	.kpi-highlight {
		background-color: #f3f4f6;
		color: #0b0c0e;
		border-radius: 12px;
		padding: 1.35rem;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		gap: 1.15rem;
	}

	.kpi-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.kpi-kicker {
		font-size: 0.8rem;
		font-weight: 500;
		color: #52525b;
	}

	.kpi-pill {
		font-size: 0.72rem;
		font-weight: 600;
		background-color: #0b0c0e;
		color: #ffffff;
		padding: 0.2rem 0.55rem;
		border-radius: 9999px;
	}

	.kpi-hero {
		font-size: 2.5rem;
		font-weight: 700;
		letter-spacing: -0.04em;
		line-height: 1;
		color: #0b0c0e;
	}

	.kpi-foot {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		font-size: 0.76rem;
		color: #52525b;
	}

	.meter-track {
		width: 100%;
		height: 5px;
		border-radius: 9999px;
		overflow: hidden;
	}

	.meter-track-light {
		background-color: #d4d4d8;
	}

	.meter-fill {
		height: 100%;
		border-radius: 9999px;
	}

	.meter-fill-dark {
		background-color: #0b0c0e;
	}

	.kpi-card {
		background-color: #15171b;
		border: 1px solid #23262e;
		border-radius: 12px;
		padding: 1.35rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.kpi-card-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.card-kicker {
		font-size: 0.82rem;
		font-weight: 600;
		color: #e4e4e7;
	}

	.metric-rows {
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
	}

	.metric-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 0.82rem;
		color: #a1a1aa;
		padding-bottom: 0.5rem;
		border-bottom: 1px solid #1f2228;
	}

	.metric-row:last-child {
		border-bottom: none;
		padding-bottom: 0;
	}

	.metric-row strong {
		color: #f4f4f5;
		font-family: var(--font-mono);
		font-size: 0.84rem;
	}

	.text-warn { color: #facc15 !important; }
	.text-err { color: #f87171 !important; }

	.two-col {
		display: grid;
		grid-template-columns: 1.25fr 1fr;
		gap: 1rem;
	}

	.surface-box {
		background-color: #15171b;
		border: 1px solid #23262e;
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
		color: #f4f4f5;
	}

	.box-sub {
		font-size: 0.78rem;
		color: #71717a;
		margin: 0.2rem 0 0;
	}

	.seg-tabs {
		display: inline-flex;
		background-color: #0b0c0e;
		border: 1px solid #23262e;
		border-radius: 6px;
		padding: 2px;
	}

	.seg-tabs button {
		background: transparent;
		border: none;
		color: #a1a1aa;
		font-size: 0.75rem;
		font-weight: 500;
		padding: 0.3rem 0.65rem;
		border-radius: 4px;
		cursor: pointer;
	}

	.seg-tabs button.active {
		background-color: #ffffff;
		color: #0b0c0e;
	}

	.kv-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.kv-row {
		display: grid;
		grid-template-columns: 112px 1fr auto;
		align-items: center;
		gap: 0.75rem;
		padding: 0.55rem 0.75rem;
		background-color: #0b0c0e;
		border: 1px solid #23262e;
		border-radius: 6px;
	}

	.kv-label {
		font-size: 0.74rem;
		color: #71717a;
	}

	.kv-code {
		font-size: 0.76rem;
		color: #e4e4e7;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.copy-btn {
		background: transparent;
		border: 1px solid #2e333d;
		color: #d4d4d8;
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
		background-color: #0b0c0e;
		border: 1px solid #23262e;
		border-radius: 8px;
		padding: 0.95rem;
	}

	.code-panel pre {
		margin: 0;
		overflow-x: auto;
		color: #d4d4d8;
		font-size: 0.76rem;
		line-height: 1.55;
	}

	.copy-snippet {
		position: absolute;
		top: 0.6rem;
		right: 0.6rem;
		background-color: #15171b;
		border: 1px solid #2e333d;
		color: #d4d4d8;
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
		border-bottom: 1px solid #1f2228;
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
		color: #f4f4f5;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.feed-meta {
		font-size: 0.72rem;
		color: #71717a;
	}

	.feed-time {
		font-size: 0.74rem;
		color: #71717a;
		white-space: nowrap;
	}

	.empty-surface,
	.empty-inline {
		padding: 2rem;
		text-align: center;
		color: #71717a;
		font-size: 0.85rem;
		background-color: #15171b;
		border: 1px solid #23262e;
		border-radius: 12px;
	}

	.empty-inline {
		background: transparent;
		border: none;
		padding: 1.5rem 0;
	}

	@media (max-width: 1080px) {
		.bento-grid,
		.two-col {
			grid-template-columns: 1fr;
		}
	}
</style>

