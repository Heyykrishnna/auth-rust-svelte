<script lang="ts">
	import SectionHeader from './SectionHeader.svelte';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import type {
		AnalyticsWindow,
		EnvironmentAnalyticsDto,
		EnvironmentKind
	} from '$lib/api/types';

	interface Props {
		appId: string;
		envKind: EnvironmentKind;
		canMutate: boolean;
		onNavigate?: (section: 'audit' | 'webhooks' | 'users') => void;
	}

	let { appId, envKind, canMutate, onNavigate }: Props = $props();

	const windows: { id: AnalyticsWindow; label: string }[] = [
		{ id: '24h', label: '24h' },
		{ id: '7d', label: '7d' },
		{ id: '30d', label: '30d' },
		{ id: '90d', label: '90d' }
	];

	let selectedWindow = $state<AnalyticsWindow>('7d');
	let report = $state<EnvironmentAnalyticsDto | null>(null);
	let loading = $state(false);
	let refreshing = $state(false);
	let hoveredIdx = $state<number | null>(null);

	async function loadAnalytics() {
		loading = true;
		try {
			report = await controlApi.getAnalytics(appId, envKind, selectedWindow);
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load analytics', 'error');
		} finally {
			loading = false;
		}
	}

	async function handleRefreshRollups() {
		if (!canMutate) {
			await loadAnalytics();
			return;
		}
		refreshing = true;
		try {
			await controlApi.refreshAnalytics(appId, envKind, selectedWindow);
			await loadAnalytics();
			addToast('Analytics rollups refreshed', 'success');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to refresh rollups', 'error');
		} finally {
			refreshing = false;
		}
	}

	$effect(() => {
		const _app = appId;
		const _env = envKind;
		const _win = selectedWindow;
		void _app;
		void _env;
		void _win;
		loadAnalytics();
	});

	const signInSuccessRate = $derived.by(() => {
		if (!report) return 100;
		const total = report.summary.sign_in_success + report.summary.sign_in_failure;
		if (total === 0) return 100;
		return Math.round((report.summary.sign_in_success / total) * 1000) / 10;
	});

	const maxAuthPoint = $derived.by(() => {
		if (!report || report.series.length === 0) return 1;
		return Math.max(
			1,
			...report.series.map((p) => p.sign_in_success + p.sign_in_failure)
		);
	});

	const maxApiPoint = $derived.by(() => {
		if (!report || report.series.length === 0) return 1;
		return Math.max(
			1,
			...report.series.map((p) => p.client_api_requests + p.backend_api_requests)
		);
	});

	const providerRows = $derived.by(() => {
		if (!report) return [];
		const b = report.provider_breakdown;
		const total = Math.max(1, b.password + b.google + b.github + b.oidc_sso);
		return [
			{ label: 'Password', count: b.password, pct: Math.round((b.password / total) * 100) },
			{ label: 'Google OAuth', count: b.google, pct: Math.round((b.google / total) * 100) },
			{ label: 'GitHub OAuth', count: b.github, pct: Math.round((b.github / total) * 100) },
			{ label: 'OIDC SSO Silent Hit', count: b.oidc_sso, pct: Math.round((b.oidc_sso / total) * 100) }
		];
	});

	function formatBucketLabel(iso: string, granularity: string): string {
		const d = new Date(iso);
		if (Number.isNaN(d.getTime())) return iso;
		if (granularity === 'hour') {
			return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		}
		return d.toLocaleDateString([], { month: 'short', day: 'numeric' });
	}
</script>

