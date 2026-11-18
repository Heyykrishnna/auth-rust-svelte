<script lang="ts">
	import { setContext, type Snippet } from 'svelte';
	import type { AuthClientConfig } from '@secure-auth/core/client';
	import {
		AUTH_CONTEXT_KEY,
		createSvelteAuthContext,
		type PublicAuthState
	} from '../index.js';

	interface Props {
		config: AuthClientConfig;
		initialState?: Partial<PublicAuthState>;
		children?: Snippet;
	}

	let { config, initialState, children }: Props = $props();

	const authContext = createSvelteAuthContext(config, initialState);
	setContext(AUTH_CONTEXT_KEY, authContext);

	$effect(() => {
		if (initialState) {
			authContext.hydrate(initialState);
		}
	});
</script>

{@render children?.()}

