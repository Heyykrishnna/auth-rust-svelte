<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { portalApi, type PortalTransactionContext } from '$lib/api/portal';
	import { isGlobalEmailCollisionError, resolvePortalBrandTheme } from '$lib/portal/state';

	let txContext = $state<PortalTransactionContext | null>(null);
	let loading = $state(true);
	let submitting = $state(false);
	let step = $state<'form' | 'otp'>('form');
	let errorMessage = $state('');
	let emailCollision = $state(false);

	let displayName = $state('');
	let email = $state('');
	let password = $state('');
	let otpCode = $state('');

	const txId = $derived(page.url.searchParams.get('tx') ?? '');
	const brand = $derived(
		txContext
			? resolvePortalBrandTheme(txContext.branding, txContext.application_name)
			: resolvePortalBrandTheme(null, 'Application')
	);

	onMount(async () => {
		if (!txId) {
			errorMessage = 'Missing OIDC authorization transaction ID.';
			loading = false;
			return;
		}
		try {
			txContext = await portalApi.getTransaction(txId);
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Unable to load registration session.';
		} finally {
			loading = false;
		}
	});

	async function handleSignUp(e: SubmitEvent) {
		e.preventDefault();
		if (!txContext) return;
		errorMessage = '';
		emailCollision = false;
		submitting = true;
		try {
			await portalApi.signUp(txContext.tx_id, {
				csrf_token: txContext.csrf_token,
				display_name: displayName,
				email,
				password
			});
			step = 'otp';
		} catch (err) {
			const msg = err instanceof Error ? err.message : 'Registration failed.';
			errorMessage = msg;
			emailCollision = isGlobalEmailCollisionError(msg);
		} finally {
			submitting = false;
		}
	}

	async function handleVerifyOtp(e: SubmitEvent) {
		e.preventDefault();
		if (!txContext) return;
		errorMessage = '';
		submitting = true;
		try {
			const res = await portalApi.verifySignUp(txContext.tx_id, {
				csrf_token: txContext.csrf_token,
				email,
				code: otpCode
			});
			if (res.redirect_url) {
				window.location.href = res.redirect_url;
			}
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Verification failed.';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>Create account for {brand.appName}</title>
</svelte:head>

{#if loading}
	<p class="muted">Loading registration session...</p>
{:else if !txContext}
	<div class="alert-error" role="alert">{errorMessage || 'Invalid registration link.'}</div>
{:else}
	<div class="portal-header">
		<h1>Create account for {brand.appName}</h1>
		<p class="muted">
			{#if step === 'form'}
				One central identity works securely across your connected apps.
			{:else}
				Enter the 6-digit verification code sent to {email}.
			{/if}
		</p>
	</div>

	{#if errorMessage}
		<div class="alert-error" role="alert">
			<p>{errorMessage}</p>
			{#if emailCollision}
				<a
					class="collision-link"
					href={`/portal/login?tx=${encodeURIComponent(txContext.tx_id)}&email=${encodeURIComponent(email)}`}
				>
					Sign in with your existing account instead →
				</a>
			{/if}
		</div>
	{/if}

	{#if step === 'form'}
		<form class="portal-form" onsubmit={handleSignUp}>
			<label class="field">
				<span>Display name</span>
				<input type="text" bind:value={displayName} required placeholder="Alice Carter" />
			</label>
			<label class="field">
				<span>Email address</span>
				<input type="email" bind:value={email} required autocomplete="email" />
			</label>
			<label class="field">
				<span>Password (min 8 characters)</span>
				<input type="password" bind:value={password} required minlength="8" autocomplete="new-password" />
			</label>
			<button
				type="submit"
				class="btn-primary"
				style={`background-color: ${brand.primaryColor}`}
				disabled={submitting}
			>
				{submitting ? 'Sending code...' : 'Continue'}
			</button>
		</form>
	{:else}
		<form class="portal-form" onsubmit={handleVerifyOtp}>
			<label class="field">
				<span>6-digit verification code</span>
				<input type="text" bind:value={otpCode} required maxlength="6" inputmode="numeric" />
			</label>
			<button
				type="submit"
				class="btn-primary"
				style={`background-color: ${brand.primaryColor}`}
				disabled={submitting}
			>
				{submitting ? 'Verifying...' : `Verify & continue to ${brand.appName}`}
			</button>
		</form>
	{/if}

	<div class="portal-links">
		<a href={`/portal/login?tx=${encodeURIComponent(txContext.tx_id)}`}>Already have an account? Sign in</a>
	</div>
{/if}

<style>
	.portal-header {
		text-align: center;
		margin-bottom: 1.5rem;
	}
	h1 {
		font-size: 1.35rem;
		font-weight: 700;
		margin: 0 0 0.4rem;
	}
	.muted {
		color: #94a3b8;
		font-size: 0.9rem;
		margin: 0;
	}
	.alert-error {
		background: rgba(239, 68, 68, 0.14);
		border: 1px solid rgba(239, 68, 68, 0.4);
		color: #fecaca;
		padding: 0.75rem 1rem;
		border-radius: 10px;
		margin-bottom: 1rem;
		font-size: 0.875rem;
	}
	.alert-error p {
		margin: 0;
	}
	.collision-link {
		display: inline-block;
		margin-top: 0.5rem;
		color: #fde68a;
		font-weight: 600;
		text-decoration: underline;
	}
	.portal-form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		font-size: 0.85rem;
		color: #cbd5e1;
	}
	input {
		padding: 0.7rem 0.85rem;
		border-radius: 10px;
		border: 1px solid rgba(148, 163, 184, 0.28);
		background: rgba(15, 23, 42, 0.75);
		color: #f8fafc;
		font-size: 0.95rem;
	}
	.btn-primary {
		padding: 0.78rem 1rem;
		border: none;
		border-radius: 10px;
		color: #ffffff;
		font-weight: 600;
		font-size: 0.95rem;
		cursor: pointer;
	}
	.portal-links {
		margin-top: 1.5rem;
		text-align: center;
		font-size: 0.85rem;
	}
	.portal-links a {
		color: #a5b4fc;
		text-decoration: none;
	}
</style>

