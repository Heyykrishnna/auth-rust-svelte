<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { authStore, logout } from '$lib/stores/auth.svelte';
	import { getUserInitials, formatUserDisplayName } from '$lib/auth/session';
	import { addToast } from '$lib/components/Toast.svelte';
	import Badge from './Badge.svelte';
	import Button from './Button.svelte';

	let loggingOut = $state(false);

	async function handleLogout() {
		loggingOut = true;
		try {
			await logout();
			addToast('Signed out successfully', 'info');
			goto('/login');
		} finally {
			loggingOut = false;
		}
	}

	function isActive(path: string): boolean {
		return page.url.pathname === path || page.url.pathname.startsWith(`${path}/`);
	}
</script>

<header class="navbar-wrapper">
	<div class="navbar container">
		<a href={authStore.isAuthenticated ? '/dashboard' : '/'} class="brand">
			<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
				<line x1="12" y1="2" x2="12" y2="22" />
				<line x1="2" y1="12" x2="22" y2="12" />
				<line x1="5" y1="5" x2="19" y2="19" />
				<line x1="19" y1="5" x2="5" y2="19" />
			</svg>
			<span class="brand-text">Secure Auth</span>
		</a>

		<nav class="nav-links" aria-label="Main Navigation">
			{#if authStore.isAuthenticated}
				<a href="/dashboard" class="nav-link" class:active={isActive('/dashboard')}>
					Dashboard
				</a>
				<a href="/settings" class="nav-link" class:active={isActive('/settings')}>
					Settings
				</a>
				<a href="/control" class="nav-link" class:active={isActive('/control')}>
					Control Plane
				</a>
				{#if authStore.isAdmin}
					<a href="/admin" class="nav-link" class:active={isActive('/admin')}>
						Admin
					</a>
				{/if}
			{:else}
				<a href="/" class="nav-link" class:active={page.url.pathname === '/'}>
					Overview
				</a>
				<a href="/control" class="nav-link" class:active={isActive('/control')}>
					Control Plane
				</a>
				<a href="/login" class="nav-link" class:active={isActive('/login')}>
					Sign In
				</a>
				<a href="/register" class="nav-link" class:active={isActive('/register')}>
					Create Account
				</a>
			{/if}
		</nav>

		<div class="nav-actions">
			{#if authStore.isAuthenticated && authStore.user}
				<div class="user-profile-pill">
					<div class="avatar" title={authStore.user.email}>
						{getUserInitials(authStore.user)}
					</div>
					<div class="user-meta">
						<span class="user-name">{formatUserDisplayName(authStore.user)}</span>
						<Badge variant="neutral" size="sm">{authStore.isAdmin ? 'Admin' : 'User'}</Badge>
					</div>
				</div>

				<Button
					variant="ghost"
					size="sm"
					onclick={handleLogout}
					loading={loggingOut}
					disabled={loggingOut}
				>
					Sign Out
				</Button>
			{:else}
				<Button variant="primary" size="sm" href="/control/login">
					Console
				</Button>
			{/if}
		</div>
	</div>
</header>

<style>
	.navbar-wrapper {
		position: sticky;
		top: 0;
		z-index: 50;
		background-color: var(--color-bg-sidebar);
		border-bottom: 1px solid var(--color-border);
	}

	.container {
		max-width: 1240px;
		margin: 0 auto;
		padding: 0 1.5rem;
	}

	.navbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 60px;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		text-decoration: none;
		font-weight: 600;
		font-size: 1rem;
		color: var(--color-text);
		letter-spacing: -0.02em;
	}

	.nav-links {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	.nav-link {
		color: var(--color-text-muted);
		text-decoration: none;
		font-weight: 500;
		font-size: 0.875rem;
		padding: 0.45rem 0.75rem;
		border-radius: 6px;
		border: 1px solid transparent;
	}

	.nav-link:hover {
		color: var(--color-text);
		background-color: var(--color-bg-elevated);
	}

	.nav-link.active {
		color: var(--color-text);
		background-color: var(--color-bg-elevated);
		border-color: var(--color-border-hover);
	}

	.nav-actions {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.user-profile-pill {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.25rem 0.65rem 0.25rem 0.3rem;
		border-radius: 9999px;
		background-color: var(--color-bg-surface);
		border: 1px solid var(--color-border);
	}

	.avatar {
		width: 28px;
		height: 28px;
		border-radius: 50%;
		background-color: #ffffff;
		color: #0b0c0e;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 600;
		font-size: 0.72rem;
	}

	.user-meta {
		display: flex;
		align-items: center;
		gap: 0.45rem;
	}

	.user-name {
		font-size: 0.82rem;
		font-weight: 500;
		color: var(--color-text);
		max-width: 140px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	@media (max-width: 768px) {
		.user-meta {
			display: none;
		}
	}
</style>
