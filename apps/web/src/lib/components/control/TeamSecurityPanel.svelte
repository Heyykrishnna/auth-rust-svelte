<script lang="ts">
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import type { WorkspaceMemberDto, WorkspaceRole } from '$lib/api/types';
	import {
		canEditMemberRole,
		canManageOwners,
		canManageTeam,
		formatRelativeTimestamp
	} from '$lib/control/permissions';

	interface Props {
		workspaceId: string;
		workspaceName: string;
		actorRole: string;
		currentDeveloperId: string;
		mode: 'team' | 'security';
	}

	let { workspaceId, workspaceName, actorRole, currentDeveloperId, mode }: Props = $props();

	let members = $state<WorkspaceMemberDto[]>([]);
	let loading = $state(false);
	let inviteEmail = $state('');
	let inviteRole = $state<WorkspaceRole>('developer');
	let inviting = $state(false);

	let currentPassword = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let changingPassword = $state(false);
	let passwordError = $state('');

	const allowTeamManagement = $derived(canManageTeam(actorRole));
	const allowOwnerAssign = $derived(canManageOwners(actorRole));

	async function loadMembers() {
		if (mode !== 'team') return;
		loading = true;
		try {
			members = await controlApi.listWorkspaceMembers(workspaceId);
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load workspace team', 'error');
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const _ws = workspaceId;
		const _mode = mode;
		void _ws;
		void _mode;
		loadMembers();
	});

	async function handleAddMember(e: SubmitEvent) {
		e.preventDefault();
		if (!allowTeamManagement) return;
		inviting = true;
		try {
			await controlApi.addWorkspaceMember(workspaceId, {
				email: inviteEmail.trim(),
				role: inviteRole
			});
			inviteEmail = '';
			addToast('Workspace member added', 'success');
			await loadMembers();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to add member', 'error');
		} finally {
			inviting = false;
		}
	}

	async function handleRoleChange(member: WorkspaceMemberDto, nextRole: WorkspaceRole) {
		if (!canEditMemberRole(actorRole, member.role)) return;
		try {
			await controlApi.updateWorkspaceMemberRole(workspaceId, member.developer_id, nextRole);
			addToast(`Updated ${member.email} to ${nextRole}`, 'success');
			await loadMembers();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to update role', 'error');
		}
	}

	async function handleRemoveMember(member: WorkspaceMemberDto) {
		if (!canEditMemberRole(actorRole, member.role)) return;
		try {
			await controlApi.removeWorkspaceMember(workspaceId, member.developer_id);
			addToast(`Removed ${member.email} from workspace`, 'info');
			await loadMembers();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to remove member', 'error');
		}
	}

	async function handlePasswordChange(e: SubmitEvent) {
		e.preventDefault();
		passwordError = '';
		if (newPassword.length < 12) {
			passwordError = 'New password must be at least 12 characters.';
			return;
		}
		if (newPassword !== confirmPassword) {
			passwordError = 'Password confirmation does not match.';
			return;
		}
		changingPassword = true;
		try {
			await controlApi.changePassword({
				current_password: currentPassword,
				new_password: newPassword
			});
			currentPassword = '';
			newPassword = '';
			confirmPassword = '';
			addToast('Developer account password updated', 'success');
		} catch (err) {
			passwordError = err instanceof Error ? err.message : 'Password update failed';
		} finally {
			changingPassword = false;
		}
	}
</script>

