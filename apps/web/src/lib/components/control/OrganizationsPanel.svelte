<script lang="ts">
	import SectionHeader from './SectionHeader.svelte';
	import Dropdown from '$lib/components/Dropdown.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import CopyButton from '$lib/components/CopyButton.svelte';
	import DeleteButton from '$lib/components/DeleteButton.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import type {
		EnvironmentKind,
		EnvironmentRoleDto,
		OrganizationDto,
		OrganizationInvitationDto,
		OrganizationMemberDto
	} from '$lib/api/types';
	import { formatRelativeTimestamp } from '$lib/control/permissions';
	import { validateSlug } from '$lib/control/validation';

	interface Props {
		appId: string;
		envKind: EnvironmentKind;
		canMutate: boolean;
	}

	let { appId, envKind, canMutate }: Props = $props();

	let organizations = $state<OrganizationDto[]>([]);
	let orgRoles = $state<EnvironmentRoleDto[]>([]);
	let loading = $state(false);
	let searchInput = $state('');
	let viewMode = $state<'table' | 'cards'>('table');
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	let createModalOpen = $state(false);
	let newOrgName = $state('');
	let newOrgSlug = $state('');
	let newOrgLogo = $state('');
	let createError = $state('');
	let busyAction = $state(false);

	let selectedOrg = $state<OrganizationDto | null>(null);
	let drawerOpen = $state(false);
	let detailLoading = $state(false);
	let members = $state<OrganizationMemberDto[]>([]);
	let invitations = $state<OrganizationInvitationDto[]>([]);

	let editName = $state('');
	let editSlug = $state('');
	let editMetadata = $state('{}');
	let metaError = $state('');

	let inviteEmail = $state('');
	let inviteRole = $state('org:member');
	let inviteDays = $state(7);
	let lastIssuedToken = $state<string | null>(null);

	async function loadOrganizations() {
		loading = true;
		try {
			const [page, roles] = await Promise.all([
				controlApi.listOrganizations(appId, envKind, {
					q: searchInput.trim() || undefined,
					limit: 50
				}),
				controlApi.listEnvironmentRoles(appId, envKind)
			]);
			organizations = page.data;
			orgRoles = roles.filter((r) => r.scope === 'organization');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load organizations', 'error');
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const _app = appId;
		const _env = envKind;
		void _app;
		void _env;
		loadOrganizations();
	});

	function handleSearchInput(e: Event) {
		searchInput = (e.currentTarget as HTMLInputElement).value;
		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => {
			loadOrganizations();
		}, 250);
	}

	async function handleCreateOrg() {
		createError = '';
		const slugErr = validateSlug(newOrgSlug);
		if (slugErr) {
			createError = slugErr;
			return;
		}
		if (newOrgName.trim().length < 2) {
			createError = 'Organization name must be at least 2 characters.';
			return;
		}
		busyAction = true;
		try {
			await controlApi.createOrganization(appId, envKind, {
				name: newOrgName.trim(),
				slug: newOrgSlug.trim().toLowerCase(),
				logo_url: newOrgLogo.trim() || undefined
			});
			createModalOpen = false;
			newOrgName = '';
			newOrgSlug = '';
			newOrgLogo = '';
			addToast('Organization created', 'success');
			await loadOrganizations();
		} catch (err) {
			createError = err instanceof Error ? err.message : 'Failed to create organization';
		} finally {
			busyAction = false;
		}
	}

	async function openOrgDetail(org: OrganizationDto) {
		selectedOrg = org;
		editName = org.name;
		editSlug = org.slug;
		editMetadata = JSON.stringify(org.metadata ?? {}, null, 2);
		metaError = '';
		lastIssuedToken = null;
		drawerOpen = true;
		detailLoading = true;
		try {
			const [memberList, invList] = await Promise.all([
				controlApi.listOrganizationMembers(appId, envKind, org.id),
				controlApi.listOrganizationInvitations(appId, envKind, org.id)
			]);
			members = memberList;
			invitations = invList;
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load organization details', 'error');
		} finally {
			detailLoading = false;
		}
	}

	async function saveOrgSettings() {
		if (!selectedOrg || !canMutate) return;
		metaError = '';
		let parsedMeta: Record<string, unknown>;
		try {
			parsedMeta = JSON.parse(editMetadata || '{}') as Record<string, unknown>;
		} catch {
			metaError = 'Metadata must be a valid JSON object.';
			return;
		}
		busyAction = true;
		try {
			const updated = await controlApi.updateOrganization(appId, envKind, selectedOrg.id, {
				name: editName.trim(),
				slug: editSlug.trim().toLowerCase(),
				metadata: parsedMeta
			});
			selectedOrg = updated;
			addToast('Organization settings updated', 'success');
			await loadOrganizations();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to update organization', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function toggleOrgStatus() {
		if (!selectedOrg || !canMutate) return;
		const nextStatus = selectedOrg.status === 'suspended' ? 'active' : 'suspended';
		busyAction = true;
		try {
			const updated = await controlApi.updateOrganization(appId, envKind, selectedOrg.id, {
				status: nextStatus
			});
			selectedOrg = updated;
			addToast(
				nextStatus === 'suspended' ? 'Organization suspended' : 'Organization restored',
				'success'
			);
			await loadOrganizations();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Status update failed', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function handleDeleteOrg() {
		if (!selectedOrg || !canMutate) return;
		busyAction = true;
		try {
			await controlApi.deleteOrganization(appId, envKind, selectedOrg.id);
			drawerOpen = false;
			selectedOrg = null;
			addToast('Organization archived and deleted', 'info');
			await loadOrganizations();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to delete organization', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function handleMemberRoleChange(memberId: string, roleSlug: string) {
		if (!selectedOrg || !canMutate) return;
		busyAction = true;
		try {
			await controlApi.updateOrganizationMemberRole(appId, envKind, selectedOrg.id, memberId, roleSlug);
			members = await controlApi.listOrganizationMembers(appId, envKind, selectedOrg.id);
			addToast(`Updated member role to ${roleSlug}`, 'success');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to update member role', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function handleRemoveMember(memberId: string) {
		if (!selectedOrg || !canMutate) return;
		busyAction = true;
		try {
			await controlApi.removeOrganizationMember(appId, envKind, selectedOrg.id, memberId);
			members = await controlApi.listOrganizationMembers(appId, envKind, selectedOrg.id);
			addToast('Member removed from organization', 'info');
			await loadOrganizations();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Cannot remove member', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function handleCreateInvitation(e: SubmitEvent) {
		e.preventDefault();
		if (!selectedOrg || !canMutate || !inviteEmail.trim()) return;
		busyAction = true;
		try {
			const created = await controlApi.createOrganizationInvitation(appId, envKind, selectedOrg.id, {
				email: inviteEmail.trim(),
				role_slug: inviteRole,
				expires_in_days: inviteDays
			});
			inviteEmail = '';
			lastIssuedToken = created.invitation_token;
			invitations = await controlApi.listOrganizationInvitations(appId, envKind, selectedOrg.id);
			addToast('Invitation created; copy the one-time token below', 'success');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to create invitation', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function handleRevokeInvitation(invitationId: string) {
		if (!selectedOrg || !canMutate) return;
		busyAction = true;
		try {
			await controlApi.revokeOrganizationInvitation(appId, envKind, selectedOrg.id, invitationId);
			invitations = await controlApi.listOrganizationInvitations(appId, envKind, selectedOrg.id);
			addToast('Invitation revoked', 'info');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to revoke invitation', 'error');
		} finally {
			busyAction = false;
		}
	}

	async function copyToken(value: string) {
		try {
			await navigator.clipboard.writeText(value);
			addToast('Invitation token copied to clipboard', 'success');
		} catch {
			addToast('Unable to access clipboard', 'error');
		}
	}
</script>

<section class="grid min-w-0 grid-cols-1 gap-5 sm:gap-6" aria-label="Organizations directory">
	<SectionHeader title="Organizations" description="Manage B2B workspaces, member roles, and replay-proof invitations in this environment.">
		<div class="flex min-w-0 flex-wrap items-center gap-2">
			<div class="view-toggle" role="group" aria-label="Layout toggle">
				<button type="button" class:active={viewMode === 'table'} onclick={() => (viewMode = 'table')}>
					Table
				</button>
				<button type="button" class:active={viewMode === 'cards'} onclick={() => (viewMode = 'cards')}>
					Cards
				</button>
			</div>
			{#if canMutate}
				<Button
					size="sm"
					onclick={() => {
						createError = '';
						createModalOpen = true;
					}}
				>
					New Organization
				</Button>
			{/if}
		</div>
	</SectionHeader>

	<div class="toolbar">
		<input
			type="search"
			class="search-input"
			placeholder="Search organizations by name or slug..."
			value={searchInput}
			oninput={handleSearchInput}
			aria-label="Search organizations"
		/>
		<span class="count-pill">{organizations.length} active</span>
	</div>

	{#if loading}
		<div class="empty-box">Loading organizations...</div>
	{:else if organizations.length === 0}
		<div class="empty-box">
			<strong>No organizations yet</strong>
			<p>Create a workspace organization to enable B2B multi-tenancy and org-scoped JWT claims.</p>
		</div>
	{:else if viewMode === 'table'}
		<div class="table-wrap">
			<table class="data-table">
				<thead>
					<tr>
						<th>Organization</th>
						<th>Slug</th>
						<th>Status</th>
						<th>Members</th>
						<th>Version</th>
						<th>Updated</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each organizations as org (org.id)}
						<tr>
							<td><strong>{org.name}</strong></td>
							<td><code>{org.slug}</code></td>
							<td>
								<Badge variant={org.status === 'active' ? 'success' : 'warning'} size="sm" dot>
									{org.status}
								</Badge>
							</td>
							<td>{org.member_count}</td>
							<td><code>v{org.version}</code></td>
							<td>{formatRelativeTimestamp(org.updated_at)}</td>
							<td class="cell-right">
								<Button size="sm" onclick={() => openOrgDetail(org)}>
									Manage
								</Button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else}
		<div class="cards-grid">
			{#each organizations as org (org.id)}
				<article class="org-card task-surface">
					<div class="org-card-top">
						<div>
							<h3>{org.name}</h3>
							<code>{org.slug}</code>
						</div>
						<Badge variant={org.status === 'active' ? 'success' : 'warning'} size="sm" dot>
							{org.status}
						</Badge>
					</div>
					<div class="org-card-meta">
						<span>{org.member_count} members</span>
						<span>v{org.version}</span>
						<span>{formatRelativeTimestamp(org.updated_at)}</span>
					</div>
					<Button size="sm" onclick={() => openOrgDetail(org)}>
						Inspect &amp; Invite
					</Button>
				</article>
			{/each}
		</div>
	{/if}
</section>

<Modal
	open={createModalOpen}
	title="Create Organization"
	description="Provision a B2B tenant workspace isolated to this environment."
	confirmText="Create Organization"
	loading={busyAction}
	onconfirm={handleCreateOrg}
	oncancel={() => (createModalOpen = false)}
>
	<div class="modal-form">
		{#if createError}
			<p class="error-text" role="alert">{createError}</p>
		{/if}
		<label class="field">
			<span>Organization Name</span>
			<input type="text" required bind:value={newOrgName} placeholder="Acme Corporation" />
		</label>
		<label class="field">
			<span>Slug</span>
			<input type="text" required bind:value={newOrgSlug} placeholder="acme-corp" />
		</label>
		<label class="field">
			<span>Logo URL (optional)</span>
			<input type="url" bind:value={newOrgLogo} placeholder="https://acme.example/logo.png" />
		</label>
	</div>
</Modal>

{#if drawerOpen && selectedOrg}
	<div
		class="drawer-backdrop"
		role="dialog"
		aria-modal="true"
		aria-label="Organization inspector"
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
					<h2>{selectedOrg.name}</h2>
					<p class="drawer-sub">{selectedOrg.slug} · v{selectedOrg.version} · {selectedOrg.status}</p>
				</div>
				<button type="button" class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold" onclick={() => (drawerOpen = false)}>Close</button>
			</div>

			<div class="drawer-body">
				<div class="drawer-actions">
					<Button
						size="sm"
						disabled={!canMutate || busyAction}
						onclick={toggleOrgStatus}
					>
						{selectedOrg.status === 'suspended' ? 'Restore Organization' : 'Suspend Organization'}
					</Button>
					<DeleteButton
						label="Delete Organization"
						disabled={!canMutate || busyAction}
						onclick={handleDeleteOrg}
					/>
				</div>

				<div class="drawer-section">
					<h3>Profile &amp; Metadata</h3>
					{#if metaError}
						<p class="error-text" role="alert">{metaError}</p>
					{/if}
					<div class="two-col">
						<label class="field">
							<span>Name</span>
							<input type="text" bind:value={editName} disabled={!canMutate} />
						</label>
						<label class="field">
							<span>Slug</span>
							<input type="text" bind:value={editSlug} disabled={!canMutate} />
						</label>
					</div>
					<label class="field">
						<span>Metadata (JSON)</span>
						<textarea rows="4" bind:value={editMetadata} disabled={!canMutate}></textarea>
					</label>
					<Button
						size="sm"
						disabled={!canMutate || busyAction}
						onclick={saveOrgSettings}
					>
						Save Changes
					</Button>
				</div>

				<div class="drawer-section">
					<h3>Members ({members.length})</h3>
					{#if detailLoading}
						<p class="muted">Loading members...</p>
					{:else if members.length === 0}
						<p class="muted">No active members in this organization yet.</p>
					{:else}
						<div class="mini-list">
							{#each members as member (member.id)}
								<div class="mini-row">
									<div>
										<strong>{member.display_name || member.email}</strong>
										<span class="row-sub">{member.email}</span>
									</div>
									<div class="row-controls">
										<Dropdown
											label={`Role for ${member.email}`}
											value={member.roles[0] ?? 'org:member'}
											options={orgRoles.map((role) => ({ value: role.slug, label: role.slug }))}
											disabled={!canMutate || busyAction}
											onchange={(role) => handleMemberRoleChange(member.id, role)}
										/>
										<DeleteButton
											label="Remove member"
											disabled={!canMutate || busyAction}
											onclick={() => handleRemoveMember(member.id)}
										/>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</div>

				<div class="drawer-section">
					<h3>Invite Member</h3>
					<form class="invite-form grid min-w-0 grid-cols-1 items-center gap-2 sm:grid-cols-[minmax(0,1fr)_auto_4.5rem_auto] [&>button]:h-9 [&>input]:h-9 [&>input]:min-w-0" onsubmit={handleCreateInvitation}>
						<input
							type="email"
							required
							placeholder="member@company.com"
							bind:value={inviteEmail}
							disabled={!canMutate || busyAction}
						/>
						<Dropdown
							label="Invite role"
							bind:value={inviteRole}
							options={orgRoles.map((role) => ({ value: role.slug, label: role.slug }))}
							disabled={!canMutate || busyAction}
						/>
						<input
							type="number"
							min="1"
							max="30"
							bind:value={inviteDays}
							disabled={!canMutate || busyAction}
							aria-label="Expiry days"
						/>
						<Button size="sm" type="submit" disabled={!canMutate || busyAction}>
							Issue Invite
						</Button>
					</form>

					{#if lastIssuedToken}
						<div class="token-box" role="status">
							<div>
								<strong>One-time Invitation Token</strong>
								<code>{lastIssuedToken}</code>
							</div>
							<CopyButton label="Copy invitation token" onclick={() => copyToken(lastIssuedToken!)} />
						</div>
					{/if}
				</div>

				<div class="drawer-section">
					<h3>Invitations ({invitations.length})</h3>
					{#if invitations.length === 0}
						<p class="muted">No invitations recorded for this organization.</p>
					{:else}
						<div class="mini-list">
							{#each invitations as inv (inv.id)}
								<div class="mini-row">
									<div>
										<strong>{inv.email}</strong>
										<span class="row-sub">
											{inv.role_slug ?? 'org:member'} · {inv.status} · expires {formatRelativeTimestamp(inv.expires_at)}
										</span>
									</div>
									{#if inv.status === 'pending'}
										<DeleteButton
											label="Revoke invitation"
											disabled={!canMutate || busyAction}
											onclick={() => handleRevokeInvitation(inv.id)}
										/>
									{:else}
										<Badge variant="neutral" size="sm">{inv.status}</Badge>
									{/if}
								</div>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		</aside>
	</div>
{/if}

<style>
	.view-toggle {
		display: inline-flex;
		background: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 6px;
		padding: 2px;
	}

	.view-toggle button {
		background: transparent;
		border: none;
		color: var(--color-brand-text-secondary);
		padding: 0.3rem 0.65rem;
		font-size: 0.78rem;
		border-radius: 4px;
		cursor: pointer;
	}

	.view-toggle button.active {
		background: var(--color-brand-surface-raised);
		color: var(--color-brand-text-primary);
	}

	.toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		flex-wrap: wrap;
	}

	.search-input {
		flex: 1;
		min-width: 220px;
		max-width: 380px;
		background: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 6px;
		padding: 0.5rem 0.75rem;
		color: var(--color-brand-text-primary);
		font-size: 0.85rem;
	}

	.count-pill {
		font-size: 0.78rem;
		color: var(--color-brand-text-muted);
		border: 1px solid var(--color-brand-border);
		background: var(--color-brand-surface-card);
		padding: 0.3rem 0.6rem;
		border-radius: 999px;
	}

	.empty-box {
		background: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 8px;
		padding: 2.25rem 1.5rem;
		text-align: center;
		color: var(--color-brand-text-secondary);
	}

	.empty-box strong {
		display: block;
		color: var(--color-brand-text-primary);
		margin-bottom: 0.35rem;
	}

	.table-wrap {
		background: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 8px;
		overflow-x: auto;
	}

	.data-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}

	.data-table th,
	.data-table td {
		padding: 0.75rem 1rem;
		text-align: left;
		border-bottom: 1px solid var(--color-brand-border);
	}

	.data-table th {
		font-size: 0.74rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--color-brand-text-muted);
		font-weight: 500;
	}

	.data-table tr:last-child td {
		border-bottom: none;
	}

	.cell-right {
		text-align: right;
	}

	.cards-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 1rem;
	}

	.org-card {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}

	.org-card-top {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.org-card-top h3 {
		margin: 0 0 0.2rem;
		font-size: 0.95rem;
		color: var(--color-brand-text-primary);
	}

	.org-card-meta {
		display: flex;
		gap: 0.75rem;
		font-size: 0.78rem;
		color: var(--color-brand-text-muted);
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
		color: var(--color-brand-text-secondary);
	}

	.field input,
	.field textarea,
	.invite-form input {
		background: var(--color-brand-surface);
		border: 1px solid var(--color-brand-border);
		border-radius: 6px;
		padding: 0.45rem 0.65rem;
		color: var(--color-brand-text-primary);
		font-size: 0.84rem;
		font-family: inherit;
	}

	.error-text {
		color: var(--color-brand-danger-text);
		font-size: 0.82rem;
		margin: 0;
	}

	.muted {
		color: var(--color-brand-text-muted);
		font-size: 0.84rem;
		margin: 0;
	}

	.drawer-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.65);
		display: flex;
		justify-content: flex-end;
		z-index: 60;
	}

	.drawer-panel {
		width: min(620px, 100%);
		background: var(--color-brand-surface-card);
		border-left: 1px solid var(--color-brand-border);
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
	}

	.drawer-header {
		padding: 1.1rem 1.25rem;
		border-bottom: 1px solid var(--color-brand-border);
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.drawer-header h2 {
		margin: 0;
		font-size: 1.08rem;
		color: var(--color-brand-text-primary);
	}

	.drawer-sub {
		margin: 0.2rem 0 0;
		font-size: 0.78rem;
		color: var(--color-brand-text-muted);
	}


	.drawer-body {
		padding: 1.25rem;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.drawer-actions {
		display: flex;
		gap: 0.6rem;
		flex-wrap: wrap;
	}

	.drawer-section {
		background: var(--color-brand-surface-card);
		border: 1px solid var(--color-brand-border);
		border-radius: 8px;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.drawer-section h3 {
		margin: 0;
		font-size: 0.9rem;
		color: var(--color-brand-text-primary);
	}

	.two-col {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}

	.mini-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.mini-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.75rem;
		padding: 0.55rem 0.7rem;
		background: var(--color-brand-surface);
		border: 1px solid var(--color-brand-border);
		border-radius: 6px;
		flex-wrap: wrap;
	}

	.mini-row strong {
		display: block;
		font-size: 0.84rem;
		color: var(--color-brand-text-primary);
	}

	.row-sub {
		display: block;
		font-size: 0.76rem;
		color: var(--color-brand-text-muted);
	}

	.row-controls {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}


	.token-box {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.75rem;
		padding: 0.7rem;
		background: var(--color-brand-surface);
		border: 1px solid var(--color-brand-border-strong);
		border-radius: 6px;
	}

	.token-box strong {
		display: block;
		font-size: 0.78rem;
		color: var(--color-brand-text-secondary);
		margin-bottom: 0.25rem;
	}

	.token-box code {
		font-size: 0.78rem;
		word-break: break-all;
		color: var(--color-brand-text-primary);
	}
</style>
