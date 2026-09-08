<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import Input from '$lib/components/Input.svelte';
	import { authStore, logout, setAuth } from '$lib/stores/auth.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { authApi, type SessionItem } from '$lib/api/auth';
	import { ApiError } from '$lib/api/client';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	type DashboardTab = 'overview' | 'sessions' | 'security' | 'profile';

	let activeTab = $state<DashboardTab>('overview');
	let loggingOut = $state(false);

	let sessions = $state<SessionItem[]>([]);
	let sessionsLoading = $state(false);
	let revokingId = $state<string | null>(null);

	let editDisplayName = $state('');
	let profileLoading = $state(false);
	let profileError = $state('');

	onMount(() => {
		if (!authStore.isAuthenticated) {
			goto('/login');
		} else if (authStore.user) {
			editDisplayName = authStore.user.display_name;
		}
	});

	$effect(() => {
		if (authStore.user && !editDisplayName) {
			editDisplayName = authStore.user.display_name;
		}
	});

	async function handleLogout() {
		loggingOut = true;
		try {
			await logout();
			addToast('You have been signed out', 'info');
			goto('/login');
		} finally {
			loggingOut = false;
		}
	}

	async function loadSessions() {
		sessionsLoading = true;
		try {
			const data = await authApi.listSessions();
			sessions = data;
		} catch (err) {
			if (err instanceof ApiError) {
				addToast(err.message, 'error');
			}
		} finally {
			sessionsLoading = false;
		}
	}

	async function handleRevokeSession(sessionId: string) {
		revokingId = sessionId;
		try {
			await authApi.revokeSession(sessionId);
			sessions = sessions.filter((s) => s.id !== sessionId);
			addToast('Session successfully revoked', 'success');
		} catch (err) {
			if (err instanceof ApiError) {
				addToast(err.message, 'error');
			} else {
				addToast('Failed to revoke session', 'error');
			}
		} finally {
			revokingId = null;
		}
	}

	async function handleUpdateProfile(e: SubmitEvent) {
		e.preventDefault();
		if (!editDisplayName.trim() || editDisplayName.trim().length < 2) {
			profileError = 'Display name must be at least 2 characters';
			return;
		}

		profileError = '';
		profileLoading = true;
		try {
			const updated = await authApi.updateProfile({ display_name: editDisplayName.trim() });
			setAuth(updated);
			addToast('Profile updated successfully', 'success');
		} catch (err) {
			if (err instanceof ApiError) {
				profileError = err.message;
			} else {
				profileError = 'Failed to update profile';
			}
		} finally {
			profileLoading = false;
		}
	}

	function switchTab(tab: DashboardTab) {
		activeTab = tab;
		if (tab === 'sessions') {
			loadSessions();
		}
	}

	const navItems: { id: DashboardTab; label: string; icon: string }[] = [
		{ id: 'overview', label: 'Overview', icon: '📊' },
		{ id: 'sessions', label: 'Sessions', icon: '🔑' },
		{ id: 'security', label: 'Security', icon: '🛡️' },
		{ id: 'profile', label: 'Profile', icon: '👤' }
	];
</script>

<svelte:head>
	<title>Dashboard — AuthApp</title>
	<meta name="description" content="Your AuthApp dashboard." />
</svelte:head>

