<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import AuthSplitShell from '$lib/components/AuthSplitShell.svelte';
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
	<title>Authorize {brand.appName}</title>
</svelte:head>

<AuthSplitShell
	brandName={brand.appName}
	logoUrl={brand.logoUrl}
	footerText="Central Identity Provider · Application Authorization"
	topActionLabel={txContext ? 'Switch account' : undefined}
	topActionHref={txContext ? `/portal/login?tx=${encodeURIComponent(txContext.tx_id)}` : undefined}
>
	{#if loading}
		<p class="editorial-subtitle">Loading requested permissions...</p>
	{:else if !txContext}
		<h1 class="editorial-title">Authorize</h1>
		<div class="editorial-alert" role="alert">{errorMessage || 'Invalid consent link.'}</div>
	{:else}
		<h1 class="editorial-title">Authorize</h1>
		<p class="editorial-subtitle">
			{brand.appName} requests access
			{#if txContext.active_user}
				for <strong>{txContext.active_user.email}</strong>
			{/if}
		</p>

		{#if errorMessage}
			<div class="editorial-alert" role="alert">{errorMessage}</div>
		{/if}

		<ul class="list-none p-0 m-0 border-t border-[#23262e]">
			{#each scopeDescriptions as item}
				<li class="flex flex-col gap-1 py-3.5 border-b border-[#23262e]">
					<strong class="text-sm font-medium text-white">{item.title}</strong>
					<span class="text-xs text-zinc-400">{item.description}</span>
				</li>
			{/each}
		</ul>

		<div class="editorial-actions-footer">
			<button
				type="button"
				class="editorial-oauth-btn"
				disabled={submitting}
				onclick={() => decide(false)}
			>
				Deny access
			</button>

			<button
				type="button"
				class="editorial-circle-cta"
				disabled={submitting}
				onclick={() => decide(true)}
			>
				{submitting ? '...' : 'ALLOW'}
			</button>
		</div>
	{/if}
</AuthSplitShell>
