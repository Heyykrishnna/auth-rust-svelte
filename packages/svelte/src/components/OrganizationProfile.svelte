<script lang="ts">
	import { getContext } from 'svelte';
	import type { OrganizationInvitationView, OrganizationMemberView } from '@secure-auth/core/client';
	import {
		AUTH_CONTEXT_KEY,
		type PublicAuthState,
		type SvelteAuthContext
	} from '../index.js';

	interface Props {
		organizationId?: string;
	}

	let { organizationId }: Props = $props();
	const auth = getContext<SvelteAuthContext>(AUTH_CONTEXT_KEY);

	let state = $state<PublicAuthState>(auth.getState());
	let members = $state<OrganizationMemberView[]>([]);
	let invitations = $state<OrganizationInvitationView[]>([]);
	let inviteEmail = $state('');
	let inviteRole = $state('org:member');
	let busy = $state(false);
	let errorMessage = $state<string | null>(null);
	let statusMessage = $state<string | null>(null);

	$effect(() => {
		return auth.subscribe((next) => {
			state = next;
		});
	});

	let targetOrgId = $derived(organizationId ?? state.activeOrgId);

	$effect(() => {
		if (!targetOrgId || !state.isSignedIn) {
			members = [];
			invitations = [];
			return;
		}
		void loadOrgDetails(targetOrgId);
	});

	async function loadOrgDetails(orgId: string): Promise<void> {
		errorMessage = null;
		try {
			members = await auth.client.listOrganizationMembers(orgId);
			invitations = await auth.client
				.listOrganizationInvitations(orgId)
				.catch(() => [] as OrganizationInvitationView[]);
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Unable to load organization details';
		}
	}

	async function handleInvite(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		if (!targetOrgId || !inviteEmail.trim()) return;
		busy = true;
		errorMessage = null;
		statusMessage = null;
		try {
			await auth.client.createOrganizationInvitation(targetOrgId, {
				email: inviteEmail.trim(),
				roleSlug: inviteRole
			});
			inviteEmail = '';
			statusMessage = 'Invitation created';
			await loadOrgDetails(targetOrgId);
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Unable to invite member';
		} finally {
			busy = false;
		}
	}
</script>

{#if targetOrgId}
	<section class="sa-org-profile" aria-label="Organization profile">
		<header class="sa-org-profile-header">
			<h3>{state.orgSlug ?? targetOrgId}</h3>
			{#if state.orgRole}
				<span class="sa-org-role">{state.orgRole}</span>
			{/if}
		</header>

		{#if errorMessage}
			<p class="sa-error" role="alert">{errorMessage}</p>
		{/if}
		{#if statusMessage}
			<p class="sa-status" role="status">{statusMessage}</p>
		{/if}

		<div class="sa-org-members">
			<h4>Members ({members.length})</h4>
			<ul>
				{#each members as member (member.id)}
					<li>
						<span>{member.display_name} ({member.email})</span>
						<span>{member.roles.join(', ')}</span>
					</li>
				{/each}
			</ul>
		</div>

		<form class="sa-form" onsubmit={handleInvite}>
			<label for="sa-org-invite-email">Invite member</label>
			<input
				id="sa-org-invite-email"
				type="email"
				required
				placeholder="teammate@company.com"
				bind:value={inviteEmail}
			/>
			<select aria-label="Invitation role" bind:value={inviteRole}>
				<option value="org:member">org:member</option>
				<option value="org:admin">org:admin</option>
				<option value="org:owner">org:owner</option>
			</select>
			<button type="submit" disabled={busy}>
				{busy ? 'Sending…' : 'Send invitation'}
			</button>
		</form>

		{#if invitations.length > 0}
			<div class="sa-org-invitations">
				<h4>Pending invitations ({invitations.length})</h4>
				<ul>
					{#each invitations as inv (inv.id)}
						<li>
							<span>{inv.email}</span>
							<span>{inv.role_slug ?? 'org:member'} · {inv.status}</span>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</section>
{/if}

