<script lang="ts">
	import { goto } from '$app/navigation';
	import { authApi } from '$lib/api/auth';
	import { setAuth } from '$lib/stores/auth.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { onMount } from 'svelte';

	let status = $state<'loading' | 'success' | 'error'>('loading');
	let errorMessage = $state('');

	onMount(async () => {
		const url = new URL(window.location.href);
		const code = url.searchParams.get('code');
		const state = url.searchParams.get('state');
		const error = url.searchParams.get('error');
		const provider = (url.searchParams.get('provider') ?? 'google') as 'google' | 'github';

		if (error) {
			status = 'error';
			errorMessage = error === 'access_denied'
				? 'You denied access to your account.'
				: `Authentication error: ${error}`;
			return;
		}

		if (!code || !state) {
			status = 'error';
			errorMessage = 'Invalid callback parameters.';
			return;
		}

		try {
			const session = await authApi.oidcCallback(provider, code, state);
			setAuth(session.user);
			status = 'success';
			addToast('Successfully signed in!', 'success');
			setTimeout(() => goto('/dashboard'), 1000);
		} catch {
			status = 'error';
			errorMessage = 'Failed to complete sign-in. Please try again.';
		}
	});
</script>

<svelte:head>
	<title>Completing Sign In — AuthApp</title>
</svelte:head>

<div class="auth-layout">
	<div class="auth-card glass-card" style="text-align: center;">
		{#if status === 'loading'}
			<div class="spinner" style="width: 3rem; height: 3rem; border-width: 3px; margin: 0 auto 1.5rem;"></div>
			<h2>Completing sign in…</h2>
			<p>Please wait while we authenticate you.</p>
		{:else if status === 'success'}
			<div class="success-icon">✓</div>
			<h2>Successfully signed in!</h2>
			<p>Redirecting to your dashboard…</p>
		{:else}
			<div class="error-icon">✕</div>
			<h2>Authentication failed</h2>
			<p>{errorMessage}</p>
			<a href="/login" class="btn btn-primary" style="margin-top: 1.5rem; display: inline-flex;">
				Try Again
			</a>
		{/if}
	</div>
</div>

<style>
	.success-icon, .error-icon {
		width: 4rem;
		height: 4rem;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 1.75rem;
		font-weight: 700;
		margin: 0 auto 1.5rem;
	}

	.success-icon { background: rgba(16, 185, 129, 0.15); color: #6ee7b7; border: 2px solid rgba(16, 185, 129, 0.3); }
	.error-icon   { background: rgba(239, 68, 68, 0.15);  color: #fca5a5; border: 2px solid rgba(239, 68, 68, 0.3); }
</style>
