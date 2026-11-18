<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { portalApi, type PortalTransactionContext } from '$lib/api/portal';
	import { describeOidcScopes, resolvePortalBrandTheme } from '$lib/portal/state';

	let txContext = $state<PortalTransactionContext | null>(null);
	let loading = $state(true);
	let submitting = $state(false);
	let errorMessage = $state('');

	const txId = $derived(page.url.searchParams.get('tx') ?? '');
	const brand = $derived(
		txContext
			? resolvePortalBrandTheme(txContext.branding, txContext.application_name)
			: resolvePortalBrandTheme(null, 'Application')
	);
	const scopeDescriptions = $derived(txContext ? describeOidcScopes(txContext.scopes) : []);

	onMount(async () => {
		if (!txId) {
			errorMessage = 'Missing OIDC authorization transaction ID.';
			loading = false;
			return;
		}
		try {
			txContext = await portalApi.getTransaction(txId);
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Unable to load consent request.';
		} finally {
			loading = false;
		}
	});

	async function decide(approved: boolean) {
		if (!txContext) return;
		errorMessage = '';
		submitting = true;
		try {
			const res = await portalApi.submitConsent(txContext.tx_id, {
				csrf_token: txContext.csrf_token,
				approved
			});
			if (res.redirect_url) {
				window.location.href = res.redirect_url;
			}
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Unable to complete consent.';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>Continue to {brand.appName}</title>
</svelte:head>

{#if loading}
	<p class="muted">Loading application permissions...</p>
{:else if !txContext}
	<div class="alert-error" role="alert">{errorMessage || 'Invalid consent link.'}</div>
{:else}
	<div class="portal-header">
		<h1>Continue to {brand.appName}?</h1>
		{#if txContext.active_user}
			<p class="muted">
				Signed in as <strong>{txContext.active_user.email}</strong>
			</p>
		{/if}
	</div>

	{#if errorMessage}
		<div class="alert-error" role="alert">{errorMessage}</div>
	{/if}

	<div class="scope-box">
		<p class="scope-heading">{brand.appName} will receive access to:</p>
		<ul>
			{#each scopeDescriptions as item}
				<li>
					<strong>{item.title}</strong>
					<span>{item.description}</span>
				</li>
			{/each}
		</ul>
	</div>

	<div class="actions">
		<button
			type="button"
			class="btn-primary"
			style={`background-color: ${brand.primaryColor}`}
			disabled={submitting}
			onclick={() => decide(true)}
		>
			{submitting ? 'Authorizing...' : `Allow & continue to ${brand.appName}`}
		</button>
		<button type="button" class="btn-secondary" disabled={submitting} onclick={() => decide(false)}>
			Cancel
		</button>
	</div>

	<div class="portal-links">
		<a href={`/portal/login?tx=${encodeURIComponent(txContext.tx_id)}`}>Switch account</a>
	</div>
{/if}

<style>
	.portal-header {
		text-align: center;
		margin-bottom: 1.25rem;
	}
	h1 {
		font-size: 1.35rem;
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
	.scope-box {
		background: rgba(15, 23, 42, 0.65);
		border: 1px solid rgba(148, 163, 184, 0.2);
		border-radius: 12px;
		padding: 1rem;
		margin-bottom: 1.25rem;
	}
	.scope-heading {
		margin: 0 0 0.75rem;
		font-size: 0.85rem;
		color: #cbd5e1;
	}
	ul {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	li {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	li strong {
		font-size: 0.9rem;
		color: #f8fafc;
	}
	li span {
		font-size: 0.8rem;
		color: #94a3b8;
	}
	.actions {
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
	}
	.btn-primary {
		padding: 0.78rem 1rem;
		border: none;
		border-radius: 10px;
		color: #ffffff;
		font-weight: 600;
		cursor: pointer;
	}
	.btn-secondary {
		padding: 0.72rem 1rem;
		border-radius: 10px;
		border: 1px solid rgba(148, 163, 184, 0.3);
		background: transparent;
		color: #e2e8f0;
		cursor: pointer;
	}
	.portal-links {
		margin-top: 1.25rem;
		text-align: center;
		font-size: 0.85rem;
	}
	.portal-links a {
		color: #a5b4fc;
		text-decoration: none;
	}
</style>

