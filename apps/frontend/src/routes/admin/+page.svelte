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
	<title>Admin Console — AuthPlatform</title>
	<meta name="description" content="Administrative console for managing users and role assignments." />
</svelte:head>

<div class="admin-container container">
	<div class="admin-header">
		<div>
			<div class="admin-badge-row">
				<Badge variant="purple" size="sm">Admin Role Required</Badge>
			</div>
			<h1 class="admin-title">User Management Console</h1>
			<p class="admin-subtitle">
				Inspect registered platform accounts, review permissions, and manage user lifecycles directly via the Rust API.
			</p>
		</div>

		<div class="admin-actions">
			<Button variant="outline" onclick={loadUsers} {loading}>
				Refresh Users
			</Button>
		</div>
	</div>

	{#if error}
		<div class="admin-error-box" role="alert">
			<span>⚠</span> {error}
		</div>
	{/if}

	<div class="search-bar">
		<span class="search-icon">🔍</span>
		<input
			type="text"
			placeholder="Search users by name, email, or UUID..."
			bind:value={searchQuery}
			class="search-input"
		/>
		{#if searchQuery}
			<button class="clear-search" onclick={() => (searchQuery = '')} aria-label="Clear search">
				✕
			</button>
		{/if}
	</div>

	<Card variant="glass">
		{#if loading && users.length === 0}
			<div class="state-message">
				<div class="spinner"></div>
				<p>Querying Rust API for registered users...</p>
			</div>
		{:else if filteredUsers.length === 0}
			<div class="state-message">
				<p>No matching users found.</p>
			</div>
		{:else}
			<div class="table-responsive">
				<table class="user-table">
					<thead>
						<tr>
							<th>User</th>
							<th>Status</th>
							<th>Verification</th>
							<th>Joined</th>
							<th class="text-right">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each filteredUsers as user (user.id)}
							<tr class:current-user-row={user.id === authStore.user?.id}>
								<td>
									<div class="user-cell">
										<div class="table-avatar">
											{getUserInitials(user)}
										</div>
										<div class="user-info">
											<span class="user-name">
												{user.display_name}
												{#if user.id === authStore.user?.id}
													<span class="you-tag">(You)</span>
												{/if}
											</span>
											<span class="user-email">{user.email}</span>
											<span class="user-id font-mono">{user.id}</span>
										</div>
									</div>
								</td>
								<td>
									{#if user.status === 'active'}
										<Badge variant="success" size="sm">Active</Badge>
									{:else}
										<Badge variant="warning" size="sm">{user.status}</Badge>
									{/if}
								</td>
								<td>
									{#if user.email_verified}
										<Badge variant="success" size="sm">Verified</Badge>
									{:else}
										<Badge variant="neutral" size="sm">Unverified</Badge>
									{/if}
								</td>
								<td class="font-mono date-cell">
									{formatDate(user.created_at)}
								</td>
								<td class="text-right">
									{#if user.id !== authStore.user?.id}
										<Button
											variant="danger"
											size="sm"
											onclick={() => confirmDelete(user)}
										>
											Delete
										</Button>
									{:else}
										<span class="protected-pill">Protected</span>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
		{#snippet footer()}
			<span class="footer-count">Total registered accounts: {filteredUsers.length}</span>
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

<style>
	.container {
		max-width: 1100px;
		margin: 0 auto;
		padding: 2rem 1.5rem 4rem;
	}

	.admin-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 2rem;
		flex-wrap: wrap;
		gap: 1rem;
	}

	.admin-badge-row {
		margin-bottom: 0.5rem;
	}

	.admin-title {
		font-size: 2rem;
		font-weight: 700;
		color: #ffffff;
		margin: 0 0 0.4rem;
		letter-spacing: -0.02em;
	}

	.admin-subtitle {
		margin: 0;
		color: #94a3b8;
		font-size: 0.95rem;
		max-width: 600px;
		line-height: 1.5;
	}

	.admin-error-box {
		padding: 1rem 1.25rem;
		background: rgba(239, 68, 68, 0.12);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 10px;
		color: #f87171;
		font-size: 0.92rem;
		margin-bottom: 1.5rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.search-bar {
		position: relative;
		margin-bottom: 1.5rem;
		display: flex;
		align-items: center;
	}

	.search-icon {
		position: absolute;
		left: 1rem;
		font-size: 0.95rem;
		pointer-events: none;
		opacity: 0.5;
	}

	.search-input {
		width: 100%;
		padding: 0.8rem 2.5rem 0.8rem 2.75rem;
		background: rgba(15, 23, 42, 0.6);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 10px;
		color: #f8fafc;
		font-size: 0.95rem;
		outline: none;
		transition: border-color 0.2s ease;
	}

	.search-input:focus {
		border-color: #818cf8;
	}

	.clear-search {
		position: absolute;
		right: 1rem;
		background: transparent;
		border: none;
		color: #94a3b8;
		cursor: pointer;
		font-size: 0.9rem;
	}

	.table-responsive {
		overflow-x: auto;
	}

	.user-table {
		width: 100%;
		border-collapse: collapse;
		text-align: left;
	}

	.user-table th {
		padding: 0.85rem 1rem;
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #94a3b8;
		font-weight: 600;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.user-table td {
		padding: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		font-size: 0.9rem;
		color: #cbd5e1;
		vertical-align: middle;
	}

	.current-user-row {
		background: rgba(99, 102, 241, 0.04);
	}

	.user-cell {
		display: flex;
		align-items: center;
		gap: 0.85rem;
	}

	.table-avatar {
		width: 38px;
		height: 38px;
		border-radius: 50%;
		background: linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 600;
		font-size: 0.85rem;
		color: #ffffff;
		flex-shrink: 0;
	}

	.user-info {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}

	.user-name {
		font-weight: 600;
		color: #f8fafc;
	}

	.you-tag {
		font-size: 0.75rem;
		color: #818cf8;
		font-weight: 500;
	}

	.user-email {
		font-size: 0.8rem;
		color: #94a3b8;
	}

	.user-id {
		font-size: 0.725rem;
		color: #64748b;
	}

	.font-mono {
		font-family: var(--font-mono, monospace);
	}

	.date-cell {
		font-size: 0.825rem;
		color: #94a3b8;
	}

	.text-right {
		text-align: right;
	}

	.protected-pill {
		font-size: 0.75rem;
		color: #64748b;
		padding: 0.25rem 0.5rem;
		background: rgba(255, 255, 255, 0.04);
		border-radius: 4px;
	}

	.state-message {
		text-align: center;
		padding: 3rem 1rem;
		color: #94a3b8;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
	}

	.footer-count {
		font-size: 0.85rem;
		color: #94a3b8;
	}
</style>
