<script lang="ts">
	import { getContext } from 'svelte';
	import {
		AUTH_CONTEXT_KEY,
		type PublicAuthState,
		type SvelteAuthContext
	} from '../index.js';

	interface Props {
		allowPersonal?: boolean;
		onSwitch?: (orgId: string | null) => void;
	}

	let { allowPersonal = true, onSwitch }: Props = $props();
	const auth = getContext<SvelteAuthContext>(AUTH_CONTEXT_KEY);

	let state = $state<PublicAuthState>(auth.getState());
	let open = $state(false);
	let switching = $state(false);
	let errorMessage = $state<string | null>(null);

	$effect(() => {
		const unsub = auth.subscribe((next) => {
			state = next;
		});
		if (state.isSignedIn && state.organizations.length === 0) {
			void auth.refreshOrganizations().catch(() => undefined);
		}
		return unsub;
	});

	let activeLabel = $derived.by(() => {
		if (!state.activeOrgId) {
			return 'Personal Account';
		}
		const hit = state.organizations.find((item) => item.organization.id === state.activeOrgId);
		return hit?.organization.name ?? state.orgSlug ?? 'Organization';
	});

	async function selectOrg(orgId: string | null): Promise<void> {
		switching = true;
		errorMessage = null;
		try {
			await auth.setActiveOrganization(orgId);
			open = false;
			onSwitch?.(orgId);
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Unable to switch organization';
		} finally {
			switching = false;
		}
	}
</script>

{#if state.isSignedIn}
	<div class="sa-org-switcher">
		<button
			type="button"
			class="sa-org-trigger"
			aria-haspopup="listbox"
			aria-expanded={open}
			disabled={switching}
			onclick={() => {
				open = !open;
			}}
		>
			<span class="sa-org-label">{activeLabel}</span>
			{#if state.orgRole}
				<span class="sa-org-role">{state.orgRole}</span>
			{/if}
		</button>
		{#if open}
			<div class="sa-org-menu" role="listbox" aria-label="Organizations">
				{#if allowPersonal}
					<button
						type="button"
						role="option"
						class="sa-org-item"
						aria-selected={state.activeOrgId === null}
						disabled={switching}
						onclick={() => selectOrg(null)}
					>
						<span>Personal Account</span>
					</button>
				{/if}
				{#each state.organizations as item (item.organization.id)}
					<button
						type="button"
						role="option"
						class="sa-org-item"
						aria-selected={state.activeOrgId === item.organization.id}
						disabled={switching}
						onclick={() => selectOrg(item.organization.id)}
					>
						<span>{item.organization.name}</span>
						<span class="sa-org-meta">{item.roles[0] ?? 'org:member'}</span>
					</button>
				{/each}
			</div>
		{/if}
		{#if errorMessage}
			<p class="sa-error" role="alert">{errorMessage}</p>
		{/if}
	</div>
{/if}

