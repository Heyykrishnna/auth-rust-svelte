<script lang="ts">
	import SectionHeader from './SectionHeader.svelte';
	import Dropdown from '$lib/components/Dropdown.svelte';
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
    const roleOptions = $derived.by(() => {
        const roles: { value: WorkspaceRole; label: string }[] = [
            { value: 'admin', label: 'Admin' },
            { value: 'developer', label: 'Developer' },
            { value: 'viewer', label: 'Viewer' }
        ];
        if (allowOwnerAssign) roles.unshift({ value: 'owner', label: 'Owner' });
        return roles;
    });

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
	<section class="grid min-w-0 grid-cols-1 gap-5 sm:gap-6" aria-label="Workspace team and roles">
		<SectionHeader title="Team &amp; Roles" description="Manage collaborators for {workspaceName} with strict RBAC enforcement (owner, admin, developer, viewer).">
			<Badge variant="neutral" size="sm">Your role: {actorRole}</Badge>
		</SectionHeader>

		{#if allowTeamManagement}
			<div class="surface-box">
				<h2 class="box-title">Add Workspace Member</h2>
				<form class="flex min-w-0 flex-wrap items-center gap-2 [&>button]:h-9 [&>input]:h-9 [&>input]:max-w-full" onsubmit={handleAddMember}>
					<input class="control-input" type="email" bind:value={inviteEmail} placeholder="Registered developer email" required />
					<Dropdown label="Member role" bind:value={inviteRole} options={roleOptions} disabled={inviting} />
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
							{@const currentRole = roleOptions.find(({ value }) => value === m.role)?.value}
							<tr>
								<td>
									<div class="member-cell">
										<strong>{m.display_name || m.email} {m.developer_id === currentDeveloperId ? '(You)' : ''}</strong>
										<span>{m.email}</span>
									</div>
								</td>
								<td>
									{#if editable && currentRole}
										<Dropdown
											label={`Role for ${m.email}`}
											value={currentRole}
											options={roleOptions}
											onchange={(role) => handleRoleChange(m, role)}
										/>
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
	<section class="grid min-w-0 grid-cols-1 gap-5 sm:gap-6" aria-label="Developer account security">
		<SectionHeader title="Account Security" description="Rotate your Developer Control Plane password using Argon2id hashing." />

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
	.surface-box { background-color: var(--color-brand-surface-card); border: 1px solid var(--color-brand-border); border-radius: 12px; padding: 1.35rem; display: flex; flex-direction: column; gap: 1rem; }
	.narrow { max-width: 520px; }
	.box-title { font-size: 0.98rem; font-weight: 600; color: var(--color-brand-text-primary); margin: 0; }
	.control-input { background-color: var(--color-brand-surface); border: 1px solid var(--color-brand-border); border-radius: 6px; padding: 0.5rem 0.75rem; color: var(--color-brand-text-primary); font-size: 0.82rem; outline: none; }
	.control-input { flex: 1; min-width: 220px; }
	.table-wrap { background-color: var(--color-brand-surface-card); border: 1px solid var(--color-brand-border); border-radius: 12px; overflow-x: auto; }
	.data-table { width: 100%; border-collapse: collapse; text-align: left; }
	.data-table th { font-size: 0.72rem; color: var(--color-brand-text-secondary); padding: 0.75rem 1rem; border-bottom: 1px solid var(--color-brand-border); }
	.data-table td { padding: 0.85rem 1rem; border-bottom: 1px solid var(--color-brand-border); font-size: 0.84rem; color: #d4d4d4; }
	.member-cell { display: flex; flex-direction: column; }
	.member-cell strong { color: #ffffff; font-size: 0.86rem; }
	.member-cell span { color: var(--color-brand-text-secondary); font-size: 0.76rem; }
	.muted { color: var(--color-brand-text-secondary); font-size: 0.8rem; }
	.row-actions { text-align: right; }
	.password-form { display: flex; flex-direction: column; gap: 0.9rem; }
	.field { display: flex; flex-direction: column; gap: 0.35rem; }
	.field span { font-size: 0.76rem; color: var(--color-brand-text-secondary); }
	.field input { background-color: var(--color-brand-surface); border: 1px solid var(--color-brand-border); border-radius: 6px; padding: 0.55rem 0.75rem; color: var(--color-brand-text-primary); font-size: 0.85rem; outline: none; }
	.form-actions { display: flex; justify-content: flex-end; }
</style>

