<script lang="ts">
	import { onMount } from 'svelte';
	import AuthSplitShell from '$lib/components/AuthSplitShell.svelte';
	import { portalApi, type PortalAccountView } from '$lib/api/portal';

	let account = $state<PortalAccountView | null>(null);
	let loading = $state(true);
	let busy = $state(false);
	let message = $state('');
	let errorMessage = $state('');

	async function loadAccount() {
		loading = true;
		errorMessage = '';
		try {
			account = await portalApi.getAccount();
		} catch (err) {
			account = null;
			errorMessage = err instanceof Error ? err.message : 'Not signed in to Central IdP.';
		} finally {
			loading = false;
		}
	}

	onMount(loadAccount);

	async function handleSignOut(global: boolean) {
		busy = true;
		errorMessage = '';
		try {
			const res = await portalApi.logout({ global });
			message = res.global_logout
				? `Signed out everywhere (${res.revoked_app_sessions} app sessions revoked).`
				: 'Local sign-out completed.';
			if (res.global_logout) account = null;
			else await loadAccount();
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Sign-out failed.';
		} finally {
			busy = false;
		}
	}

	async function handleRevokeSso(ssoId: string) {
		busy = true;
		errorMessage = '';
		try {
			await portalApi.revokeSsoSession(ssoId);
			await loadAccount();
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Failed to revoke SSO session.';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head>
	<title>SSO Account — Secure Auth</title>
</svelte:head>

<AuthSplitShell
	brandName="Secure Auth"
	footerText="Central Identity Provider · Session Control"
	topActionLabel="Control Plane"
	topActionHref="/control"
>
	<h1 class="editorial-title">SSO Sessions</h1>
	<p class="editorial-subtitle">Inspect connected applications and revoke active sessions.</p>

	{#if message}
		<div class="status-box" role="status">{message}</div>
	{/if}
	{#if errorMessage}
		<div class="editorial-alert" role="alert">{errorMessage}</div>
	{/if}

	{#if loading}
		<p class="editorial-subtitle">Checking active SSO session...</p>
	{:else if account}
		<div class="account-meta">
			<strong>{account.user.display_name}</strong>
			<span>{account.user.email}</span>
		</div>

		<div class="block">
			<h2 class="block-title">Connected Applications ({account.connected_apps.length})</h2>
			{#if account.connected_apps.length === 0}
				<p class="quiet">No connected applications.</p>
			{:else}
				<ul class="item-list">
					{#each account.connected_apps as app}
						<li>
							<div>
								<strong>{app.application_name}</strong>
								<span>{app.environment_kind}</span>
							</div>
							<span class="pill">{app.active_sessions} active</span>
						</li>
					{/each}
				</ul>
			{/if}
		</div>

		<div class="block">
			<h2 class="block-title">Active SSO Sessions ({account.sso_sessions.length})</h2>
			<ul class="item-list">
				{#each account.sso_sessions as sess}
					<li>
						<div>
							<strong>{sess.is_current ? 'Current browser' : 'Other device'}</strong>
							<span>{sess.ip_address ?? 'Unknown IP'}</span>
						</div>
						<button type="button" class="revoke-btn" disabled={busy} onclick={() => handleRevokeSso(sess.id)}>
							Revoke
						</button>
					</li>
				{/each}
			</ul>
		</div>

		<div class="editorial-actions-footer">
			<span class="quiet">Global sign-out revokes all app sessions</span>
			<button type="button" class="editorial-circle-cta" disabled={busy} onclick={() => handleSignOut(true)}>
				{busy ? '...' : 'SIGN OUT'}
			</button>
		</div>
	{/if}
</AuthSplitShell>

<style>
	.status-box {
		padding: 0.75rem 0.9rem;
		border: 1px solid #1d442e;
		background-color: #11241a;
		color: #4ade80;
		border-radius: 6px;
		font-size: 0.82rem;
		margin-bottom: 1.25rem;
	}

	.account-meta {
		display: flex;
		flex-direction: column;
		padding-bottom: 1rem;
		margin-bottom: 1.25rem;
		border-bottom: 1px solid #23262e;
	}

	.account-meta strong {
		color: #ffffff;
		font-size: 0.95rem;
	}

	.account-meta span {
		color: #71717a;
		font-size: 0.8rem;
	}

	.block {
		margin-bottom: 1.25rem;
	}

	.block-title {
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: #71717a;
		margin-bottom: 0.55rem;
	}

	.item-list {
		list-style: none;
		padding: 0;
		margin: 0;
		border-top: 1px solid #23262e;
	}

	.item-list li {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.65rem 0;
		border-bottom: 1px solid #23262e;
	}

	.item-list li div {
		display: flex;
		flex-direction: column;
	}

	.item-list li strong {
		font-size: 0.85rem;
		color: #f4f4f5;
	}

	.item-list li span {
		font-size: 0.75rem;
		color: #71717a;
	}

	.pill {
		font-size: 0.72rem;
		color: #d4d4d8;
		border: 1px solid #2e2e34;
		padding: 0.15rem 0.5rem;
		border-radius: 4px;
	}

	.revoke-btn {
		background: transparent;
		border: 1px solid #572327;
		color: #f87171;
		border-radius: 4px;
		padding: 0.25rem 0.6rem;
		font-size: 0.75rem;
		cursor: pointer;
	}

	.quiet {
		font-size: 0.78rem;
		color: #71717a;
	}
</style>
