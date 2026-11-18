<script lang="ts">
	import { onMount } from 'svelte';
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
			if (res.global_logout) {
				account = null;
			} else {
				await loadAccount();
			}
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
	<title>Central SSO Account & Sign Out</title>
</svelte:head>

<div class="portal-header">
	<h1>Central SSO Account</h1>
	<p class="muted">Manage active IdP sessions and connected applications.</p>
</div>

{#if message}
	<div class="alert-info" role="status">{message}</div>
{/if}
{#if errorMessage}
	<div class="alert-error" role="alert">{errorMessage}</div>
{/if}

{#if loading}
	<p class="muted">Checking active SSO session...</p>
{:else if account}
	<div class="identity-banner">
		<strong>{account.user.display_name}</strong>
		<span>{account.user.email}</span>
	</div>

	<section class="section">
		<h2>Connected Applications ({account.connected_apps.length})</h2>
		{#if account.connected_apps.length === 0}
			<p class="muted">No connected applications yet.</p>
		{:else}
			<ul class="list">
				{#each account.connected_apps as app}
					<li>
						<div>
							<strong>{app.application_name}</strong>
							<span class="sub">{app.environment_kind}</span>
						</div>
						<span class="pill">{app.active_sessions} active</span>
					</li>
				{/each}
			</ul>
		{/if}
	</section>

	<section class="section">
		<h2>Active SSO Sessions ({account.sso_sessions.length})</h2>
		<ul class="list">
			{#each account.sso_sessions as sess}
				<li>
					<div>
						<strong>{sess.is_current ? 'Current browser' : 'Other device'}</strong>
						<span class="sub">{sess.ip_address ?? 'Unknown IP'}</span>
					</div>
					<button type="button" class="btn-link" disabled={busy} onclick={() => handleRevokeSso(sess.id)}>
						Revoke
					</button>
				</li>
			{/each}
		</ul>
	</section>

	<div class="actions">
		<button type="button" class="btn-danger" disabled={busy} onclick={() => handleSignOut(true)}>
			{busy ? 'Signing out...' : 'Sign out of all applications'}
		</button>
	</div>
{/if}

<style>
	.portal-header {
		text-align: center;
		margin-bottom: 1.25rem;
	}
	h1 {
		font-size: 1.35rem;
		margin: 0 0 0.35rem;
	}
	h2 {
		font-size: 0.9rem;
		color: #cbd5e1;
		margin: 0 0 0.6rem;
	}
	.muted {
		color: #94a3b8;
		font-size: 0.88rem;
		margin: 0;
	}
	.identity-banner {
		display: flex;
		flex-direction: column;
		padding: 0.85rem 1rem;
		border-radius: 10px;
		background: rgba(99, 102, 241, 0.14);
		border: 1px solid rgba(99, 102, 241, 0.35);
		margin-bottom: 1.25rem;
	}
	.identity-banner span {
		font-size: 0.82rem;
		color: #c7d2fe;
	}
	.section {
		margin-bottom: 1.25rem;
	}
	.list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.list li {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.65rem 0.85rem;
		border-radius: 10px;
		background: rgba(15, 23, 42, 0.7);
		border: 1px solid rgba(148, 163, 184, 0.18);
	}
	.list li div {
		display: flex;
		flex-direction: column;
	}
	.sub {
		font-size: 0.76rem;
		color: #94a3b8;
	}
	.pill {
		font-size: 0.75rem;
		padding: 0.2rem 0.55rem;
		border-radius: 999px;
		background: rgba(16, 185, 129, 0.15);
		color: #6ee7b7;
	}
	.btn-link {
		background: transparent;
		border: none;
		color: #fca5a5;
		cursor: pointer;
		font-size: 0.82rem;
	}
	.actions {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.btn-danger {
		padding: 0.78rem 1rem;
		border-radius: 10px;
		border: none;
		background: #dc2626;
		color: #ffffff;
		font-weight: 600;
		cursor: pointer;
	}
	.alert-info {
		background: rgba(16, 185, 129, 0.15);
		border: 1px solid rgba(16, 185, 129, 0.4);
		color: #a7f3d0;
		padding: 0.75rem 1rem;
		border-radius: 10px;
		margin-bottom: 1rem;
		font-size: 0.875rem;
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
</style>