{#if mode === 'team'}
	<section class="panel-stack" aria-label="Workspace team and roles">
		<div class="section-header">
			<div>
				<h1 class="page-title">Team &amp; Roles</h1>
				<p class="page-subtitle">Manage collaborators for {workspaceName} with strict RBAC enforcement (owner, admin, developer, viewer).</p>
			</div>
			<Badge variant="neutral" size="sm">Your role: {actorRole}</Badge>
		</div>

		{#if allowTeamManagement}
			<div class="surface-box">
				<h2 class="box-title">Add Workspace Member</h2>
				<form class="inline-form" onsubmit={handleAddMember}>
					<input type="email" bind:value={inviteEmail} placeholder="Registered developer email" required />
					<select bind:value={inviteRole} aria-label="Member role">
						{#if allowOwnerAssign}
							<option value="owner">Owner</option>
						{/if}
						<option value="admin">Admin</option>
						<option value="developer">Developer</option>
						<option value="viewer">Viewer (Read-only)</option>
					</select>
					<Button type="submit" variant="primary" size="sm" loading={inviting} disabled={inviting}>
						Add Member
					</Button>
				</form>
			</div>
		{/if}

		<div class="table-wrap">
			<table class="data-table">
				<thead>
					<tr>
						<th>Member</th>
						<th>Role</th>
						<th>Joined</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#if loading && members.length === 0}
						<tr><td colspan="4" class="muted">Loading team...</td></tr>
					{:else}
						{#each members as m}
							{@const editable = canEditMemberRole(actorRole, m.role)}
							<tr>
								<td>
									<div class="member-cell">
										<strong>{m.display_name || m.email} {m.developer_id === currentDeveloperId ? '(You)' : ''}</strong>
										<span>{m.email}</span>
									</div>
								</td>
								<td>
									{#if editable}
										<select
											class="role-select"
											value={m.role}
											aria-label={`Role for ${m.email}`}
											onchange={(e) =>
												handleRoleChange(
													m,
													(e.currentTarget as HTMLSelectElement).value as WorkspaceRole
												)}
										>
											{#if allowOwnerAssign}
												<option value="owner">owner</option>
											{/if}
											<option value="admin">admin</option>
											<option value="developer">developer</option>
											<option value="viewer">viewer</option>
										</select>
									{:else}
										<Badge variant="neutral" size="sm">{m.role}</Badge>
									{/if}
								</td>
								<td class="muted">{formatRelativeTimestamp(m.joined_at)}</td>
								<td class="row-actions">
									{#if editable && m.developer_id !== currentDeveloperId}
										<Button variant="danger" size="sm" onclick={() => handleRemoveMember(m)}>Remove</Button>
									{/if}
								</td>
							</tr>
						{/each}
					{/if}
				</tbody>
			</table>
		</div>
	</section>
{:else}
	<section class="panel-stack" aria-label="Developer account security">
		<div class="section-header">
			<div>
				<h1 class="page-title">Account Security</h1>
				<p class="page-subtitle">Rotate your Developer Control Plane password using Argon2id hashing.</p>
			</div>
		</div>

		<div class="surface-box narrow">
			<h2 class="box-title">Change Developer Password</h2>
			{#if passwordError}
				<p class="error-text" role="alert">{passwordError}</p>
			{/if}
			<form class="password-form" onsubmit={handlePasswordChange}>
				<label class="field">
					<span>Current Password</span>
					<input type="password" bind:value={currentPassword} autocomplete="current-password" required />
				</label>
				<label class="field">
					<span>New Password (minimum 12 characters)</span>
					<input type="password" bind:value={newPassword} autocomplete="new-password" minlength="12" required />
				</label>
				<label class="field">
					<span>Confirm New Password</span>
					<input type="password" bind:value={confirmPassword} autocomplete="new-password" minlength="12" required />
				</label>
				<div class="form-actions">
					<Button type="submit" variant="primary" size="sm" loading={changingPassword} disabled={changingPassword}>
						Update Password
					</Button>
				</div>
			</form>
		</div>
	</section>
{/if}

<style>
	.panel-stack { display: flex; flex-direction: column; gap: 1.5rem; }
	.section-header { display: flex; align-items: flex-end; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
	.page-title { font-size: 1.75rem; font-weight: 600; color: #ffffff; margin: 0; }
	.page-subtitle { font-size: 0.85rem; color: #71717a; margin: 0.25rem 0 0; }
	.surface-box { background-color: #15171b; border: 1px solid #23262e; border-radius: 12px; padding: 1.35rem; display: flex; flex-direction: column; gap: 1rem; }
	.narrow { max-width: 520px; }
	.box-title { font-size: 0.98rem; font-weight: 600; color: #f4f4f5; margin: 0; }
	.inline-form { display: flex; gap: 0.65rem; flex-wrap: wrap; }
	.inline-form input, .inline-form select, .role-select { background-color: #0b0c0e; border: 1px solid #23262e; border-radius: 6px; padding: 0.5rem 0.75rem; color: #f4f4f5; font-size: 0.82rem; outline: none; }
	.inline-form input { flex: 1; min-width: 220px; }
	.table-wrap { background-color: #15171b; border: 1px solid #23262e; border-radius: 12px; overflow-x: auto; }
	.data-table { width: 100%; border-collapse: collapse; text-align: left; }
	.data-table th { font-size: 0.72rem; color: #71717a; padding: 0.75rem 1rem; border-bottom: 1px solid #23262e; }
	.data-table td { padding: 0.85rem 1rem; border-bottom: 1px solid #1f2228; font-size: 0.84rem; color: #d4d4d8; }
	.member-cell { display: flex; flex-direction: column; }
	.member-cell strong { color: #ffffff; font-size: 0.86rem; }
	.member-cell span { color: #71717a; font-size: 0.76rem; }
	.muted { color: #71717a; font-size: 0.8rem; }
	.row-actions { text-align: right; }
	.password-form { display: flex; flex-direction: column; gap: 0.9rem; }
	.field { display: flex; flex-direction: column; gap: 0.35rem; }
	.field span { font-size: 0.76rem; color: #a1a1aa; }
	.field input { background-color: #0b0c0e; border: 1px solid #23262e; border-radius: 6px; padding: 0.55rem 0.75rem; color: #f4f4f5; font-size: 0.85rem; outline: none; }
	.form-actions { display: flex; justify-content: flex-end; }
</style>