{#if authStore.isAuthenticated && authStore.user}
	<div class="dashboard-layout">
		<!-- Sidebar -->
		<aside class="sidebar">
			<div class="sidebar-logo">
				<div class="auth-logo-icon">🔐</div>
				<span style="font-weight: 700; color: var(--color-text);">AuthApp</span>
			</div>

			<!-- User Card -->
			<div class="user-card glass-card">
				<div class="user-avatar">
					{authStore.user.display_name.charAt(0).toUpperCase()}
				</div>
				<div class="user-info">
					<p class="user-name">{authStore.user.display_name}</p>
					<p class="user-email">{authStore.user.email}</p>
				</div>
				{#if authStore.user.email_verified}
					<span class="badge badge-success" title="Email verified">✓</span>
				{:else}
					<span class="badge badge-warning" title="Email not verified">!</span>
				{/if}
			</div>

			<!-- Nav -->
			<nav class="sidebar-nav">
				{#each navItems as item}
					<button
						type="button"
						class="nav-item {activeTab === item.id ? 'active' : ''}"
						onclick={() => switchTab(item.id)}
					>
						<span class="nav-icon">{item.icon}</span>
						<span>{item.label}</span>
					</button>
				{/each}
			</nav>

			<div class="sidebar-footer">
				<Button variant="ghost" full loading={loggingOut} onclick={handleLogout}>
					{loggingOut ? 'Signing out…' : '↩ Sign Out'}
				</Button>
			</div>
		</aside>

		<!-- Main Content -->
		<main class="main-content">
			{#if activeTab === 'overview'}
				<div class="content-header">
					<div>
						<h1>Welcome back, {authStore.user.display_name.split(' ')[0]}! 👋</h1>
						<p>Here's an overview of your account activity.</p>
					</div>
				</div>

				<!-- Stats Grid -->
				<div class="stats-grid">
					<div class="stat-card animate-fade-in-up" style="--delay: 0ms">
						<p class="stat-label">Account Status</p>
						<p class="stat-value">Active</p>
						<span class="badge badge-success stat-delta up">● Online</span>
					</div>
					<div class="stat-card animate-fade-in-up" style="--delay: 100ms">
						<p class="stat-label">Email Verified</p>
						<p class="stat-value">{authStore.user.email_verified ? 'Yes' : 'No'}</p>
						<span
							class="badge {authStore.user.email_verified
								? 'badge-success'
								: 'badge-warning'} stat-delta"
						>
							{authStore.user.email_verified ? '✓ Verified' : '⚠ Pending'}
						</span>
					</div>
					<div class="stat-card animate-fade-in-up" style="--delay: 200ms">
						<p class="stat-label">Member Since</p>
						<p class="stat-value">{new Date(authStore.user.created_at).getFullYear()}</p>
						<span class="stat-delta" style="color: var(--color-text-muted);">
							{new Date(authStore.user.created_at).toLocaleDateString('en-US', {
								month: 'long',
								day: 'numeric'
							})}
						</span>
					</div>
					<div class="stat-card animate-fade-in-up" style="--delay: 300ms">
						<p class="stat-label">User ID</p>
						<p class="stat-value" style="font-size: 0.9rem; font-family: var(--font-mono);">
							{authStore.user.id.slice(0, 8)}…
						</p>
						<span class="stat-delta" style="color: var(--color-text-subtle);">UUID v4</span>
					</div>
				</div>

				<!-- Account Details -->
				<section class="details-section glass-card animate-fade-in-up" style="--delay: 400ms">
					<h2>Account Details</h2>
					<div class="details-grid">
						<div class="detail-item">
							<span class="detail-label">Display Name</span>
							<span class="detail-value">{authStore.user.display_name}</span>
						</div>
						<div class="detail-item">
							<span class="detail-label">Email Address</span>
							<span class="detail-value">{authStore.user.email}</span>
						</div>
						<div class="detail-item">
							<span class="detail-label">Full User ID</span>
							<span class="detail-value mono">{authStore.user.id}</span>
						</div>
						<div class="detail-item">
							<span class="detail-label">Created At</span>
							<span class="detail-value">{new Date(authStore.user.created_at).toLocaleString()}</span>
						</div>
					</div>
				</section>
			{:else if activeTab === 'sessions'}
				<div class="content-header">
					<div>
						<h1>Active Sessions 🔑</h1>
						<p>Manage and revoke active sessions across your browsers and devices.</p>
					</div>
					<Button variant="outline" size="sm" onclick={loadSessions} loading={sessionsLoading}>
						Refresh
					</Button>
				</div>

				{#if sessionsLoading && sessions.length === 0}
					<div class="glass-card" style="padding: 2rem; text-align: center;">
						<div class="spinner" style="margin: 0 auto 1rem;"></div>
						<p>Loading active sessions…</p>
					</div>
				{:else if sessions.length === 0}
					<div class="glass-card" style="padding: 2.5rem; text-align: center;">
						<p>No other active sessions found.</p>
					</div>
				{:else}
					<div class="sessions-list">
						{#each sessions as s (s.id)}
							<div class="session-card glass-card">
								<div class="session-icon">💻</div>
								<div class="session-details">
									<div class="session-header">
										<span class="session-device">{s.user_agent || 'Unknown browser'}</span>
										{#if s.is_current}
											<span class="badge badge-success">Current Session</span>
										{/if}
									</div>
									<div class="session-meta">
										<span>IP: {s.ip_address || 'Unavailable'}</span>
										<span>•</span>
										<span>Created: {new Date(s.created_at).toLocaleDateString()}</span>
										<span>•</span>
										<span>Last active: {new Date(s.last_used_at).toLocaleTimeString()}</span>
									</div>
								</div>
								{#if !s.is_current}
									<Button
										variant="danger"
										size="sm"
										loading={revokingId === s.id}
										onclick={() => handleRevokeSession(s.id)}
									>
										Revoke
									</Button>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			{:else if activeTab === 'profile'}
				<div class="content-header">
					<div>
						<h1>Profile Settings 👤</h1>
						<p>Update your personal information and account settings.</p>
					</div>
				</div>

				<div class="glass-card" style="max-width: 600px; padding: var(--space-8);">
					{#if profileError}
						<div class="form-error" role="alert" style="margin-bottom: var(--space-4);">
							<span>⚠</span> {profileError}
						</div>
					{/if}

					<form onsubmit={handleUpdateProfile} class="profile-form">
						<Input
							id="profileDisplayName"
							label="Display Name"
							type="text"
							bind:value={editDisplayName}
							required
						/>

						<div style="margin-top: var(--space-4);">
							<label class="label" for="profileEmail">Email Address</label>
							<input
								id="profileEmail"
								type="text"
								class="input"
								value={authStore.user.email}
								disabled
								style="opacity: 0.6; cursor: not-allowed;"
							/>
							<p class="helper-text" style="margin-top: 0.25rem;">Email cannot be changed directly.</p>
						</div>

						<div style="margin-top: var(--space-6);">
							<Button type="submit" variant="primary" loading={profileLoading}>
								Save Changes
							</Button>
						</div>
					</form>
				</div>
			{:else if activeTab === 'security'}
				<div class="content-header">
					<div>
						<h1>Security Overview 🛡️</h1>
						<p>Security configurations protecting your account and data.</p>
					</div>
				</div>

				<div class="security-grid">
					<div class="security-card glass-card">
						<div class="security-header">
							<span class="security-badge">Argon2id</span>
							<h3>Password Hashing</h3>
						</div>
						<p>Credentials are safeguarded with Argon2id memory-hard PHC cryptographic hashing.</p>
					</div>

					<div class="security-card glass-card">
						<div class="security-header">
							<span class="security-badge">HttpOnly</span>
							<h3>Token Security</h3>
						</div>
						<p>Access and refresh tokens are stored in HttpOnly, SameSite=Lax cookies to prevent XSS exfiltration.</p>
					</div>

					<div class="security-card glass-card">
						<div class="security-header">
							<span class="security-badge">Redis Revocation</span>
							<h3>Session Controls</h3>
						</div>
						<p>Tokens can be blacklisted immediately upon logout with sub-millisecond Redis lookups.</p>
					</div>

					<div class="security-card glass-card">
						<div class="security-header">
							<span class="security-badge">Rate Limited</span>
							<h3>Abuse Protection</h3>
						</div>
						<p>Smart IP token-bucket rate limiting restricts brute-force attempts on sensitive endpoints.</p>
					</div>
				</div>
			{/if}
		</main>
	</div>
{:else}
	<div class="auth-layout">
		<div class="spinner" style="width: 2rem; height: 2rem; border-width: 3px;"></div>
	</div>
{/if}

<style>
	.sidebar-logo {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding-bottom: var(--space-4);
		border-bottom: 1px solid var(--color-border);
	}

	.user-card {
		padding: var(--space-4);
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.user-avatar {
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-full);
		background: linear-gradient(135deg, var(--color-primary), var(--color-accent));
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		color: white;
		flex-shrink: 0;
	}

	.user-info {
		flex: 1;
		min-width: 0;
	}
	.user-name {
		font-weight: 600;
		color: var(--color-text);
		font-size: 0.9rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.user-email {
		font-size: 0.75rem;
		color: var(--color-text-subtle);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.sidebar-nav {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		flex: 1;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
		font-size: 0.9375rem;
		font-weight: 500;
		background: none;
		border: none;
		cursor: pointer;
		text-align: left;
		transition: all var(--transition-fast);
		width: 100%;
	}

	.nav-item:hover {
		background: rgba(139, 92, 246, 0.1);
		color: var(--color-text);
	}

	.nav-item.active {
		background: rgba(139, 92, 246, 0.2);
		color: var(--color-primary-light);
		font-weight: 600;
	}

	.nav-icon {
		font-size: 1.1rem;
	}

	.sidebar-footer {
		margin-top: auto;
		padding-top: var(--space-4);
		border-top: 1px solid var(--color-border);
	}

	.content-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--space-8);
	}

	.content-header h1 {
		margin-bottom: var(--space-1);
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: var(--space-4);
		margin-bottom: var(--space-8);
	}

	.stat-card {
		animation-delay: var(--delay, 0ms);
	}

	.details-section {
		padding: var(--space-6);
		animation-delay: var(--delay, 0ms);
	}

	.details-section h2 {
		font-size: 1.25rem;
		margin-bottom: var(--space-6);
		color: var(--color-text);
	}

	.details-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: var(--space-6);
	}

	.detail-item {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.detail-label {
		font-size: 0.8125rem;
		color: var(--color-text-subtle);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.detail-value {
		font-size: 0.9375rem;
		color: var(--color-text);
		font-weight: 500;
	}
	.mono {
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		color: var(--color-text-muted);
	}

	/* Sessions styling */
	.sessions-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.session-card {
		padding: var(--space-4) var(--space-6);
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}

	.session-icon {
		font-size: 1.5rem;
	}

	.session-details {
		flex: 1;
	}

	.session-header {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		margin-bottom: 0.25rem;
	}

	.session-device {
		font-weight: 600;
		color: var(--color-text);
	}

	.session-meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.8125rem;
		color: var(--color-text-subtle);
	}

	/* Security styling */
	.security-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: var(--space-6);
	}

	.security-card {
		padding: var(--space-6);
	}

	.security-header {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		margin-bottom: var(--space-3);
	}

	.security-badge {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		padding: 0.2rem 0.5rem;
		border-radius: var(--radius-sm);
		background: rgba(139, 92, 246, 0.2);
		color: var(--color-primary-light);
	}

	.form-error {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.25);
		border-radius: var(--radius-md);
		color: #fca5a5;
		font-size: 0.9rem;
	}
</style>
