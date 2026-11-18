<script lang="ts">
	import { onMount } from 'svelte';
	import { controlApi } from '$lib/api/control';
	import type {
		WorkspaceMemberDto,
		WorkspaceRole,
		WorkspaceSummary
	} from '$lib/api/types';
	import Button from '$lib/components/Button.svelte';
	import Dropdown from '$lib/components/Dropdown.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import {
		canEditMemberRole,
		canManageOwners,
		canManageTeam,
		getInitials
	} from '$lib/control/permissions';
	import SectionHeader from './SectionHeader.svelte';
	import WorkspaceSearch from './WorkspaceSearch.svelte';

	let {
		workspace,
		currentDeveloperId,
		onWorkspaceUpdated
	}: {
		workspace: WorkspaceSummary;
		currentDeveloperId: string;
		onWorkspaceUpdated: (workspace: WorkspaceSummary) => void;
	} = $props();

	let members = $state<WorkspaceMemberDto[]>([]);
	let loading = $state(true);
	let loadError = $state('');
	let error = $state('');
	let search = $state('');
	let adding = $state(false);
	let removing = $state<WorkspaceMemberDto>();
	let busy = $state(false);
	let email = $state('');
	let role = $state<WorkspaceRole>('developer');
	let addForm: HTMLFormElement;

	const mayManage = $derived(canManageTeam(workspace.role));
	const roles = $derived(
		(
			[
				{ value: 'owner', label: 'Owner' },
				{ value: 'admin', label: 'Admin' },
				{ value: 'developer', label: 'Developer' },
				{ value: 'viewer', label: 'Viewer' }
			] as const
		).filter(
			({ value }) => value !== 'owner' || canManageOwners(workspace.role)
		)
	);
	const visibleMembers = $derived.by(() => {
		const query = search.trim().toLowerCase();
		return members.filter(
			({ display_name, email }) =>
				display_name?.toLowerCase().includes(query) ||
				email.toLowerCase().includes(query)
		);
	});
	const dateFormat = new Intl.DateTimeFormat(undefined, {
		dateStyle: 'medium'
	});

	onMount(() => {
		void loadMembers();
	});

	async function loadMembers() {
		loading = true;
		loadError = '';
		try {
			members = await controlApi.listWorkspaceMembers(workspace.id);
		} catch (err) {
			loadError =
				err instanceof Error ? err.message : 'Could not load team members.';
		} finally {
			loading = false;
		}
	}

	function openAddDialog() {
		email = '';
		role = 'developer';
		error = '';
		adding = true;
	}

	async function addMember(event: SubmitEvent) {
		event.preventDefault();
		if (!mayManage || busy) return;
		busy = true;
		error = '';
		try {
			const added = await controlApi.addWorkspaceMember(workspace.id, {
				email: email.trim(),
				role
			});
			members = [...members, added];
			adding = false;
			addToast('Workspace member added', 'success');
		} catch (err) {
			error = err instanceof Error ? err.message : 'Could not add the member.';
		} finally {
			busy = false;
		}
	}

	async function changeRole(
		member: WorkspaceMemberDto,
		nextRole: WorkspaceRole
	) {
		if (!canEditMemberRole(workspace.role, member.role) || busy) return;
		busy = true;
		try {
			const updated = await controlApi.updateWorkspaceMemberRole(
				workspace.id,
				member.developer_id,
				nextRole
			);
			members = members.map((row) =>
				row.developer_id === updated.developer_id ? updated : row
			);
			if (updated.developer_id === currentDeveloperId) {
				onWorkspaceUpdated({ ...workspace, role: updated.role });
			}
			addToast('Member role updated', 'success');
		} catch (err) {
			addToast(
				err instanceof Error ? err.message : 'Could not update the role.',
				'error'
			);
		} finally {
			busy = false;
		}
	}

	async function removeMember() {
		const member = removing;
		if (!member || !canEditMemberRole(workspace.role, member.role) || busy)
			return;
		busy = true;
		error = '';
		try {
			await controlApi.removeWorkspaceMember(workspace.id, member.developer_id);
			members = members.filter(
				(row) => row.developer_id !== member.developer_id
			);
			removing = undefined;
			addToast('Workspace member removed', 'success');
		} catch (err) {
			error =
				err instanceof Error ? err.message : 'Could not remove the member.';
		} finally {
			busy = false;
		}
	}
</script>

<SectionHeader
	title="Team"
	description="Manage who can access this workspace and what they can do."
