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
			<span class="brand-icon" aria-hidden="true">🔐</span>
			<span class="brand-text">Auth<span class="brand-accent">Platform</span></span>
		</a>

		<nav class="nav-links" aria-label="Main Navigation">
			{#if authStore.isAuthenticated}
				<a href="/dashboard" class="nav-link" class:active={isActive('/dashboard')}>
					Dashboard
				</a>
				<a href="/settings" class="nav-link" class:active={isActive('/settings')}>
					Settings
				</a>
				{#if authStore.isAdmin}
					<a href="/admin" class="nav-link nav-link-admin" class:active={isActive('/admin')}>
						<span class="admin-indicator" aria-hidden="true">★</span> Admin
					</a>
				{/if}
			{:else}
				<a href="/" class="nav-link" class:active={page.url.pathname === '/'}>
					Home
				</a>
				<a href="/login" class="nav-link" class:active={isActive('/login')}>
					Sign In
				</a>
				<a href="/register" class="nav-link" class:active={isActive('/register')}>
					Register
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
						{#if authStore.isAdmin}
							<Badge variant="purple" size="sm">Admin</Badge>
						{:else}
							<Badge variant="neutral" size="sm">User</Badge>
						{/if}
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
				<Button variant="primary" size="sm" href="/login">
					Get Started
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
		background: rgba(10, 14, 23, 0.75);
		backdrop-filter: blur(16px);
		-webkit-backdrop-filter: blur(16px);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.container {
		max-width: 1200px;
		margin: 0 auto;
		padding: 0 1.5rem;
	}

	.navbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 68px;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		text-decoration: none;
		font-weight: 700;
		font-size: 1.25rem;
		color: #ffffff;
		letter-spacing: -0.02em;
	}

	.brand-icon {
		font-size: 1.4rem;
	}

	.brand-accent {
		background: linear-gradient(135deg, #818cf8 0%, #c084fc 100%);
		-webkit-background-clip: text;
		background-clip: text;
		-webkit-text-fill-color: transparent;
	}

	.nav-links {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.nav-link {
		color: #94a3b8;
		text-decoration: none;
		font-weight: 500;
		font-size: 0.92rem;
		padding: 0.5rem 0.85rem;
		border-radius: 8px;
		transition: all 0.15s ease;
	}

	.nav-link:hover {
		color: #ffffff;
		background: rgba(255, 255, 255, 0.05);
	}

	.nav-link.active {
		color: #ffffff;
		background: rgba(99, 102, 241, 0.15);
		border: 1px solid rgba(99, 102, 241, 0.3);
	}

	.nav-link-admin {
		color: #c084fc;
	}

	.nav-link-admin:hover {
		color: #e9d5ff;
		background: rgba(168, 85, 247, 0.15);
	}

	.nav-link-admin.active {
		color: #ffffff;
		background: rgba(168, 85, 247, 0.2);
		border: 1px solid rgba(168, 85, 247, 0.35);
	}

	.admin-indicator {
		font-size: 0.75rem;
		margin-right: 0.15rem;
	}

	.nav-actions {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.user-profile-pill {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.35rem 0.75rem 0.35rem 0.4rem;
		border-radius: 9999px;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 600;
		font-size: 0.8rem;
		color: #ffffff;
		box-shadow: 0 2px 8px rgba(99, 102, 241, 0.4);
	}

	.user-meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.user-name {
		font-size: 0.875rem;
		font-weight: 500;
		color: #e2e8f0;
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
