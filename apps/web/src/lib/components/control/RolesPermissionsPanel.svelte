<script lang="ts">
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import type {
		EnvironmentKind,
		EnvironmentPermissionDto,
		EnvironmentRoleDto
	} from '$lib/api/types';

	interface Props {
		appId: string;
		envKind: EnvironmentKind;
		canMutate: boolean;
	}

	let { appId, envKind, canMutate }: Props = $props();

	let permissions = $state<EnvironmentPermissionDto[]>([]);
	let roles = $state<EnvironmentRoleDto[]>([]);
	let loading = $state(false);
	let scopeFilter = $state<'all' | 'application' | 'organization'>('all');
	let busy = $state(false);

	let newPermSlug = $state('');
	let newPermDesc = $state('');

	let roleModalOpen = $state(false);
	let editingRole = $state<EnvironmentRoleDto | null>(null);
	let roleSlug = $state('');
	let roleName = $state('');
	let roleScope = $state<'application' | 'organization'>('organization');
	let roleDesc = $state('');
	let selectedPerms = $state<string[]>([]);
	let roleFormError = $state('');

	const filteredRoles = $derived(
		scopeFilter === 'all' ? roles : roles.filter((r) => r.scope === scopeFilter)
	);

	async function loadCatalog() {
		loading = true;
		try {
			const [permList, roleList] = await Promise.all([
				controlApi.listEnvironmentPermissions(appId, envKind),
				controlApi.listEnvironmentRoles(appId, envKind)
			]);
			permissions = permList;
			roles = roleList;
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load RBAC catalog', 'error');
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const _app = appId;
		const _env = envKind;
		void _app;
		void _env;
		loadCatalog();
	});

	async function handleCreatePermission(e: SubmitEvent) {
		e.preventDefault();
		if (!canMutate || !newPermSlug.trim()) return;
		busy = true;
		try {
			await controlApi.createEnvironmentPermission(appId, envKind, {
				slug: newPermSlug.trim().toLowerCase(),
				description: newPermDesc.trim() || undefined
			});
			newPermSlug = '';
			newPermDesc = '';
			addToast('Permission created', 'success');
			await loadCatalog();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to create permission', 'error');
		} finally {
			busy = false;
		}
	}

	async function handleDeletePermission(permissionId: string) {
		if (!canMutate) return;
		busy = true;
		try {
			await controlApi.deleteEnvironmentPermission(appId, envKind, permissionId);
			addToast('Permission removed', 'info');
			await loadCatalog();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to delete permission', 'error');
		} finally {
			busy = false;
		}
	}

	function openCreateRoleModal() {
		editingRole = null;
		roleSlug = '';
		roleName = '';
		roleScope = 'organization';
		roleDesc = '';
		selectedPerms = [];
		roleFormError = '';
		roleModalOpen = true;
	}

	function openEditRoleModal(role: EnvironmentRoleDto) {
		editingRole = role;
		roleSlug = role.slug;
		roleName = role.name;
		roleScope = role.scope === 'application' ? 'application' : 'organization';
		roleDesc = role.description;
		selectedPerms = [...role.permissions];
		roleFormError = '';
		roleModalOpen = true;
	}

	function togglePermissionSelection(slug: string) {
		if (selectedPerms.includes(slug)) {
			selectedPerms = selectedPerms.filter((p) => p !== slug);
		} else {
			selectedPerms = [...selectedPerms, slug];
		}
	}

	async function handleSaveRole() {
		if (!canMutate) return;
		roleFormError = '';
		busy = true;
		try {
			if (editingRole) {
				await controlApi.updateEnvironmentRole(appId, envKind, editingRole.id, {
					name: roleName.trim(),
					description: roleDesc.trim(),
					permissions: selectedPerms
				});
				addToast('Role updated', 'success');
			} else {
				await controlApi.createEnvironmentRole(appId, envKind, {
					slug: roleSlug.trim().toLowerCase(),
					name: roleName.trim(),
					scope: roleScope,
					description: roleDesc.trim() || undefined,
					permissions: selectedPerms
				});
				addToast('Custom role created', 'success');
			}
			roleModalOpen = false;
			await loadCatalog();
		} catch (err) {
			roleFormError = err instanceof Error ? err.message : 'Failed to save role';
		} finally {
			busy = false;
		}
	}

	async function handleDeleteRole(role: EnvironmentRoleDto) {
		if (!canMutate || role.is_system) return;
		busy = true;
		try {
			await controlApi.deleteEnvironmentRole(appId, envKind, role.id);
			addToast(`Role '${role.slug}' deleted`, 'info');
			await loadCatalog();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to delete role', 'error');
		} finally {
			busy = false;
		}
	}
