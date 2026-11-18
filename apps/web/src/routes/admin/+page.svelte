<script lang="ts">
	import Card from '$lib/components/Card.svelte';
	import Button from '$lib/components/Button.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { authStore } from '$lib/stores/auth.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { usersApi } from '$lib/api/users';
	import type { UserProfile } from '$lib/api/types';
	import { ApiError } from '$lib/api/client';
	import { getUserInitials } from '$lib/auth/session';
	import { onMount } from 'svelte';

	let users = $state<UserProfile[]>([]);
	let loading = $state(false);
	let error = $state('');
	let searchQuery = $state('');

	let userToDelete = $state<UserProfile | null>(null);
	let deletingUser = $state(false);

	onMount(async () => {
		await loadUsers();
	});

	async function loadUsers() {
		loading = true;
		error = '';
		try {
			users = await usersApi.listUsers({ limit: 50, offset: 0 });
		} catch (err) {
			if (err instanceof ApiError) {
				error = err.message;
				addToast(err.message, 'error');
			} else {
				error = 'Failed to load users from backend API';
			}
		} finally {
			loading = false;
		}
	}

	function confirmDelete(user: UserProfile) {
		userToDelete = user;
	}

	async function executeDeleteUser() {
		if (!userToDelete) return;
		deletingUser = true;
		try {
			await usersApi.deleteUser(userToDelete.id);
			users = users.filter((u) => u.id !== userToDelete?.id);
			addToast(`User ${userToDelete.email} deleted successfully`, 'success');
			userToDelete = null;
		} catch (err) {
			if (err instanceof ApiError) {
				addToast(err.message, 'error');
			} else {
				addToast('Failed to delete user', 'error');
			}
		} finally {
			deletingUser = false;
		}
	}

	let filteredUsers = $derived(
		users.filter(
			(u) =>
				u.email.toLowerCase().includes(searchQuery.toLowerCase()) ||
				u.display_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
				u.id.toLowerCase().includes(searchQuery.toLowerCase())
		)
	);

	function formatDate(iso: string): string {
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
	<title>Admin Console — Signet</title>
	<meta name="description" content="Administrative console for managing users and role assignments." />
</svelte:head>

<div class="max-w-6xl mx-auto px-6 py-8 pb-16">
	<div class="flex flex-wrap justify-between items-start gap-4 mb-8">
		<div>
			<div class="mb-2">
				<Badge variant="purple" size="sm">Admin Role Required</Badge>
			</div>
			<h1 class="text-3xl font-bold font-heading text-brand-text-primary tracking-tight mb-1">User Management Console</h1>
			<p class="text-sm text-brand-text-secondary max-w-xl leading-relaxed m-0">
				Inspect registered platform accounts, review permissions, and manage user lifecycles directly via the Rust API.
			</p>
		</div>

		<div class="flex items-center gap-3">
			<Button variant="outline" onclick={loadUsers} {loading}>
				Refresh Users
			</Button>
		</div>
	</div>

	{#if error}
		<div class="p-3.5 bg-rose-500/10 border border-rose-500/30 rounded-xl text-rose-300 text-sm mb-6 flex items-center gap-2" role="alert">{error}</div>
	{/if}

	<div class="relative mb-6 flex items-center">
		<input
			type="text"
			placeholder="Search users by name, email, or UUID..."
			bind:value={searchQuery}
			class="w-full pl-4 pr-10 py-3 bg-brand-surface-card border border-brand-border rounded-xl text-brand-text-primary text-sm outline-none placeholder:text-brand-text-subtle focus:border-brand-lime transition-colors"
		/>
		{#if searchQuery}
			<button class="absolute right-3.5 bg-transparent border-none text-brand-text-muted hover:text-brand-text-primary cursor-pointer text-sm p-1" onclick={() => (searchQuery = '')} aria-label="Clear search">
				✕
			</button>
		{/if}
	</div>

	<Card variant="glass">
		{#if loading && users.length === 0}
			<div class="text-center py-12 px-4 text-brand-text-muted flex flex-col items-center gap-3">
				<div class="w-6 h-6 border-2 border-brand-lime border-r-transparent rounded-full animate-spin"></div>
				<p class="m-0 text-sm">Querying Rust API for registered users...</p>
			</div>
		{:else if filteredUsers.length === 0}
			<div class="text-center py-12 px-4 text-brand-text-muted flex flex-col items-center gap-3">
				<p class="m-0 text-sm">No matching users found.</p>
			</div>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full border-collapse text-left">
					<thead>
						<tr>
							<th class="px-4 py-3 text-xs uppercase tracking-wider text-brand-text-muted font-semibold border-b border-white/10">User</th>
							<th class="px-4 py-3 text-xs uppercase tracking-wider text-brand-text-muted font-semibold border-b border-white/10">Status</th>
							<th class="px-4 py-3 text-xs uppercase tracking-wider text-brand-text-muted font-semibold border-b border-white/10">Verification</th>
							<th class="px-4 py-3 text-xs uppercase tracking-wider text-brand-text-muted font-semibold border-b border-white/10">Joined</th>
							<th class="px-4 py-3 text-xs uppercase tracking-wider text-brand-text-muted font-semibold border-b border-white/10 text-right">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each filteredUsers as user (user.id)}
							<tr class="transition-colors {user.id === authStore.user?.id ? 'bg-brand-lime/[0.04]' : 'hover:bg-white/[0.02]'}">
								<td class="px-4 py-3.5 border-b border-white/5 text-sm align-middle">
									<div class="flex items-center gap-3">
										<div class="w-9 h-9 rounded-full bg-brand-lime/10 text-brand-lime border border-brand-lime/20 flex items-center justify-center font-bold text-xs shrink-0">
											{getUserInitials(user)}
										</div>
										<div class="flex flex-col gap-0.5">
											<span class="font-semibold text-brand-text-primary text-sm">
												{user.display_name}
												{#if user.id === authStore.user?.id}
													<span class="text-xs text-brand-lime font-medium ml-1">(You)</span>
												{/if}
											</span>
											<span class="text-xs text-brand-text-muted">{user.email}</span>
											<span class="text-[11px] font-mono text-brand-text-subtle">{user.id}</span>
										</div>
									</div>
								</td>
								<td class="px-4 py-3.5 border-b border-white/5 text-sm align-middle">
									{#if user.status === 'active'}
										<Badge variant="success" size="sm">Active</Badge>
									{:else}
										<Badge variant="warning" size="sm">{user.status}</Badge>
									{/if}
								</td>
								<td class="px-4 py-3.5 border-b border-white/5 text-sm align-middle">
									{#if user.email_verified}
										<Badge variant="success" size="sm">Verified</Badge>
									{:else}
										<Badge variant="neutral" size="sm">Unverified</Badge>
									{/if}
								</td>
								<td class="px-4 py-3.5 border-b border-white/5 text-xs font-mono text-brand-text-secondary align-middle">
									{formatDate(user.created_at)}
								</td>
								<td class="px-4 py-3.5 border-b border-white/5 text-sm align-middle text-right">
									{#if user.id !== authStore.user?.id}
										<Button
											variant="danger"
											size="sm"
											onclick={() => confirmDelete(user)}
										>
											Delete
										</Button>
									{:else}
										<span class="text-xs text-brand-text-subtle px-2 py-1 bg-white/5 rounded">Protected</span>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
		{#snippet footer()}
			<span class="text-xs text-brand-text-muted">Total registered accounts: {filteredUsers.length}</span>
		{/snippet}
	</Card>
</div>

<Modal
	open={userToDelete !== null}
	title="Delete User Account?"
	description="Are you sure you want to permanently delete the account for {userToDelete?.email}? This action cannot be undone and will revoke all related sessions and tokens."
	confirmText="Permanently Delete"
	variant="danger"
	loading={deletingUser}
	onconfirm={executeDeleteUser}
	oncancel={() => (userToDelete = null)}
/>
