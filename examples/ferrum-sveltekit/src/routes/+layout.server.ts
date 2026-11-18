import {
	serializeClientAuthState,
	type SvelteKitAuthLocals
} from '@secure-auth/svelte/server';

export function load(input: { locals: { auth?: SvelteKitAuthLocals } }) {
	return {
		auth: serializeClientAuthState(input.locals)
	};
}

