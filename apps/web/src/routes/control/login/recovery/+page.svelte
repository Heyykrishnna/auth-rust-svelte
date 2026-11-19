<script lang="ts">
	import { goto, replaceState } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { developerSecurityApi } from '$lib/api/developer-security';
	import AuthSplitShell from '$lib/components/AuthSplitShell.svelte';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { setDeveloper } from '$lib/stores/control.svelte';

	const field =
		'min-h-10 w-full min-w-0 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 py-2 text-sm text-brand-text-primary outline-none focus:border-action focus:ring-1 focus:ring-action';
	let token = $state('');
	let email = $state('');
	let password = $state('');
	let confirmation = $state('');
	let pending = $state(false);
	let requested = $state(false);
	let error = $state('');

	onMount(() => {
		token = new URLSearchParams(location.hash.slice(1)).get('token') ?? '';
		if (token) replaceState('/control/login/recovery', page.state);
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (pending) return;
		error = '';
		if (token && password !== confirmation) {
			error = 'Passwords do not match.';
			return;
		}
		pending = true;
		try {
			if (token) {
				await developerSecurityApi.resetPassword(token, password);
				setDeveloper(null);
				addToast('Password reset. Sign in with your new password.', 'success');
				await goto('/control/login', { replace: true });
			} else {
				await developerSecurityApi.requestRecovery(email.trim());
				requested = true;
			}
		} catch (err) {
			error =
				err instanceof Error
					? err.message
					: 'Could not complete account recovery.';
		} finally {
			pending = false;
		}
	}
</script>

<svelte:head>
	<title>Developer account recovery — Signet</title>
	<meta name="referrer" content="no-referrer" />
</svelte:head>

<AuthSplitShell topActionLabel="Back to sign in" topActionHref="/control/login">
	<h1
		class="font-heading text-2xl font-bold tracking-tight text-brand-text-primary"
	>
		{token ? 'Choose a new password' : 'Recover your developer account'}
	</h1>
	{#if requested}
		<p class="mt-3 text-sm text-brand-text-secondary" role="status">
			If this address has a developer account, you’ll receive a one-time
			password reset link.
		</p>
	{:else}
		<p class="mt-2 mb-5 text-sm text-brand-text-secondary">
			{token
				? 'Resetting your password signs out your developer sessions. Your registered authenticators stay active.'
				: 'Use the email address you sign in with.'}
		</p>
		{#if error}<p class="mb-3 text-sm text-brand-danger-text" role="alert">
				{error}
			</p>{/if}
		<form class="grid gap-4" onsubmit={submit}>
			{#if token}
				<label class="grid gap-1.5 text-sm font-medium">
					New password
					<input
						class={field}
						type="password"
						autocomplete="new-password"
						minlength="12"
						maxlength="128"
						bind:value={password}
						required
					/>
				</label>
				<label class="grid gap-1.5 text-sm font-medium">
					Confirm new password
					<input
						class={field}
						type="password"
						autocomplete="new-password"
						minlength="12"
						maxlength="128"
						bind:value={confirmation}
						required
					/>
				</label>
			{:else}
				<label class="grid gap-1.5 text-sm font-medium">
					Developer email
					<input
						class={field}
						type="email"
						autocomplete="email"
						maxlength="254"
						bind:value={email}
						required
					/>
				</label>
			{/if}
			<Button type="submit" loading={pending} full>
				{token ? 'Reset password' : 'Send reset link'}
			</Button>
		</form>
	{/if}
</AuthSplitShell>
