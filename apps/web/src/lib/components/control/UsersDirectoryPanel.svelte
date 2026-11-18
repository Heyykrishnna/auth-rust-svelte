<script lang="ts">
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import type {
		EnvironmentKind,
		EnvironmentUserDetailDto,
		EnvironmentUserListItemDto
	} from '$lib/api/types';
	import { formatRelativeTimestamp, getInitials } from '$lib/control/permissions';

	interface Props {
		appId: string;
		envKind: EnvironmentKind;
		canMutate: boolean;
	}

	let { appId, envKind, canMutate }: Props = $props();

	let users = $state<EnvironmentUserListItemDto[]>([]);
	let total = $state(0);
	let limit = $state(20);
	let offset = $state(0);
	let loading = $state(false);

	let statusTab = $state<'all' | 'active' | 'suspended' | 'unverified'>('all');
	let searchInput = $state('');
	let roleFilter = $state('');
	let viewMode = $state<'table' | 'cards'>('table');
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	let selectedUser = $state<EnvironmentUserDetailDto | null>(null);
	let detailLoading = $state(false);
	let drawerOpen = $state(false);
	let busyAction = $state(false);

	let editExternalId = $state('');
	let editPublicMeta = $state('{}');
	let editPrivateMeta = $state('{}');
	let metaError = $state('');

	async function loadUsers() {
		loading = true;
		try {
			const res = await controlApi.listUsers(appId, envKind, {
				q: searchInput.trim() || undefined,
				status:
					statusTab === 'active' || statusTab === 'suspended' ? statusTab : undefined,
				email_verified: statusTab === 'unverified' ? false : undefined,
				role: roleFilter.trim() || undefined,
				limit,
				offset
			});
			users = res.users;
			total = res.total;
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load users', 'error');
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const _app = appId;
		const _env = envKind;
		const _tab = statusTab;
		const _offset = offset;
		const _role = roleFilter;
		void _app;
		void _env;
		void _tab;
		void _offset;
		void _role;
		loadUsers();
	});

	function handleSearchInput(e: Event) {
		searchInput = (e.currentTarget as HTMLInputElement).value;
		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => {
			offset = 0;
			loadUsers();
		}, 250);
	}

	async function openUserDetail(appUserId: string) {
		drawerOpen = true;
		detailLoading = true;
		metaError = '';
		try {
			const detail = await controlApi.getUserDetail(appId, envKind, appUserId);
			selectedUser = detail;
			editExternalId = detail.external_id ?? '';
			editPublicMeta = JSON.stringify(detail.public_metadata ?? {}, null, 2);
			editPrivateMeta = JSON.stringify(detail.private_metadata ?? {}, null, 2);
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load user details', 'error');
			drawerOpen = false;
		} finally {
			detailLoading = false;
		}
	}

	async function saveMetadata() {
		if (!selectedUser || !canMutate) return;
		metaError = '';
		let parsedPublic: Record<string, unknown>;
		let parsedPrivate: Record<string, unknown>;
		try {
			parsedPublic = JSON.parse(editPublicMeta || '{}') as Record<string, unknown>;
			parsedPrivate = JSON.parse(editPrivateMeta || '{}') as Record<string, unknown>;
		} catch {
			metaError = 'Metadata must be valid JSON objects.';
			return;
		}
		busyAction = true;
		try {
			await controlApi.updateUserMetadata(appId, envKind, selectedUser.app_user_id, {
				external_id: editExternalId.trim() || null,
				public_metadata: parsedPublic,
				private_metadata: parsedPrivate
			});
			addToast('User metadata updated', 'success');
			await openUserDetail(selectedUser.app_user_id);
			await loadUsers();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to update metadata', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function toggleRole(role: string) {
		if (!selectedUser || !canMutate) return;
		const hasRole = selectedUser.roles.includes(role);
		busyAction = true;
		try {
			await controlApi.updateUserRole(appId, envKind, selectedUser.app_user_id, {
				role,
				action: hasRole ? 'revoke' : 'assign'
			});
			addToast(hasRole ? `Revoked role '${role}'` : `Assigned role '${role}'`, 'success');
			await openUserDetail(selectedUser.app_user_id);
			await loadUsers();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to update role', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function toggleSuspend() {
		if (!selectedUser || !canMutate) return;
		busyAction = true;
		try {
			if (selectedUser.status === 'suspended') {
				await controlApi.restoreUser(appId, envKind, selectedUser.app_user_id);
				addToast('User restored in this environment', 'success');
			} else {
				await controlApi.suspendUser(
					appId,
					envKind,
					selectedUser.app_user_id,
					'Suspended via Developer Dashboard'
				);
				addToast('User suspended and active sessions revoked', 'warning');
			}
			await openUserDetail(selectedUser.app_user_id);
			await loadUsers();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Status update failed', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function triggerPasswordReset() {
		if (!selectedUser || !canMutate) return;
		busyAction = true;
		try {
			const res = await controlApi.triggerPasswordReset(appId, envKind, selectedUser.app_user_id);
			addToast(`Password reset triggered for ${res.email}`, 'info');
			await openUserDetail(selectedUser.app_user_id);
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to trigger password reset', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function revokeAllSessions() {
		if (!selectedUser || !canMutate) return;
		busyAction = true;
		try {
			const res = await controlApi.revokeAllUserSessions(appId, envKind, selectedUser.app_user_id);
			addToast(`Revoked ${res.revoked_sessions} active session(s)`, 'info');
			await openUserDetail(selectedUser.app_user_id);
			await loadUsers();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to revoke sessions', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function revokeSingleSession(sessionId: string) {
		if (!selectedUser || !canMutate) return;
		busyAction = true;
		try {
			await controlApi.revokeSession(appId, envKind, sessionId);
			addToast('Session revoked', 'info');
			await openUserDetail(selectedUser.app_user_id);
			await loadUsers();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to revoke session', 'error');
		} finally {
			busyAction = false;
		}
	}
</script>

<section class="panel-stack" aria-label="Users and sessions directory">
	<div class="section-header">
		<div>
			<h1 class="page-title">Users</h1>
			<p class="page-subtitle">
				{total.toLocaleString()} {total === 1 ? 'user' : 'users'} in {envKind} · click any record to inspect sessions, metadata, and roles
			</p>
		</div>
	</div>

	<div class="toolbar">
		<div class="filter-pills" role="tablist" aria-label="User status filter">
			<button type="button" role="tab" aria-selected={statusTab === 'all'} class:active={statusTab === 'all'} onclick={() => { statusTab = 'all'; offset = 0; }}>All users</button>
			<button type="button" role="tab" aria-selected={statusTab === 'active'} class:active={statusTab === 'active'} onclick={() => { statusTab = 'active'; offset = 0; }}>Active</button>
			<button type="button" role="tab" aria-selected={statusTab === 'suspended'} class:active={statusTab === 'suspended'} onclick={() => { statusTab = 'suspended'; offset = 0; }}>Suspended</button>
			<button type="button" role="tab" aria-selected={statusTab === 'unverified'} class:active={statusTab === 'unverified'} onclick={() => { statusTab = 'unverified'; offset = 0; }}>Unverified</button>
		</div>

		<div class="toolbar-right">
			<input
				type="search"
				class="search-input"
				value={searchInput}
				oninput={handleSearchInput}
				placeholder="Search email, sub, app_user_id, external_id..."
				aria-label="Search users"
			/>

			<input
				type="text"
				class="role-input"
				bind:value={roleFilter}
				placeholder="Role filter"
				aria-label="Filter by role"
			/>

			<div class="view-toggle" role="group" aria-label="Directory view mode">
				<button type="button" class:active={viewMode === 'table'} onclick={() => (viewMode = 'table')}>
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
						<line x1="3" y1="6" x2="21" y2="6" />
						<line x1="3" y1="12" x2="21" y2="12" />
						<line x1="3" y1="18" x2="21" y2="18" />
					</svg>
					Table
				</button>
				<button type="button" class:active={viewMode === 'cards'} onclick={() => (viewMode = 'cards')}>
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
						<rect x="3" y="3" width="7" height="7" />
						<rect x="14" y="3" width="7" height="7" />
						<rect x="3" y="14" width="7" height="7" />
						<rect x="14" y="14" width="7" height="7" />
					</svg>
					Cards
				</button>
			</div>
		</div>
	</div>

	{#if loading && users.length === 0}
		<div class="empty-surface">Loading directory...</div>
	{:else if users.length === 0}
		<div class="empty-surface">No users matched the current environment filters.</div>
	{:else if viewMode === 'table'}
		<div class="table-wrap">
			<table class="data-table">
				<thead>
					<tr>
						<th>User</th>
						<th>Roles</th>
						<th>Status</th>
						<th>Sessions</th>
						<th>Version</th>
						<th>Last Active</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each users as u}
						<tr onclick={() => openUserDetail(u.app_user_id)}>
							<td>
								<div class="user-cell">
									<span class="avatar-chip">{getInitials(u.display_name, u.email)}</span>
									<div class="user-text">
										<strong>{u.display_name || 'Unnamed User'}</strong>
										<span>{u.email}</span>
									</div>
								</div>
							</td>
							<td>
								<div class="role-pills">
									{#if u.roles.length === 0}
										<span class="muted">None</span>
									{:else}
										{#each u.roles as r}
											<Badge variant="neutral" size="sm">{r}</Badge>
										{/each}
									{/if}
								</div>
							</td>
							<td>
								<span class="status-inline">
									<span
										class="status-dot"
										class:dot-active={u.status === 'active'}
										class:dot-suspended={u.status === 'suspended'}
									></span>
									{u.status}
								</span>
							</td>
							<td>
								<div class="usage-cell">
									<div class="usage-track">
										<div class="usage-fill" style={`width: ${Math.min(100, u.active_sessions_count * 20)}%`}></div>
									</div>
									<span>{u.active_sessions_count} active</span>
								</div>
							</td>
							<td class="mono">v{u.version}</td>
							<td class="muted">{formatRelativeTimestamp(u.last_active_at ?? u.updated_at)}</td>
							<td class="row-action">
								<button type="button" class="inspect-btn" onclick={(e) => { e.stopPropagation(); openUserDetail(u.app_user_id); }}>
									Inspect
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else}
		<div class="cards-grid">
			{#each users as u}
				<div class="user-card">
					<div class="card-top">
						<div class="user-cell">
							<span class="avatar-chip">{getInitials(u.display_name, u.email)}</span>
							<div class="user-text">
								<strong>{u.display_name || 'Unnamed User'}</strong>
								<span>{u.email}</span>
							</div>
						</div>
						<button type="button" class="inspect-btn" onclick={() => openUserDetail(u.app_user_id)}>Inspect</button>
					</div>
					<div class="card-badges">
						{#each u.roles as r}
							<Badge variant="neutral" size="sm">{r}</Badge>
						{/each}
						<Badge variant={u.status === 'active' ? 'success' : 'danger'} size="sm" dot>{u.status}</Badge>
					</div>
					<div class="card-foot">
						<span>{u.active_sessions_count} active sessions</span>
						<span>v{u.version} · {formatRelativeTimestamp(u.last_active_at ?? u.updated_at)}</span>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<div class="pagination-row">
		<span class="muted">
			Showing {total === 0 ? 0 : offset + 1}–{Math.min(offset + limit, total)} of {total}
		</span>
		<div class="page-btns">
			<Button variant="outline" size="sm" disabled={offset === 0} onclick={() => (offset = Math.max(0, offset - limit))}>
				Previous
			</Button>
			<Button variant="outline" size="sm" disabled={offset + limit >= total} onclick={() => (offset = offset + limit)}>
				Next
			</Button>
		</div>
	</div>
</section>

{#if drawerOpen}
	<div
		class="drawer-backdrop"
		role="dialog"
		aria-modal="true"
		aria-label="User detail inspector"
		tabindex="-1"
		onclick={(e) => {
			if (e.target === e.currentTarget) drawerOpen = false;
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape') drawerOpen = false;
		}}
	>
		<aside class="drawer-panel">
			<div class="drawer-header">
				<div>
					<h2>{selectedUser?.display_name || selectedUser?.email || 'User Detail'}</h2>
					{#if selectedUser}
						<p class="drawer-sub">{selectedUser.email} · v{selectedUser.version}</p>
					{/if}
				</div>
				<button type="button" class="close-btn" onclick={() => (drawerOpen = false)} aria-label="Close user drawer">
					Close
				</button>
			</div>

			{#if detailLoading || !selectedUser}
				<div class="drawer-body">Loading user profile...</div>
			{:else}
				<div class="drawer-body">
					<div class="id-strip">
						<div><span>sub</span><code>{selectedUser.sub}</code></div>
						<div><span>app_user_id</span><code>{selectedUser.app_user_id}</code></div>
						<div><span>Verified</span><strong>{selectedUser.email_verified ? 'Yes' : 'No'}</strong></div>
						<div><span>Status</span><strong>{selectedUser.status}</strong></div>
					</div>

					<div class="drawer-actions">
						<Button variant={selectedUser.status === 'suspended' ? 'primary' : 'danger'} size="sm" disabled={!canMutate || busyAction} onclick={toggleSuspend}>
							{selectedUser.status === 'suspended' ? 'Restore User' : 'Suspend User'}
						</Button>
						<Button variant="outline" size="sm" disabled={!canMutate || busyAction} onclick={triggerPasswordReset}>
							Trigger Password Reset
						</Button>
						<Button variant="outline" size="sm" disabled={!canMutate || busyAction} onclick={revokeAllSessions}>
							Revoke All Sessions
						</Button>
					</div>

					<div class="drawer-section">
						<h3>Environment Roles</h3>
						<div class="role-toggle-grid">
							{#each selectedUser.available_roles as role}
								{@const assigned = selectedUser.roles.includes(role)}
								<button
									type="button"
									class="role-toggle-btn"
									class:assigned
									disabled={!canMutate || busyAction}
									onclick={() => toggleRole(role)}
								>
									{role} {assigned ? '(Active)' : '(Assign)'}
								</button>
							{/each}
						</div>
					</div>

					<div class="drawer-section">
						<h3>Linked OAuth Providers</h3>
						{#if selectedUser.linked_providers.length === 0}
							<p class="muted">No linked social providers.</p>
						{:else}
							<div class="linked-list">
								{#each selectedUser.linked_providers as lp}
									<Badge variant="neutral" size="sm">{lp.provider}: {lp.email ?? 'linked'}</Badge>
								{/each}
							</div>
						{/if}
					</div>

					<div class="drawer-section">
						<h3>Metadata &amp; External ID</h3>
						{#if metaError}
							<p class="error-text" role="alert">{metaError}</p>
						{/if}
						<label class="field">
							<span>external_id</span>
							<input type="text" bind:value={editExternalId} disabled={!canMutate} placeholder="Optional external key" />
						</label>
						<div class="meta-grid">
							<label class="field">
								<span>public_metadata (JSON)</span>
								<textarea rows="5" bind:value={editPublicMeta} disabled={!canMutate}></textarea>
							</label>
							<label class="field">
								<span>private_metadata (JSON)</span>
								<textarea rows="5" bind:value={editPrivateMeta} disabled={!canMutate}></textarea>
							</label>
						</div>
						<Button variant="primary" size="sm" disabled={!canMutate || busyAction} onclick={saveMetadata}>
							Save Metadata
						</Button>
					</div>

					<div class="drawer-section">
						<h3>App Sessions ({selectedUser.sessions.length})</h3>
						{#if selectedUser.sessions.length === 0}
							<p class="muted">No sessions recorded in this environment.</p>
						{:else}
							<div class="mini-list">
								{#each selectedUser.sessions as s}
									<div class="mini-row">
										<div>
											<strong>{s.user_agent ?? 'Unknown client'}</strong>
											<span>{s.ip_address ?? 'No IP'} · Last used {formatRelativeTimestamp(s.last_used_at)}</span>
										</div>
										{#if s.revoked_at}
											<Badge variant="neutral" size="sm">Revoked</Badge>
										{:else}
											<Button variant="danger" size="sm" disabled={!canMutate || busyAction} onclick={() => revokeSingleSession(s.id)}>
												Revoke
											</Button>
										{/if}
									</div>
								{/each}
							</div>
						{/if}
					</div>

					<div class="drawer-section">
						<h3>User Audit Trail ({selectedUser.audit_logs.length})</h3>
						{#if selectedUser.audit_logs.length === 0}
							<p class="muted">No user-scoped audit events.</p>
						{:else}
							<div class="mini-list">
								{#each selectedUser.audit_logs as log}
									<div class="mini-row">
										<div>
											<strong class="mono">{log.event}</strong>
											<span>{log.actor_type} · {formatRelativeTimestamp(log.created_at)}</span>
										</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				</div>
			{/if}
		</aside>
	</div>
{/if}

<style>
	.panel-stack { display: flex; flex-direction: column; gap: 1.25rem; }
	.section-header { display: flex; align-items: flex-end; justify-content: space-between; gap: 1rem; }
	.page-title { font-size: 1.75rem; font-weight: 600; color: #ffffff; margin: 0; }
	.page-subtitle { font-size: 0.85rem; color: #71717a; margin: 0.25rem 0 0; }
	.toolbar { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; flex-wrap: wrap; }
	.filter-pills { display: inline-flex; gap: 0.35rem; background-color: #15171b; border: 1px solid #23262e; border-radius: 8px; padding: 3px; }
	.filter-pills button { background: transparent; border: none; color: #a1a1aa; font-size: 0.8rem; font-weight: 500; padding: 0.4rem 0.75rem; border-radius: 6px; cursor: pointer; }
	.filter-pills button.active { background-color: #23262e; color: #ffffff; }
	.toolbar-right { display: flex; align-items: center; gap: 0.55rem; flex-wrap: wrap; }
	.search-input { width: 280px; background-color: #15171b; border: 1px solid #23262e; border-radius: 8px; padding: 0.48rem 0.75rem; color: #f4f4f5; font-size: 0.82rem; outline: none; }
	.search-input:focus { border-color: #ffffff; }
	.role-input { width: 120px; background-color: #15171b; border: 1px solid #23262e; border-radius: 8px; padding: 0.48rem 0.7rem; color: #f4f4f5; font-size: 0.82rem; outline: none; }
	.view-toggle { display: inline-flex; background-color: #15171b; border: 1px solid #23262e; border-radius: 8px; padding: 3px; }
	.view-toggle button { display: inline-flex; align-items: center; gap: 0.35rem; background: transparent; border: none; color: #a1a1aa; font-size: 0.78rem; padding: 0.35rem 0.65rem; border-radius: 6px; cursor: pointer; }
	.view-toggle button.active { background-color: #23262e; color: #ffffff; }
	.table-wrap { background-color: #15171b; border: 1px solid #23262e; border-radius: 12px; overflow-x: auto; }
	.data-table { width: 100%; border-collapse: collapse; text-align: left; }
	.data-table th { font-size: 0.74rem; font-weight: 500; color: #71717a; padding: 0.8rem 1rem; border-bottom: 1px solid #23262e; }
	.data-table td { padding: 0.85rem 1rem; border-bottom: 1px solid #1f2228; font-size: 0.84rem; color: #d4d4d8; vertical-align: middle; }
	.data-table tbody tr { cursor: pointer; }
	.data-table tbody tr:hover { background-color: #1b1e24; }
	.data-table tbody tr:last-child td { border-bottom: none; }
	.user-cell { display: flex; align-items: center; gap: 0.7rem; }
	.avatar-chip { width: 32px; height: 32px; border-radius: 9999px; background-color: #23262e; color: #ffffff; font-size: 0.72rem; font-weight: 600; display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; }
	.user-text { display: flex; flex-direction: column; min-width: 0; }
	.user-text strong { font-size: 0.85rem; color: #ffffff; font-weight: 500; }
	.user-text span { font-size: 0.76rem; color: #71717a; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.role-pills { display: flex; gap: 0.3rem; flex-wrap: wrap; }
	.status-inline { display: inline-flex; align-items: center; gap: 0.45rem; font-size: 0.8rem; text-transform: capitalize; }
	.status-dot { width: 6px; height: 6px; border-radius: 50%; background-color: #71717a; }
	.dot-active { background-color: #22c55e; }
	.dot-suspended { background-color: #ef4444; }
	.usage-cell { display: flex; align-items: center; gap: 0.55rem; font-size: 0.78rem; color: #a1a1aa; }
	.usage-track { width: 52px; height: 4px; background-color: #23262e; border-radius: 9999px; overflow: hidden; }
	.usage-fill { height: 100%; background-color: #ffffff; }
	.mono { font-family: var(--font-mono); font-size: 0.78rem; }
	.muted { color: #71717a; font-size: 0.8rem; }
	.row-action { text-align: right; }
	.inspect-btn { background: transparent; border: 1px solid #2e333d; color: #d4d4d8; border-radius: 6px; padding: 0.3rem 0.65rem; font-size: 0.76rem; cursor: pointer; }
	.inspect-btn:hover { border-color: #ffffff; color: #ffffff; }
	.cards-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.9rem; }
	.user-card { background-color: #15171b; border: 1px solid #23262e; border-radius: 12px; padding: 1.1rem; display: flex; flex-direction: column; gap: 0.9rem; }
	.card-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 0.5rem; }
	.card-badges { display: flex; gap: 0.35rem; flex-wrap: wrap; }
	.card-foot { display: flex; justify-content: space-between; font-size: 0.74rem; color: #71717a; padding-top: 0.65rem; border-top: 1px solid #1f2228; }
	.pagination-row { display: flex; align-items: center; justify-content: space-between; }
	.page-btns { display: flex; gap: 0.5rem; }
	.empty-surface { padding: 2.5rem; text-align: center; color: #71717a; background-color: #15171b; border: 1px solid #23262e; border-radius: 12px; font-size: 0.85rem; }
	.drawer-backdrop { position: fixed; inset: 0; background-color: rgba(0, 0, 0, 0.72); z-index: 900; display: flex; justify-content: flex-end; }
	.drawer-panel { width: min(640px, 100vw); height: 100vh; background-color: #111215; border-left: 1px solid #23262e; display: flex; flex-direction: column; }
	.drawer-header { padding: 1.25rem 1.5rem; border-bottom: 1px solid #23262e; display: flex; align-items: center; justify-content: space-between; }
	.drawer-header h2 { margin: 0; font-size: 1.1rem; color: #ffffff; }
	.drawer-sub { margin: 0.2rem 0 0; font-size: 0.78rem; color: #71717a; }
	.close-btn { background: transparent; border: 1px solid #2e333d; color: #d4d4d8; padding: 0.35rem 0.7rem; border-radius: 6px; font-size: 0.78rem; cursor: pointer; }
	.drawer-body { padding: 1.35rem 1.5rem; overflow-y: auto; display: flex; flex-direction: column; gap: 1.35rem; }
	.id-strip { display: grid; grid-template-columns: 1fr 1fr; gap: 0.65rem; background-color: #15171b; border: 1px solid #23262e; border-radius: 8px; padding: 0.85rem; }
	.id-strip div { display: flex; flex-direction: column; gap: 0.15rem; min-width: 0; }
	.id-strip span { font-size: 0.7rem; color: #71717a; text-transform: uppercase; }
	.id-strip code { font-size: 0.74rem; color: #e4e4e7; overflow: hidden; text-overflow: ellipsis; }
	.id-strip strong { font-size: 0.82rem; color: #ffffff; }
	.drawer-actions { display: flex; gap: 0.55rem; flex-wrap: wrap; }
	.drawer-section { display: flex; flex-direction: column; gap: 0.65rem; }
	.drawer-section h3 { font-size: 0.82rem; text-transform: uppercase; letter-spacing: 0.05em; color: #a1a1aa; margin: 0; }
	.role-toggle-grid { display: flex; gap: 0.45rem; flex-wrap: wrap; }
	.role-toggle-btn { background-color: #15171b; border: 1px solid #23262e; color: #a1a1aa; padding: 0.35rem 0.65rem; border-radius: 6px; font-size: 0.76rem; cursor: pointer; }
	.role-toggle-btn.assigned { background-color: #ffffff; color: #0b0c0e; border-color: #ffffff; font-weight: 600; }
	.linked-list { display: flex; gap: 0.45rem; flex-wrap: wrap; }
	.field { display: flex; flex-direction: column; gap: 0.3rem; }
	.field span { font-size: 0.74rem; color: #a1a1aa; }
	.field input, .field textarea { background-color: #0b0c0e; border: 1px solid #23262e; border-radius: 6px; padding: 0.55rem 0.7rem; color: #f4f4f5; font-family: var(--font-mono); font-size: 0.76rem; outline: none; }
	.meta-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
	.mini-list { display: flex; flex-direction: column; border-top: 1px solid #23262e; }
	.mini-row { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; padding: 0.65rem 0; border-bottom: 1px solid #23262e; }
	.mini-row div { display: flex; flex-direction: column; gap: 0.15rem; min-width: 0; }
	.mini-row strong { font-size: 0.8rem; color: #f4f4f5; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.mini-row span { font-size: 0.74rem; color: #71717a; }
	@media (max-width: 1080px) {
		.cards-grid, .meta-grid, .id-strip { grid-template-columns: 1fr; }
		.search-input { width: 100%; }
	}
</style>

