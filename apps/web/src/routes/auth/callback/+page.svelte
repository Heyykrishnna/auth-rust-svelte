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
			errorMessage =
				error === 'access_denied'
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
	<title>Completing Sign In — Signet</title>
</svelte:head>

<div class="min-h-[70vh] flex items-center justify-center p-6">
	<div class="max-w-md w-full p-8 rounded-xl bg-brand-surface-card border border-brand-border text-center shadow-2xl">
		{#if status === 'loading'}
			<div
				class="w-12 h-12 border-4 border-brand-lime border-r-transparent rounded-full animate-spin mx-auto mb-6"
				aria-hidden="true"
			></div>
			<h2 class="text-xl font-bold font-heading text-brand-text-primary mb-2">Completing sign in…</h2>
			<p class="text-sm text-brand-text-secondary m-0">Please wait while we authenticate you.</p>
		{:else if status === 'success'}
			<div class="w-16 h-16 rounded-full flex items-center justify-center text-2xl font-bold mx-auto mb-6 bg-emerald-500/15 text-emerald-300 border-2 border-emerald-500/30">
				✓
			</div>
			<h2 class="text-xl font-bold font-heading text-brand-text-primary mb-2">Successfully signed in!</h2>
			<p class="text-sm text-brand-text-secondary m-0">Redirecting to your dashboard…</p>
		{:else}
			<div class="w-16 h-16 rounded-full flex items-center justify-center text-2xl font-bold mx-auto mb-6 bg-rose-500/15 text-rose-300 border-2 border-rose-500/30">
				✕
			</div>
			<h2 class="text-xl font-bold font-heading text-brand-text-primary mb-2">Authentication failed</h2>
			<p class="text-sm text-brand-text-secondary mb-6">{errorMessage}</p>
			<a
				href="/login"
				class="action-button inline-flex min-h-10 shrink-0 items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold no-underline"
			>
				Try Again
			</a>
		{/if}
	</div>
</div>