>
	{#if mayManage}
		<Button
			onclick={openAddDialog}
			disabled={loading || Boolean(loadError) || busy}
		>
			Add member
		</Button>
	{/if}
</SectionHeader>

{#if loadError}
	<div
		class="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-brand-danger-border bg-brand-danger-soft p-4"
	>
		<p role="alert" class="text-sm text-brand-danger-text">{loadError}</p>
		<Button size="sm" onclick={loadMembers}>Retry</Button>
	</div>
{:else if loading}
	<p
		role="status"
		class="task-surface p-5 text-sm text-brand-text-secondary"
	>
		Loading team members…
	</p>
{:else}
	<section
		class="min-w-0 overflow-hidden task-surface"
		aria-label="Workspace members"
	>
		<div
			class="flex flex-wrap items-center justify-between gap-3 border-b border-brand-border p-4 sm:px-5"
		>
			<div>
				<h2 class="font-heading text-base font-bold">Members</h2>
				<p class="mt-0.5 text-xs text-brand-text-secondary">
					{members.length}
					{members.length === 1 ? 'person' : 'people'} with access
				</p>
			</div>
			{#if members.length > 0}
				<WorkspaceSearch label="Search members" bind:value={search} />
			{/if}
		</div>
		{#if visibleMembers.length > 0}
			<ul class="m-0 list-none divide-y divide-brand-border p-0">
				{#each visibleMembers as member (member.developer_id)}
					{@const self = member.developer_id === currentDeveloperId}
					{@const editable = canEditMemberRole(workspace.role, member.role)}
					{@const currentRole = roles.find(
						(option) => option.value === member.role
					)?.value}
					<li
						class="grid min-w-0 gap-3 px-4 py-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center sm:px-5"
					>
						<div class="flex min-w-0 items-center gap-3">
							<span
								class="grid size-9 shrink-0 place-items-center rounded-full bg-brand-surface-alt text-xs font-semibold"
								aria-hidden="true"
							>
								{getInitials(member.display_name, member.email)}
							</span>
							<div class="min-w-0">
								<p class="text-sm font-medium break-words">
									{member.display_name || member.email}
									{#if self}<span
											class="text-xs font-normal text-brand-text-muted"
										>
											(you)
										</span>{/if}
								</p>
								<p class="mt-0.5 text-xs break-words text-brand-text-secondary">
									{member.email} <span aria-hidden="true">·</span> Joined
									<time datetime={member.joined_at}>
										{dateFormat.format(new Date(member.joined_at))}
									</time>
								</p>
							</div>
						</div>
						<div
							class="flex min-w-0 flex-wrap items-center gap-2 sm:justify-end"
						>
							{#if editable && currentRole}
								<Dropdown
									label={`Role for ${member.email}`}
									value={currentRole}
									options={roles}
									disabled={busy}
									onchange={(nextRole) => changeRole(member, nextRole)}
								/>
							{:else}
								<span class="px-2 text-sm text-brand-text-secondary capitalize">
									{member.role}
								</span>
							{/if}
							{#if editable && !self}
								<Button
									size="sm"
									disabled={busy}
									onclick={() => {
										error = '';
										removing = member;
									}}
								>
									Remove
								</Button>
							{/if}
						</div>
					</li>
				{/each}
			</ul>
		{:else}
			<p
				class="px-5 py-9 text-center text-sm text-brand-text-secondary"
				role="status"
			>
				{search.trim()
					? 'No members match your search.'
					: 'No workspace members found.'}
			</p>
		{/if}
	</section>
{/if}

<Modal
	open={adding}
	title="Add workspace member"
	description="Give an existing Signet developer account access to your workspace."
	confirmText="Add member"
	loading={busy}
	onconfirm={() => addForm.requestSubmit()}
	oncancel={() => (adding = false)}
>
	<form bind:this={addForm} class="grid gap-4 pt-2" onsubmit={addMember}>
		<label class="grid gap-1.5 font-medium text-brand-text-primary">
			Developer email
			<input
				class="min-h-10 min-w-0 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 py-2 outline-none focus:border-brand-lime focus:ring-1 focus:ring-brand-lime"
				type="email"
				bind:value={email}
				required
			/>
		</label>
		<div class="grid gap-1.5">
			<span class="font-medium text-brand-text-primary">Role</span>
			<Dropdown
				label="Member role"
				bind:value={role}
				options={roles}
				disabled={busy}
			/>
		</div>
		{#if error}<p role="alert" class="text-brand-danger-text">{error}</p>{/if}
		<button type="submit" hidden>Submit</button>
	</form>
</Modal>

<Modal
	open={Boolean(removing)}
	title="Remove workspace member"
	description={removing
		? `${removing.email} will lose access to this workspace and its applications.`
		: undefined}
	confirmText="Remove member"
	variant="danger"
	loading={busy}
	onconfirm={removeMember}
	oncancel={() => (removing = undefined)}
>
	{#if error}<p role="alert" class="text-brand-danger-text">{error}</p>{/if}
</Modal>
