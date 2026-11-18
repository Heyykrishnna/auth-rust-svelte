<script lang="ts">
	import { getContext } from 'svelte';
	import { AUTH_CONTEXT_KEY, type SvelteAuthContext } from '../index.js';

	interface Props {
		mode?: 'hosted' | 'direct';
		redirectUri?: string;
		onSuccess?: () => void;
		onError?: (error: unknown) => void;
	}

	let { mode = 'hosted', redirectUri, onSuccess, onError }: Props = $props();
	const auth = getContext<SvelteAuthContext>(AUTH_CONTEXT_KEY);

	let email = $state('');
	let password = $state('');
	let submitting = $state(false);
	let errorMessage = $state<string | null>(null);

	async function handleHostedRedirect(): Promise<void> {
		submitting = true;
		errorMessage = null;
		try {
			const url = await auth.signInWithRedirect({ redirectUri });
			if (typeof window !== 'undefined') {
				window.location.assign(url);
			}
			onSuccess?.();
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Unable to start sign-in';
			onError?.(err);
		} finally {
			submitting = false;
		}
	}

	async function handleDirectSubmit(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		submitting = true;
		errorMessage = null;
		try {
			await auth.signInWithPassword({ email: email.trim(), password });
			onSuccess?.();
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Sign-in failed';
			onError?.(err);
		} finally {
			submitting = false;
		}
	}
</script>

<section class="sa-card" aria-label="Sign in">
	{#if mode === 'hosted'}
		<button
			type="button"
			class="sa-button sa-button-primary"
			disabled={submitting}
			onclick={handleHostedRedirect}
		>
			{submitting ? 'Redirecting…' : 'Continue with SSO'}
		</button>
	{:else}
		<form class="sa-form" onsubmit={handleDirectSubmit}>
			<div class="sa-field">
				<label for="sa-signin-email">Email</label>
				<input
					id="sa-signin-email"
					type="email"
					autocomplete="email"
					required
					bind:value={email}
				/>
			</div>
			<div class="sa-field">
				<label for="sa-signin-password">Password</label>
				<input
					id="sa-signin-password"
					type="password"
					autocomplete="current-password"
					required
					bind:value={password}
				/>
			</div>
			<button type="submit" class="sa-button sa-button-primary" disabled={submitting}>
				{submitting ? 'Signing in…' : 'Sign in'}
			</button>
		</form>
	{/if}
	{#if errorMessage}
		<p class="sa-error" role="alert" aria-live="polite">{errorMessage}</p>
	{/if}
</section>
