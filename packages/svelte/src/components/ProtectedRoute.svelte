<script lang="ts">
	import { getContext, type Snippet } from 'svelte';
	import {
		AUTH_CONTEXT_KEY,
		canAccessRoute,
		type PublicAuthState,
		type SvelteAuthContext
	} from '../index.js';

	interface Props {
		permission?: string;
		role?: string;
		children?: Snippet;
		fallback?: Snippet;
	}

	let { permission, role, children, fallback }: Props = $props();
	const auth = getContext<SvelteAuthContext>(AUTH_CONTEXT_KEY);
	let state = $state<PublicAuthState>(auth.getState());

	$effect(() => {
		return auth.subscribe((next) => {
			state = next;
		});
	});

	let allowed = $derived(canAccessRoute(state, { permission, role }));
</script>

{#if state.isLoaded && allowed}
	{@render children?.()}
{:else if state.isLoaded}
	{@render fallback?.()}
{/if}

