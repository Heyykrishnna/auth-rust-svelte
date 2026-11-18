<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { portalApi, type PortalTransactionContext } from '$lib/api/portal';
	import { resolvePortalBrandTheme } from '$lib/portal/state';

	let txContext = $state<PortalTransactionContext | null>(null);
	let loading = $state(true);
	let submitting = $state(false);
	let errorMessage = $state('');
	let email = $state('');
	let password = $state('');

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
			const prefillEmail = page.url.searchParams.get('email');
			if (prefillEmail) {
				email = prefillEmail;
			}
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Unable to load sign-in session.';
		} finally {
			loading = false;
		}
	});

	async function handleLogin(e: SubmitEvent) {
		e.preventDefault();
		if (!txContext) return;
		errorMessage = '';
		submitting = true;
		try {
			const res = await portalApi.login(txContext.tx_id, {
				csrf_token: txContext.csrf_token,
				email,
				password
			});
			if (res.action === 'consent') {
				await goto(`/portal/consent?tx=${encodeURIComponent(txContext.tx_id)}`);
			} else if (res.redirect_url) {
				window.location.href = res.redirect_url;
			}
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Sign-in failed.';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>Sign in to {brand.appName}</title>
</svelte:head>

{#if loading}
	<p class="muted">Loading sign-in request...</p>
{:else if !txContext}
	<div class="alert-error" role="alert">{errorMessage || 'Invalid sign-in link.'}</div>
{:else}
	<div class="portal-header">
		{#if brand.logoUrl}
			<img src={brand.logoUrl} alt={brand.appName} class="brand-logo" />
		{/if}
		<h1>Sign in to {brand.appName}</h1>
		<p class="muted">Use your central account to continue to {txContext.application_name}.</p>
	</div>

	{#if errorMessage}
		<div class="alert-error" role="alert">{errorMessage}</div>
	{/if}

	<form class="portal-form" onsubmit={handleLogin}>
		<label class="field">
			<span>Email address</span>
			<input type="email" bind:value={email} required autocomplete="email" placeholder="you@example.com" />
		</label>

		<label class="field">
			<span>Password</span>
			<input type="password" bind:value={password} required autocomplete="current-password" />
		</label>

		<button
			type="submit"
			class="btn-primary"
			style={`background-color: ${brand.primaryColor}`}
			disabled={submitting}
		>
			{submitting ? 'Signing in...' : `Continue to ${brand.appName}`}
		</button>
	</form>

	{#if txContext.social_providers.length > 0}
		<div class="divider"><span>or continue with</span></div>
		<div class="social-row">
			{#each txContext.social_providers as provider}
				<span class="social-chip">{provider}</span>
			{/each}
		</div>
	{/if}

	<div class="portal-links">
		<a href={`/portal/sign-up?tx=${encodeURIComponent(txContext.tx_id)}`}>Create an account</a>
		<a href="/portal/logout">Manage SSO sessions</a>
	</div>
{/if}

<style>
	.portal-header {
		text-align: center;
		margin-bottom: 1.5rem;
	}
	.brand-logo {
		max-height: 40px;
		margin: 0 auto 0.75rem;
	}
	h1 {
		font-size: 1.4rem;
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
	.btn-primary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.divider {
		text-align: center;
		margin: 1.25rem 0 0.75rem;
		font-size: 0.78rem;
		color: #64748b;
	}
	.social-row {
		display: flex;
		justify-content: center;
		gap: 0.5rem;
	}
	.social-chip {
		text-transform: capitalize;
		border: 1px solid rgba(148, 163, 184, 0.25);
		padding: 0.35rem 0.8rem;
		border-radius: 999px;
		font-size: 0.8rem;
		color: #e2e8f0;
	}
	.portal-links {
		display: flex;
		justify-content: space-between;
		margin-top: 1.5rem;
		font-size: 0.85rem;
	}
	.portal-links a {
		color: #a5b4fc;
		text-decoration: none;
	}
</style>