</script>

<section class="panel-stack" aria-label="Custom roles and permissions">
	<div class="panel-header">
		<div>
			<h1>Roles &amp; Permissions</h1>
			<p>Define custom environment permissions and assign them to application-wide or organization-scoped roles.</p>
		</div>
		{#if canMutate}
			<Button variant="primary" size="sm" onclick={openCreateRoleModal}>
				Create Custom Role
			</Button>
		{/if}
	</div>

	<div class="split-grid">
		<div class="card">
			<div class="card-head">
				<div>
					<h2>Roles ({filteredRoles.length})</h2>
					<p>System roles are protected; custom roles can be edited or removed anytime.</p>
				</div>
				<div class="filter-tabs" role="tablist" aria-label="Role scope filter">
					<button type="button" class:active={scopeFilter === 'all'} onclick={() => (scopeFilter = 'all')}>
						All
					</button>
					<button
						type="button"
						class:active={scopeFilter === 'organization'}
						onclick={() => (scopeFilter = 'organization')}
					>
						Organization
					</button>
					<button
						type="button"
						class:active={scopeFilter === 'application'}
						onclick={() => (scopeFilter = 'application')}
					>
						Application
					</button>
				</div>
			</div>

			{#if loading}
				<p class="muted">Loading roles...</p>
			{:else if filteredRoles.length === 0}
				<p class="muted">No roles match the selected scope.</p>
			{:else}
				<div class="role-list">
					{#each filteredRoles as role (role.id)}
						<article class="role-row">
							<div class="role-main">
								<div class="role-title">
									<strong>{role.name}</strong>
									<code>{role.slug}</code>
									<Badge variant="neutral" size="sm">{role.scope}</Badge>
									{#if role.is_system}
										<Badge variant="warning" size="sm">System</Badge>
									{/if}
								</div>
								{#if role.description}
									<p class="role-desc">{role.description}</p>
								{/if}
								<div class="perm-tags">
									{#if role.permissions.length === 0}
										<span class="muted">No explicit permissions attached</span>
									{:else}
										{#each role.permissions as perm (perm)}
											<code class="perm-chip">{perm}</code>
										{/each}
									{/if}
								</div>
							</div>
							<div class="role-actions">
								{#if canMutate && !role.is_system}
									<Button variant="outline" size="sm" onclick={() => openEditRoleModal(role)}>
										Edit
									</Button>
									<Button
										variant="danger"
										size="sm"
										disabled={busy}
										onclick={() => handleDeleteRole(role)}
									>
										Delete
									</Button>
								{/if}
							</div>
						</article>
					{/each}
				</div>
			{/if}
		</div>

		<div class="card">
			<div class="card-head">
				<div>
					<h2>Permissions ({permissions.length})</h2>
					<p>Granular capability slugs embedded into tenant ES256 access tokens.</p>
				</div>
			</div>

			{#if canMutate}
				<form class="perm-form" onsubmit={handleCreatePermission}>
					<input
						type="text"
						required
						placeholder="billing:invoices:write"
						bind:value={newPermSlug}
						disabled={busy}
						aria-label="Permission slug"
					/>
					<input
						type="text"
						placeholder="Description (optional)"
						bind:value={newPermDesc}
						disabled={busy}
						aria-label="Permission description"
					/>
					<Button variant="primary" size="sm" type="submit" disabled={busy}>Add</Button>
				</form>
			{/if}

			<div class="perm-list">
				{#each permissions as perm (perm.id)}
					<div class="perm-row">
						<div>
							<code>{perm.slug}</code>
							{#if perm.description}
								<span class="perm-desc">{perm.description}</span>
							{/if}
						</div>
						{#if canMutate}
							<Button
								variant="outline"
								size="sm"
								disabled={busy}
								onclick={() => handleDeletePermission(perm.id)}
							>
								Remove
							</Button>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	</div>
</section>

<Modal
	open={roleModalOpen}
	title={editingRole ? `Edit Role: ${editingRole.slug}` : 'Create Custom Role'}
	description="Attach environment permission slugs to define granular access policies."
	confirmText={editingRole ? 'Save Role' : 'Create Role'}
	loading={busy}
	onconfirm={handleSaveRole}
	oncancel={() => (roleModalOpen = false)}
>
	<div class="modal-form">
		{#if roleFormError}
			<p class="error-text" role="alert">{roleFormError}</p>
		{/if}
		{#if !editingRole}
			<label class="field">
				<span>Role Slug</span>
				<input type="text" required bind:value={roleSlug} placeholder="org:billing_admin" />
			</label>
			<label class="field">
				<span>Scope</span>
				<select bind:value={roleScope}>
					<option value="organization">organization</option>
					<option value="application">application</option>
				</select>
			</label>
		{/if}
		<label class="field">
			<span>Display Name</span>
			<input type="text" required bind:value={roleName} placeholder="Billing Admin" />
		</label>
		<label class="field">
			<span>Description</span>
			<input type="text" bind:value={roleDesc} placeholder="Can manage invoices and subscriptions" />
		</label>

		<div class="field">
			<span>Attached Permissions ({selectedPerms.length})</span>
			<div class="perm-check-grid">
				{#each permissions as perm (perm.id)}
					<label class="perm-check-item">
						<input
							type="checkbox"
							checked={selectedPerms.includes(perm.slug)}
							onchange={() => togglePermissionSelection(perm.slug)}
						/>
						<code>{perm.slug}</code>
					</label>
				{/each}
			</div>
		</div>
	</div>
</Modal>

<style>
	.panel-stack {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.panel-header h1 {
		margin: 0;
		font-size: 1.35rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.panel-header p {
		margin: 0.25rem 0 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.split-grid {
		display: grid;
		grid-template-columns: 1.4fr 1fr;
		gap: 1rem;
		align-items: start;
	}

	@media (max-width: 980px) {
		.split-grid {
			grid-template-columns: 1fr;
		}
	}

	.card {
		background: var(--bg-surface);
		border: 1px solid var(--border-subtle);
		border-radius: 8px;
		padding: 1.1rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.card-head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.75rem;
		flex-wrap: wrap;
	}

	.card-head h2 {
		margin: 0;
		font-size: 1rem;
		color: var(--text-primary);
	}

	.card-head p {
		margin: 0.2rem 0 0;
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.filter-tabs {
		display: inline-flex;
		background: var(--bg-canvas);
		border: 1px solid var(--border-subtle);
		border-radius: 6px;
		padding: 2px;
	}

	.filter-tabs button {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		padding: 0.28rem 0.6rem;
		font-size: 0.76rem;
		border-radius: 4px;
		cursor: pointer;
	}

	.filter-tabs button.active {
		background: var(--bg-elevated);
		color: var(--text-primary);
	}

	.role-list,
	.perm-list {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}

	.role-row,
	.perm-row {
		background: var(--bg-canvas);
		border: 1px solid var(--border-subtle);
		border-radius: 6px;
		padding: 0.75rem 0.85rem;
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.75rem;
	}

	.role-main {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}

	.role-title {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.role-title strong {
		font-size: 0.9rem;
		color: var(--text-primary);
	}

	.role-desc,
	.perm-desc {
		display: block;
		margin: 0;
		font-size: 0.78rem;
		color: var(--text-muted);
	}

	.perm-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
	}

	.perm-chip {
		font-size: 0.72rem;
		padding: 0.16rem 0.45rem;
		background: var(--bg-surface);
		border: 1px solid var(--border-subtle);
		border-radius: 4px;
		color: var(--text-secondary);
	}

	.role-actions {
		display: flex;
		gap: 0.45rem;
		flex-shrink: 0;
	}

	.perm-form {
		display: grid;
		grid-template-columns: 1fr 1fr auto;
		gap: 0.5rem;
	}

	.perm-form input,
	.field input,
	.field select {
		background: var(--bg-canvas);
		border: 1px solid var(--border-subtle);
		border-radius: 6px;
		padding: 0.45rem 0.65rem;
		color: var(--text-primary);
		font-size: 0.84rem;
	}

	.modal-form {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		font-size: 0.8rem;
		color: var(--text-secondary);
	}

	.perm-check-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
		gap: 0.45rem;
		max-height: 220px;
		overflow-y: auto;
		padding: 0.5rem;
		background: var(--bg-canvas);
		border: 1px solid var(--border-subtle);
		border-radius: 6px;
	}

	.perm-check-item {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-size: 0.78rem;
		color: var(--text-primary);
		cursor: pointer;
	}

	.error-text {
		color: var(--danger);
		font-size: 0.82rem;
		margin: 0;
	}

	.muted {
		color: var(--text-muted);
		font-size: 0.82rem;
		margin: 0;
	}
</style>
