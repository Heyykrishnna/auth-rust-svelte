// See https://svelte.dev/docs/kit/types#app.d.ts
import type { AuthUser } from '$lib/api/auth';

declare global {
	namespace App {
		interface Locals {
			user: AuthUser | null;
		}
		interface PageData {
			user?: AuthUser | null;
		}
		// interface Error {}
		// interface Platform {}
	}
}

export {};
