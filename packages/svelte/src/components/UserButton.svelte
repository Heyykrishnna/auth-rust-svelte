<script lang="ts">
	import { getContext } from 'svelte';
	import {
		AUTH_CONTEXT_KEY,
		type PublicAuthState,
		type SvelteAuthContext
	} from '../index.js';

	interface Props {
		globalSignOut?: boolean;
		postLogoutRedirectUri?: string;
		onSignedOut?: () => void;
	}

	let { globalSignOut = false, postLogoutRedirectUri, onSignedOut }: Props = $props();
	const auth = getContext<SvelteAuthContext>(AUTH_CONTEXT_KEY);

	let state = $state<PublicAuthState>(auth.getState());
	let menuOpen = $state(false);
	let busy = $state(false);

	$effect(() => {
		return auth.subscribe((next) => {
			state = next;
		});
	});

	let initials = $derived.by(() => {
		const source = state.user?.display_name ?? state.user?.email ?? 'U';
		return source.slice(0, 2).toUpperCase();
	});

	async function handleSignOut(): Promise<void> {
		busy = true;
		try {
			const result = await auth.signOut({
				global: globalSignOut,
				postLogoutRedirectUri
			});
			menuOpen = false;
			onSignedOut?.();
			if (result.logoutUrl && typeof window !== 'undefined') {
				window.location.assign(result.logoutUrl);
			}
		} finally {
			busy = false;
		}
	}
</script>

{#if state.isSignedIn}
	<div class="sa-user-button">
		<button
			type="button"
			class="sa-avatar-trigger"
			aria-haspopup="menu"
			aria-expanded={menuOpen}
			onclick={() => {
				menuOpen = !menuOpen;
			}}
		>
			<span class="sa-avatar">{initials}</span>
		</button>
		{#if menuOpen}
			<div class="sa-menu" role="menu">
				<div class="sa-menu-header">
					<strong>{state.user?.display_name ?? 'Account'}</strong>
					{#if state.user?.email}
						<span>{state.user.email}</span>
					{/if}
				</div>
				<button
					type="button"
					role="menuitem"
					class="sa-menu-item"
					disabled={busy}
					onclick={handleSignOut}
				>
					{busy ? 'Signing out…' : 'Sign out'}
				</button>
			</div>
		{/if}
	</div>
{/if}

