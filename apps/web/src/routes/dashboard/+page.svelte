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
	<title>Dashboard — Signet</title>
	<meta name="description" content="View your account overview, security status, and platform metrics." />
</svelte:head>

<div class="max-w-7xl mx-auto px-6 py-8 pb-16">
	<div class="flex flex-wrap items-center justify-between gap-8 p-8 rounded-xl bg-brand-surface-card border border-brand-border mb-8">
		<div class="flex items-center gap-6">
			<div class="w-16 h-16 rounded-full bg-brand-lime text-brand-lime-ink flex items-center justify-center font-heading font-bold text-2xl shrink-0 select-none">
				{getUserInitials(authStore.user)}
			</div>
			<div class="flex flex-col">
				<div class="flex items-center gap-2 mb-2">
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
				<h1 class="m-0 mb-1.5 text-2xl sm:text-3xl font-bold font-heading text-brand-text-primary tracking-tight">
					Welcome, {formatUserDisplayName(authStore.user)}
				</h1>
				<p class="m-0 text-brand-text-secondary text-sm max-w-xl leading-relaxed">
					Connected to Rust Axum Auth API. Your session is protected by cryptographic tokens & HttpOnly cookies.
				</p>
			</div>
		</div>

		<div class="flex items-center gap-3">
			<Button href="/settings">
				Manage Settings
			</Button>
			{#if authStore.isAdmin}
				<Button href="/admin">
					Admin Console →
				</Button>
			{/if}
		</div>
	</div>

	<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-5 mb-8">
		<Card>
			<div class="flex items-start gap-4">
				<div class="w-12 h-12 rounded-xl bg-white/5 border border-white/10 flex items-center justify-center shrink-0">
					<span class="text-xs font-mono font-bold text-brand-lime">SEC</span>
				</div>
				<div class="flex flex-col gap-1">
					<span class="text-xs uppercase tracking-wider text-brand-text-muted font-semibold">Security State</span>
					<span class="text-xl font-bold text-emerald-400">Protected</span>
					<span class="text-xs text-brand-text-subtle">Argon2id + SameSite Cookies</span>
				</div>
			</div>
		</Card>

		<Card>
			<div class="flex items-start gap-4">
				<div class="w-12 h-12 rounded-xl bg-white/5 border border-white/10 flex items-center justify-center shrink-0">
					<span class="text-xs font-mono font-bold text-brand-lime">SES</span>
				</div>
				<div class="flex flex-col gap-1">
					<span class="text-xs uppercase tracking-wider text-brand-text-muted font-semibold">Active Devices</span>
					<span class="text-xl font-bold text-brand-text-primary">
						{#if loadingSessions}
							...
						{:else if activeSessionCount !== null}
							{activeSessionCount} {activeSessionCount === 1 ? 'Device' : 'Devices'}
						{:else}
							1 Device
						{/if}
					</span>
					<span class="text-xs text-brand-text-subtle">
						<a href="/settings" class="text-brand-lime hover:underline">Manage sessions →</a>
					</span>
				</div>
			</div>
		</Card>

		<Card>
			<div class="flex items-start gap-4">
				<div class="w-12 h-12 rounded-xl bg-white/5 border border-white/10 flex items-center justify-center shrink-0">
					<span class="text-xs font-mono font-bold text-brand-lime">RBAC</span>
				</div>
				<div class="flex flex-col gap-1">
					<span class="text-xs uppercase tracking-wider text-brand-text-muted font-semibold">Assigned Roles</span>
					<span class="text-xl font-bold text-brand-text-primary">
						{authStore.roles.length > 0 ? authStore.roles.join(', ') : 'user'}
					</span>
					<span class="text-xs text-brand-text-subtle">Role-Based Access Control</span>
				</div>
			</div>
		</Card>

		<Card>
			<div class="flex items-start gap-4">
				<div class="w-12 h-12 rounded-xl bg-white/5 border border-white/10 flex items-center justify-center shrink-0">
					<span class="text-xs font-mono font-bold text-brand-lime">DATE</span>
				</div>
				<div class="flex flex-col gap-1">
					<span class="text-xs uppercase tracking-wider text-brand-text-muted font-semibold">Member Since</span>
					<span class="text-xl font-bold font-mono text-brand-text-primary">
						{formatDate(authStore.user?.created_at)}
					</span>
					<span class="text-xs text-brand-text-subtle">Status: {authStore.user?.status || 'active'}</span>
				</div>
			</div>
		</Card>
	</div>

	<div class="grid grid-cols-1 lg:grid-cols-12 gap-6">
		<div class="lg:col-span-7">
			<Card title="Identity & Permissions" subtitle="Details resolved directly from backend Rust API tokens">
				<div class="flex flex-col gap-3">
					<div class="flex items-center justify-between py-2 border-b border-white/5">
						<span class="text-sm text-brand-text-secondary">User Identifier (UUID)</span>
						<span class="text-sm font-mono font-medium text-brand-text-primary">{authStore.user?.id || '—'}</span>
					</div>
					<div class="flex items-center justify-between py-2 border-b border-white/5">
						<span class="text-sm text-brand-text-secondary">Primary Email</span>
						<span class="text-sm font-medium text-brand-text-primary">{authStore.user?.email || '—'}</span>
					</div>
					<div class="flex items-center justify-between py-2 border-b border-white/5">
						<span class="text-sm text-brand-text-secondary">Display Name</span>
						<span class="text-sm font-medium text-brand-text-primary">{authStore.user?.display_name || '—'}</span>
					</div>
					<div class="flex items-center justify-between py-2">
						<span class="text-sm text-brand-text-secondary">Effective Permissions</span>
						<div class="flex flex-wrap gap-1.5 justify-end">
							{#if authStore.permissions.length > 0}
								{#each authStore.permissions as perm}
									<Badge variant="neutral" size="sm">{perm}</Badge>
								{/each}
							{:else}
								<span class="text-xs text-brand-text-subtle">Inherited based on role</span>
							{/if}
						</div>
					</div>
				</div>
				{#snippet footer()}
					<span class="text-xs text-brand-text-muted">Need to update your credentials?</span>
					<Button size="sm" href="/settings">
						Edit Profile in Settings →
					</Button>
				{/snippet}
			</Card>
		</div>

		<div class="lg:col-span-5">
			<Card title="Frontend Architecture" subtitle="Clean architectural boundaries">
				<div class="flex flex-col items-center gap-3 p-5 bg-black/30 rounded-xl border border-white/5">
					<div class="w-full p-3 rounded-lg bg-white/[0.03] border border-white/5 flex items-center gap-3">
						<div class="text-xs font-bold uppercase px-2 py-0.5 rounded bg-white/10 text-white whitespace-nowrap">Frontend</div>
						<p class="m-0 text-xs text-zinc-300">SvelteKit 5 Runes App (`src/routes`, `src/lib/api`, `src/lib/auth`, `hooks.server.ts`)</p>
					</div>
					<div class="text-xs font-semibold text-brand-lime">↓ REST HTTP (JSON + HttpOnly Cookies)</div>
					<div class="w-full p-3 rounded-lg bg-brand-lime/10 border border-brand-lime/30 flex items-center gap-3">
						<div class="text-xs font-bold uppercase px-2 py-0.5 rounded bg-brand-lime text-brand-lime-ink whitespace-nowrap font-heading">Rust Auth API</div>
						<p class="m-0 text-xs text-zinc-200">Axum Microservice (Business logic, token issuance, session validation, RBAC)</p>
					</div>
					<div class="text-xs font-semibold text-brand-lime">↓ Internal Isolated Backing Services</div>
					<div class="flex gap-3 w-full">
						<div class="flex-1 flex items-center gap-2 p-2.5 rounded-md bg-white/[0.02] border border-white/5 text-xs text-brand-text-secondary">
							<span class="font-mono font-bold text-brand-lime">PG</span>
							<span>PostgreSQL (Persistence)</span>
						</div>
						<div class="flex-1 flex items-center gap-2 p-2.5 rounded-md bg-white/[0.02] border border-white/5 text-xs text-brand-text-secondary">
							<span class="font-mono font-bold text-brand-lime">RD</span>
							<span>Redis (Session Store & Cache)</span>
						</div>
					</div>
				</div>
				<p class="mt-4 text-xs text-brand-text-secondary leading-relaxed m-0">
					Your Svelte frontend never touches PostgreSQL or Redis directly. It strictly consumes standard endpoints exposed by the Rust API layer.
				</p>
			</Card>
		</div>
	</div>
</div>