<section class="grid min-w-0 grid-cols-1 gap-5 sm:gap-6" aria-label="Environment analytics">
	<SectionHeader title="Analytics" description="Environment-isolated usage, authentication trends, provider mix, and webhook reliability.">
		<div class="flex min-w-0 flex-wrap items-center gap-2">
			<div class="window-picker" role="group" aria-label="Date range">
				{#each windows as item (item.id)}
					<button
						type="button"
						class:active={selectedWindow === item.id}
						onclick={() => (selectedWindow = item.id)}
					>
						{item.label}
					</button>
				{/each}
			</div>
			<Button variant="outline" size="sm" disabled={refreshing || loading} onclick={handleRefreshRollups}>
				{refreshing ? 'Refreshing...' : 'Refresh'}
			</Button>
		</div>
	</SectionHeader>

	{#if loading && !report}
		<div class="empty-box">Loading environment analytics...</div>
	{:else if report}
		<div class="kpi-grid">
			<article class="kpi-card highlight">
				<span class="kpi-label">Daily Active Users</span>
				<strong class="kpi-value">{report.summary.dau.toLocaleString()}</strong>
				<span class="kpi-sub">MAU: {report.summary.mau.toLocaleString()}</span>
			</article>
			<article class="kpi-card">
				<span class="kpi-label">New Registrations</span>
				<strong class="kpi-value">{report.summary.registrations.toLocaleString()}</strong>
				<span class="kpi-sub">Window: {report.window}</span>
			</article>
			<article class="kpi-card">
				<span class="kpi-label">Sign-In Success Rate</span>
				<strong class="kpi-value">{signInSuccessRate}%</strong>
				<span class="kpi-sub">
					{report.summary.sign_in_success} ok / {report.summary.sign_in_failure} failed
				</span>
			</article>
			<article class="kpi-card">
				<span class="kpi-label">Active Sessions</span>
				<strong class="kpi-value">{report.summary.active_sessions.toLocaleString()}</strong>
				<span class="kpi-sub">Lockouts: {report.summary.lockouts}</span>
			</article>
			<article class="kpi-card">
				<span class="kpi-label">API Requests</span>
				<strong class="kpi-value">
					{(report.summary.client_api_requests + report.summary.backend_api_requests).toLocaleString()}
				</strong>
				<span class="kpi-sub">
					Client {report.summary.client_api_requests} · Backend {report.summary.backend_api_requests}
				</span>
			</article>
			<article class="kpi-card">
				<span class="kpi-label">Webhook Reliability</span>
				<strong class="kpi-value">{report.summary.webhook_success_rate}%</strong>
				<span class="kpi-sub">
					Avg {report.summary.webhook_avg_latency_ms}ms · DLQ {report.summary.webhook_deliveries_dead_letter}
				</span>
			</article>
		</div>

		<div class="charts-grid">
			<div class="chart-card">
				<div class="chart-head">
					<div>
						<h2>Authentication Activity</h2>
						<p>Successful vs failed sign-ins ({report.granularity} buckets)</p>
					</div>
					<div class="legend">
						<span><i class="swatch swatch-ok"></i>Success</span>
						<span><i class="swatch swatch-fail"></i>Failure</span>
					</div>
				</div>

				<div class="svg-wrap">
					<svg viewBox="0 0 600 180" class="bar-chart" role="img" aria-label="Authentication activity chart">
						<line x1="0" y1="160" x2="600" y2="160" stroke="#262626" stroke-width="1" />
						<line x1="0" y1="80" x2="600" y2="80" stroke="#1e1e1e" stroke-width="1" />
						{#each report.series as point, idx (point.bucket_start)}
							{@const slot = 600 / Math.max(1, report.series.length)}
							{@const barW = Math.max(4, slot - 4)}
							{@const okH = (point.sign_in_success / maxAuthPoint) * 144}
							{@const failH = (point.sign_in_failure / maxAuthPoint) * 144}
							<g
								role="presentation"
								onmouseenter={() => (hoveredIdx = idx)}
								onmouseleave={() => (hoveredIdx = null)}
							>
								<rect
									x={idx * slot + 2}
									y={160 - okH - failH}
									width={barW}
									height={Math.max(0, failH)}
									fill="#686868"
								/>
								<rect
									x={idx * slot + 2}
									y={160 - okH}
									width={barW}
									height={Math.max(1, okH)}
									fill="#ffffff"
								/>
							</g>
						{/each}
					</svg>
				</div>

				{#if hoveredIdx !== null && report.series[hoveredIdx]}
					{@const pt = report.series[hoveredIdx]}
					<div class="hover-strip">
						<span>{formatBucketLabel(pt.bucket_start, report.granularity)}</span>
						<span>Success: <strong>{pt.sign_in_success}</strong></span>
						<span>Failed: <strong>{pt.sign_in_failure}</strong></span>
						<span>Registrations: <strong>{pt.registrations}</strong></span>
					</div>
				{:else}
					<div class="hover-strip muted">
						Hover over bars to inspect per-bucket sign-in and registration counts.
					</div>
				{/if}
			</div>

			<div class="chart-card">
				<div class="chart-head">
					<div>
						<h2>API Request Volume</h2>
						<p>Client plane vs Backend sync plane requests</p>
					</div>
					<div class="legend">
						<span><i class="swatch swatch-ok"></i>Client</span>
						<span><i class="swatch swatch-muted"></i>Backend</span>
					</div>
				</div>

				<div class="svg-wrap">
					<svg viewBox="0 0 600 180" class="bar-chart" role="img" aria-label="API request volume chart">
						<line x1="0" y1="160" x2="600" y2="160" stroke="#262626" stroke-width="1" />
						<line x1="0" y1="80" x2="600" y2="80" stroke="#1e1e1e" stroke-width="1" />
						{#each report.series as point, idx (point.bucket_start)}
							{@const slot = 600 / Math.max(1, report.series.length)}
							{@const barW = Math.max(4, slot - 4)}
							{@const clientH = (point.client_api_requests / maxApiPoint) * 144}
							{@const backendH = (point.backend_api_requests / maxApiPoint) * 144}
							<rect
								x={idx * slot + 2}
								y={160 - clientH - backendH}
								width={barW}
								height={Math.max(0, backendH)}
								fill="#727272"
							/>
							<rect
								x={idx * slot + 2}
								y={160 - clientH}
								width={barW}
								height={Math.max(1, clientH)}
								fill="#e7e7e7"
							/>
						{/each}
					</svg>
				</div>

				<div class="drilldown-row">
					<Button variant="outline" size="sm" onclick={() => onNavigate?.('audit')}>
						Inspect Audit Logs
					</Button>
					<Button variant="outline" size="sm" onclick={() => onNavigate?.('webhooks')}>
						Inspect Webhook Deliveries
					</Button>
					<Button variant="outline" size="sm" onclick={() => onNavigate?.('users')}>
						Browse Users
					</Button>
				</div>
			</div>
		</div>

		<div class="bottom-grid">
			<div class="chart-card">
				<div class="chart-head">
					<div>
						<h2>Authentication Provider Breakdown</h2>
						<p>Method distribution for successful sign-ins</p>
					</div>
				</div>
				<div class="provider-list">
					{#each providerRows as row (row.label)}
						<div class="provider-item">
							<div class="provider-meta">
								<span>{row.label}</span>
								<strong>{row.count} ({row.pct}%)</strong>
							</div>
							<div class="bar-track">
								<div class="bar-fill" style={`width: ${row.pct}%`}></div>
							</div>
						</div>
					{/each}
				</div>
			</div>

			<div class="chart-card">
				<div class="chart-head">
					<div>
						<h2>Security &amp; Failure Breakdown</h2>
						<p>Top failure events and anomaly indicators</p>
					</div>
					{#if report.summary.refresh_reuse_detected > 0}
						<p class="text-sm font-medium text-brand-danger-text" role="status">
							{report.summary.refresh_reuse_detected} token reuse alerts
						</p>
					{/if}
				</div>

				{#if report.summary.failure_reasons.length === 0}
					<p class="muted">No authentication failure events recorded in this window.</p>
				{:else}
					<div class="reason-list">
						{#each report.summary.failure_reasons as item (item.reason)}
							<div class="reason-row">
								<code>{item.reason}</code>
								<strong>{item.count}</strong>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	{/if}
</section>

<style>
	.window-picker {
		display: inline-flex;
		background: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 6px;
		padding: 2px;
	}

	.window-picker button {
		background: transparent;
		border: none;
		color: var(--color-brand-text-secondary);
		padding: 0.3rem 0.65rem;
		font-size: 0.78rem;
		border-radius: 4px;
		cursor: pointer;
	}

	.window-picker button.active {
		background: var(--color-brand-surface-raised);
		color: var(--color-brand-text-primary);
	}

	.empty-box {
		background: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 8px;
		padding: 2rem;
		text-align: center;
		color: var(--color-brand-text-secondary);
	}

	.kpi-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(175px, 1fr));
		gap: 0.85rem;
	}

	.kpi-card {
		background: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 8px;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	.kpi-card.highlight {
		background: #f4f4f4;
		border-color: #d5d5d5;
		color: var(--color-brand-surface);
	}

	.kpi-card.highlight .kpi-label,
	.kpi-card.highlight .kpi-sub {
		color: #545454;
	}

	.kpi-card.highlight .kpi-value {
		color: var(--color-brand-surface);
	}

	.kpi-label {
		font-size: 0.75rem;
		color: var(--color-brand-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.kpi-value {
		font-size: 1.55rem;
		font-weight: 600;
		color: var(--color-brand-text-primary);
		line-height: 1.15;
	}

	.kpi-sub {
		font-size: 0.76rem;
		color: var(--color-brand-text-secondary);
	}

	.charts-grid,
	.bottom-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 1rem;
	}

	@media (max-width: 960px) {
		.charts-grid,
		.bottom-grid {
			grid-template-columns: 1fr;
		}
	}

	.chart-card {
		background: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 8px;
		padding: 1.1rem;
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
	}

	.chart-head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.75rem;
	}

	.chart-head h2 {
		margin: 0;
		font-size: 0.96rem;
		color: var(--color-brand-text-primary);
	}

	.chart-head p {
		margin: 0.2rem 0 0;
		font-size: 0.78rem;
		color: var(--color-brand-text-muted);
	}

	.legend {
		display: flex;
		gap: 0.75rem;
		font-size: 0.75rem;
		color: var(--color-brand-text-secondary);
	}

	.legend span {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
	}

	.swatch {
		display: inline-block;
		width: 8px;
		height: 8px;
		border-radius: 2px;
	}

	.swatch-ok {
		background: #ffffff;
	}

	.swatch-fail {
		background: #686868;
	}

	.swatch-muted {
		background: #727272;
	}

	.svg-wrap {
		width: 100%;
		overflow: hidden;
	}

	.bar-chart {
		width: 100%;
		height: 180px;
		display: block;
	}

	.hover-strip {
		display: flex;
		gap: 1rem;
		flex-wrap: wrap;
		font-size: 0.78rem;
		color: var(--color-brand-text-secondary);
		padding-top: 0.35rem;
		border-top: 1px solid var(--color-brand-border);
	}

	.drilldown-row {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		padding-top: 0.35rem;
		border-top: 1px solid var(--color-brand-border);
	}

	.provider-list,
	.reason-list {
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
	}

	.provider-item {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	.provider-meta {
		display: flex;
		justify-content: space-between;
		font-size: 0.82rem;
		color: var(--color-brand-text-secondary);
	}

	.provider-meta strong {
		color: var(--color-brand-text-primary);
	}

	.bar-track {
		height: 7px;
		background: var(--color-brand-surface);
		border: 1px solid var(--color-brand-border);
		border-radius: 4px;
		overflow: hidden;
	}

	.bar-fill {
		height: 100%;
		background: #ffffff;
	}

	.reason-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem 0.65rem;
		background: var(--color-brand-surface);
		border: 1px solid var(--color-brand-border);
		border-radius: 6px;
		font-size: 0.82rem;
	}

	.muted {
		color: var(--color-brand-text-muted);
		font-size: 0.8rem;
		margin: 0;
	}
</style>

