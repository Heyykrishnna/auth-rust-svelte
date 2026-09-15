<script lang="ts">
	import Card from '$lib/components/Card.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import { authStore } from '$lib/stores/auth.svelte';
	import { formatUserDisplayName, getUserInitials } from '$lib/auth/session';
	import { sessionsApi } from '$lib/api/sessions';
	import { onMount } from 'svelte';

	let activeSessionCount = $state<number | null>(null);
	let loadingSessions = $state(false);

	onMount(async () => {
		if (authStore.isAuthenticated) {
			loadingSessions = true;
			try {
				const sessions = await sessionsApi.listSessions();
				activeSessionCount = sessions.length;
			} catch {
				activeSessionCount = null;
			} finally {
				loadingSessions = false;
			}
		}
	});

	function formatDate(iso: string | undefined): string {
		if (!iso) return 'N/A';
		try {
			return new Date(iso).toLocaleDateString(undefined, {
				year: 'numeric',
				month: 'short',
				day: 'numeric'
			});
		} catch {
			return iso;
		}
	}
</script>

<svelte:head>
	<title>Dashboard — AuthPlatform</title>
	<meta name="description" content="View your account overview, security status, and platform metrics." />
</svelte:head>

<div class="dashboard-container container">
	<div class="welcome-banner">
		<div class="welcome-content">
			<div class="user-avatar-large">
				{getUserInitials(authStore.user)}
			</div>
			<div class="welcome-text">
				<div class="welcome-badges">
					{#if authStore.isAdmin}
						<Badge variant="purple" dot>Administrator</Badge>
					{:else}
						<Badge variant="primary" dot>Standard User</Badge>
					{/if}
					{#if authStore.user?.email_verified}
						<Badge variant="success">Email Verified</Badge>
					{:else}
						<Badge variant="warning">Email Unverified</Badge>
					{/if}
				</div>
				<h1 class="welcome-title">
					Welcome, {formatUserDisplayName(authStore.user)}
				</h1>
				<p class="welcome-sub">
					Connected to Rust Axum Auth API. Your session is protected by cryptographic tokens & HttpOnly cookies.
				</p>
			</div>
		</div>

		<div class="welcome-actions">
			<Button variant="outline" href="/settings">
				Manage Settings
			</Button>
			{#if authStore.isAdmin}
				<Button variant="primary" href="/admin">
					Admin Console →
				</Button>
			{/if}
		</div>
	</div>

	<div class="stats-grid">
		<Card variant="glass">
			<div class="stat-card-inner">
				<div class="stat-icon-wrapper">
					<span class="stat-icon">🛡️</span>
				</div>
				<div class="stat-details">
					<span class="stat-title">Security State</span>
					<span class="stat-value text-success">Protected</span>
					<span class="stat-caption">Argon2id + SameSite Cookies</span>
				</div>
			</div>
		</Card>

		<Card variant="glass">
			<div class="stat-card-inner">
				<div class="stat-icon-wrapper">
					<span class="stat-icon">💻</span>
				</div>
				<div class="stat-details">
					<span class="stat-title">Active Devices</span>
					<span class="stat-value">
						{#if loadingSessions}
							...
						{:else if activeSessionCount !== null}
							{activeSessionCount} {activeSessionCount === 1 ? 'Device' : 'Devices'}
						{:else}
							1 Device
						{/if}
					</span>
					<span class="stat-caption">
						<a href="/settings" class="inline-link">Manage sessions →</a>
					</span>
				</div>
			</div>
		</Card>

		<Card variant="glass">
			<div class="stat-card-inner">
				<div class="stat-icon-wrapper">
					<span class="stat-icon">👑</span>
				</div>
				<div class="stat-details">
					<span class="stat-title">Assigned Roles</span>
					<span class="stat-value">
						{authStore.roles.length > 0 ? authStore.roles.join(', ') : 'user'}
					</span>
					<span class="stat-caption">Role-Based Access Control</span>
				</div>
			</div>
		</Card>

		<Card variant="glass">
			<div class="stat-card-inner">
				<div class="stat-icon-wrapper">
					<span class="stat-icon">📅</span>
				</div>
				<div class="stat-details">
					<span class="stat-title">Member Since</span>
					<span class="stat-value font-mono">
						{formatDate(authStore.user?.created_at)}
					</span>
					<span class="stat-caption">Status: {authStore.user?.status || 'active'}</span>
				</div>
			</div>
		</Card>
	</div>

	<div class="dashboard-sections">
		<Card title="Identity & Permissions" subtitle="Details resolved directly from backend Rust API tokens">
			<div class="info-table">
				<div class="info-row">
					<span class="info-label">User Identifier (UUID)</span>
					<span class="info-value font-mono">{authStore.user?.id || '—'}</span>
				</div>
				<div class="info-row">
					<span class="info-label">Primary Email</span>
					<span class="info-value">{authStore.user?.email || '—'}</span>
				</div>
				<div class="info-row">
					<span class="info-label">Display Name</span>
					<span class="info-value">{authStore.user?.display_name || '—'}</span>
				</div>
				<div class="info-row">
					<span class="info-label">Effective Permissions</span>
					<div class="permissions-pill-group">
						{#if authStore.permissions.length > 0}
							{#each authStore.permissions as perm}
								<Badge variant="neutral" size="sm">{perm}</Badge>
							{/each}
						{:else}
							<span class="info-subtle">Inherited based on role</span>
						{/if}
					</div>
				</div>
			</div>
			{#snippet footer()}
				<span class="footer-note">Need to update your credentials?</span>
				<Button variant="ghost" size="sm" href="/settings">
					Edit Profile in Settings →
				</Button>
			{/snippet}
		</Card>

		<Card title="Frontend Architecture" subtitle="Clean architectural boundaries">
			<div class="architecture-box">
				<div class="arch-step">
					<div class="arch-badge">Frontend</div>
					<p>SvelteKit 5 Runes App (`src/routes`, `src/lib/api`, `src/lib/auth`, `hooks.server.ts`)</p>
				</div>
				<div class="arch-arrow">↓ REST HTTP (JSON + HttpOnly Cookies)</div>
				<div class="arch-step highlight">
					<div class="arch-badge">Rust Auth API</div>
					<p>Axum Microservice (Business logic, token issuance, session validation, RBAC)</p>
				</div>
				<div class="arch-arrow">↓ Internal Isolated Backing Services</div>
				<div class="arch-backends">
					<div class="backend-item">
						<span class="backend-icon">🐘</span>
						<span>PostgreSQL (Persistence)</span>
					</div>
					<div class="backend-item">
						<span class="backend-icon">⚡</span>
						<span>Redis (Session Store & Cache)</span>
					</div>
				</div>
			</div>
				<p class="arch-explainer">
					Your Svelte frontend never touches PostgreSQL or Redis directly. It strictly consumes standard endpoints exposed by the Rust API layer.
				</p>
		</Card>
	</div>
</div>

<style>
	.container {
		max-width: 1200px;
		margin: 0 auto;
		padding: 2rem 1.5rem 4rem;
	}

	.welcome-banner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 2rem;
		padding: 2.5rem;
		border-radius: var(--radius-xl, 24px);
		background: linear-gradient(135deg, rgba(30, 41, 59, 0.7) 0%, rgba(15, 23, 42, 0.8) 100%);
		border: 1px solid rgba(255, 255, 255, 0.08);
		box-shadow: 0 20px 40px -15px rgba(0, 0, 0, 0.5);
		margin-bottom: 2rem;
		flex-wrap: wrap;
	}

	.welcome-content {
		display: flex;
		align-items: center;
		gap: 1.75rem;
	}

	.user-avatar-large {
		width: 72px;
		height: 72px;
		border-radius: 50%;
		background: linear-gradient(135deg, #6366f1 0%, #a855f7 100%);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		font-size: 1.75rem;
		color: #ffffff;
		box-shadow: 0 4px 20px rgba(99, 102, 241, 0.4);
		flex-shrink: 0;
	}

	.welcome-badges {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
	}

	.welcome-title {
		margin: 0 0 0.4rem;
		font-size: 1.85rem;
		font-weight: 700;
		color: #ffffff;
		letter-spacing: -0.02em;
	}

	.welcome-sub {
		margin: 0;
		color: #94a3b8;
		font-size: 0.95rem;
		max-width: 600px;
		line-height: 1.5;
	}

	.welcome-actions {
		display: flex;
		gap: 0.75rem;
		align-items: center;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
		gap: 1.25rem;
		margin-bottom: 2rem;
	}

	.stat-card-inner {
		display: flex;
		align-items: flex-start;
		gap: 1.25rem;
	}

	.stat-icon-wrapper {
		width: 48px;
		height: 48px;
		border-radius: 12px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.08);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.stat-icon {
		font-size: 1.4rem;
	}

	.stat-details {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.stat-title {
		font-size: 0.8rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #94a3b8;
		font-weight: 600;
	}

	.stat-value {
		font-size: 1.35rem;
		font-weight: 700;
		color: #f8fafc;
	}

	.text-success {
		color: #4ade80;
	}

	.stat-caption {
		font-size: 0.775rem;
		color: #64748b;
	}

	.inline-link {
		color: #818cf8;
		text-decoration: none;
	}

	.inline-link:hover {
		text-decoration: underline;
	}

	.dashboard-sections {
		display: grid;
		grid-template-columns: 1.2fr 1fr;
		gap: 1.5rem;
	}

	.info-table {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.info-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	.info-row:last-child {
		border-bottom: none;
		padding-bottom: 0;
	}

	.info-label {
		color: #94a3b8;
		font-size: 0.875rem;
	}

	.info-value {
		color: #f1f5f9;
		font-weight: 500;
		font-size: 0.92rem;
	}

	.font-mono {
		font-family: var(--font-mono, monospace);
		font-size: 0.85rem;
	}

	.permissions-pill-group {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
		justify-content: flex-end;
	}

	.info-subtle {
		font-size: 0.825rem;
		color: #64748b;
	}

	.footer-note {
		color: #94a3b8;
		font-size: 0.85rem;
	}

	.architecture-box {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		padding: 1.25rem;
		background: rgba(0, 0, 0, 0.2);
		border-radius: 12px;
		border: 1px solid rgba(255, 255, 255, 0.05);
	}

	.arch-step {
		width: 100%;
		padding: 0.75rem 1rem;
		border-radius: 8px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.arch-step.highlight {
		background: rgba(99, 102, 241, 0.1);
		border-color: rgba(99, 102, 241, 0.3);
	}

	.arch-badge {
		font-size: 0.75rem;
		font-weight: 700;
		text-transform: uppercase;
		padding: 0.2rem 0.5rem;
		border-radius: 4px;
		background: rgba(255, 255, 255, 0.1);
		color: #ffffff;
		white-space: nowrap;
	}

	.arch-step.highlight .arch-badge {
		background: #6366f1;
	}

	.arch-step p {
		margin: 0;
		font-size: 0.825rem;
		color: #cbd5e1;
	}

	.arch-arrow {
		font-size: 0.75rem;
		font-weight: 600;
		color: #818cf8;
	}

	.arch-backends {
		display: flex;
		gap: 1rem;
		width: 100%;
	}

	.backend-item {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.6rem 0.75rem;
		border-radius: 6px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid rgba(255, 255, 255, 0.05);
		font-size: 0.8rem;
		color: #94a3b8;
	}

	.arch-explainer {
		margin: 1rem 0 0;
		font-size: 0.85rem;
		color: #94a3b8;
		line-height: 1.5;
	}

	@media (max-width: 900px) {
		.dashboard-sections {
			grid-template-columns: 1fr;
		}

		.welcome-banner {
			flex-direction: column;
			align-items: flex-start;
		}
	}
</style>
