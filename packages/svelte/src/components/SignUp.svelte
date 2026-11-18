<script lang="ts">
	import { getContext } from 'svelte';
	import { AUTH_CONTEXT_KEY, type SvelteAuthContext } from '../index.js';

	interface Props {
		onSuccess?: () => void;
		onError?: (error: unknown) => void;
	}

	let { onSuccess, onError }: Props = $props();
	const auth = getContext<SvelteAuthContext>(AUTH_CONTEXT_KEY);

	let displayName = $state('');
	let email = $state('');
	let password = $state('');
	let otpCode = $state('');
	let awaitingVerification = $state(false);
	let submitting = $state(false);
	let errorMessage = $state<string | null>(null);

	async function handleSignUp(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		submitting = true;
		errorMessage = null;
		try {
			await auth.signUp({
				email: email.trim(),
				password,
				displayName: displayName.trim()
			});
			awaitingVerification = true;
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Sign-up failed';
			onError?.(err);
		} finally {
			submitting = false;
		}
	}

	async function handleVerifyOtp(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		submitting = true;
		errorMessage = null;
		try {
			await auth.verifyOtp({ email: email.trim(), code: otpCode.trim() });
			onSuccess?.();
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Verification failed';
			onError?.(err);
		} finally {
			submitting = false;
		}
	}
</script>

<section class="sa-card" aria-label="Create account">
	{#if !awaitingVerification}
		<form class="sa-form" onsubmit={handleSignUp}>
			<div class="sa-field">
				<label for="sa-signup-name">Display name</label>
				<input
					id="sa-signup-name"
					type="text"
					autocomplete="name"
					required
					bind:value={displayName}
				/>
			</div>
			<div class="sa-field">
				<label for="sa-signup-email">Email</label>
				<input
					id="sa-signup-email"
					type="email"
					autocomplete="email"
					required
					bind:value={email}
				/>
			</div>
			<div class="sa-field">
				<label for="sa-signup-password">Password</label>
				<input
					id="sa-signup-password"
					type="password"
					autocomplete="new-password"
					required
					minlength={12}
					bind:value={password}
				/>
			</div>
			<button type="submit" class="sa-button sa-button-primary" disabled={submitting}>
				{submitting ? 'Creating account…' : 'Create account'}
			</button>
		</form>
	{:else}
		<form class="sa-form" onsubmit={handleVerifyOtp}>
			<div class="sa-field">
				<label for="sa-signup-otp">Verification code</label>
				<input
					id="sa-signup-otp"
					type="text"
					inputmode="numeric"
					autocomplete="one-time-code"
					required
					bind:value={otpCode}
				/>
			</div>
			<button type="submit" class="sa-button sa-button-primary" disabled={submitting}>
				{submitting ? 'Verifying…' : 'Verify email'}
			</button>
		</form>
	{/if}
	{#if errorMessage}
		<p class="sa-error" role="alert" aria-live="polite">{errorMessage}</p>
	{/if}
</section>

