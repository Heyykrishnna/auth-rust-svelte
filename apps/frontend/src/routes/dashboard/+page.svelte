<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import { authStore, logout } from '$lib/stores/auth.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	onMount(() => {
		if (!authStore.isAuthenticated) {
			goto('/login');
		}
	});

	let loggingOut = $state(false);

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

	const navItems = [
		{ href: '/dashboard', label: 'Overview', icon: '📊' },
		{ href: '/dashboard/sessions', label: 'Sessions', icon: '🔑' },
		{ href: '/dashboard/security', label: 'Security', icon: '🛡️' },
		{ href: '/profile', label: 'Profile', icon: '👤' }
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
					<a href={item.href} class="nav-item">
						<span class="nav-icon">{item.icon}</span>
						<span>{item.label}</span>
					</a>
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
					<span class="badge {authStore.user.email_verified ? 'badge-success' : 'badge-warning'} stat-delta">
						{authStore.user.email_verified ? '✓ Verified' : '⚠ Pending'}
					</span>
				</div>
				<div class="stat-card animate-fade-in-up" style="--delay: 200ms">
					<p class="stat-label">Member Since</p>
					<p class="stat-value">{new Date(authStore.user.created_at).getFullYear()}</p>
					<span class="stat-delta" style="color: var(--color-text-muted);">
						{new Date(authStore.user.created_at).toLocaleDateString('en-US', { month: 'long', day: 'numeric' })}
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

	.user-info { flex: 1; min-width: 0; }
	.user-name  { font-weight: 600; color: var(--color-text); font-size: 0.9rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
	.user-email { font-size: 0.75rem; color: var(--color-text-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

	.sidebar-nav { display: flex; flex-direction: column; gap: var(--space-1); flex: 1; }

	.nav-item {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
		font-size: 0.9375rem;
		font-weight: 500;
		transition: all var(--transition-fast);
		text-decoration: none;
	}

	.nav-item:hover {
		background: rgba(139, 92, 246, 0.1);
		color: var(--color-text);
	}

	.nav-icon { font-size: 1.1rem; }

	.sidebar-footer { margin-top: auto; padding-top: var(--space-4); border-top: 1px solid var(--color-border); }

	.content-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--space-8);
	}

	.content-header h1 { margin-bottom: var(--space-1); }

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: var(--space-4);
		margin-bottom: var(--space-8);
	}

	.stat-card { animation-delay: var(--delay, 0ms); }

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

	.detail-item { display: flex; flex-direction: column; gap: var(--space-1); }
	.detail-label { font-size: 0.8125rem; color: var(--color-text-subtle); text-transform: uppercase; letter-spacing: 0.05em; }
	.detail-value { font-size: 0.9375rem; color: var(--color-text); font-weight: 500; }
	.mono { font-family: var(--font-mono); font-size: 0.8125rem; color: var(--color-text-muted); }
</style>
