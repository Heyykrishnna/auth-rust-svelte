<script lang="ts">
	import { getContext, type Snippet } from 'svelte';
	import {
		AUTH_CONTEXT_KEY,
		type PublicAuthState,
		type SvelteAuthContext
	} from '../index.js';

	interface Props {
		children?: Snippet;
	}

	let { children }: Props = $props();
	const auth = getContext<SvelteAuthContext>(AUTH_CONTEXT_KEY);
	let state = $state<PublicAuthState>(auth.getState());

	$effect(() => {
		return auth.subscribe((next) => {
			state = next;
		});
	});
</script>

{#if state.isLoaded && state.isSignedIn}
	{@render children?.()}
{/if}

