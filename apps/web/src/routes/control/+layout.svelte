<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import {
		controlStore,
		dismissOneTimeSecret,
		selectApplication,
		selectEnvironmentKind,
		selectWorkspace,
		setApplications,
		setDeveloper,
		setWorkspaces
	} from '$lib/stores/control.svelte';

	let { children }: { children: import('svelte').Snippet } = $props();
	let initializing = $state(true);

	async function hydrateControlPlane() {
		initializing = true;
		try {
			const dev = await controlApi.me();
			setDeveloper(dev);
			const workspaces = await controlApi.listWorkspaces();
			setWorkspaces(workspaces);
			if (workspaces[0]) {
				const apps = await controlApi.listApplications(workspaces[0].id);
				setApplications(apps);
			}
		} catch {
			setDeveloper(null);
			if (!page.url.pathname.startsWith('/control/login')) {
				goto('/control/login');
			}
		} finally {
			initializing = false;
		}
	}

	onMount(() => {
		hydrateControlPlane();
	});

	async function handleWorkspaceChange(e: Event) {
		const id = (e.currentTarget as HTMLSelectElement).value;
		selectWorkspace(id);
		try {
			const apps = await controlApi.listApplications(id);
			setApplications(apps);
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load workspace apps', 'error');
		}
	}

	async function handleSignOut() {
		try {
			await controlApi.signOut();
		} finally {
			setDeveloper(null);
			addToast('Signed out of Developer Control Plane', 'info');
			goto('/control/login');
		}
	}

	async function copyText(text: string, label: string) {
		try {
			await navigator.clipboard.writeText(text);
			addToast(`${label} copied to clipboard`, 'success');
		} catch {
			addToast('Copy failed — please select and copy manually', 'warning');
		}
	}
</script>

{#if page.url.pathname.startsWith('/control/login')}
	{@render children()}
{:else}
	<div class="control-shell container">
		<div class="control-topbar">
			<div class="topbar-left">
				<span class="plane-label">Developer Control Plane</span>
				{#if controlStore.isAuthenticated}
					<div class="switcher-group">
						<label class="switcher-item">
							<span class="switcher-caption">Workspace</span>
							<select
								aria-label="Workspace selector"
								value={controlStore.activeWorkspace?.id ?? ''}
								onchange={handleWorkspaceChange}
							>
								{#each controlStore.workspaces as ws}
									<option value={ws.id}>{ws.name} ({ws.role})</option>
								{/each}
							</select>
						</label>

						{#if controlStore.applications.length > 0}
							<label class="switcher-item">
								<span class="switcher-caption">Application</span>
								<select
									aria-label="Application selector"
									value={controlStore.activeApplication?.application.id ?? ''}
									onchange={(e) => selectApplication((e.currentTarget as HTMLSelectElement).value)}
								>
									{#each controlStore.applications as item}
										<option value={item.application.id}>{item.application.name}</option>
									{/each}
								</select>
							</label>
						{/if}

						<div class="env-toggle" role="group" aria-label="Environment switcher">
							<button
								type="button"
								class="env-btn"
								class:active-dev={controlStore.selectedEnvKind === 'development'}
								onclick={() => selectEnvironmentKind('development')}
							>
								Development
							</button>
							<button
								type="button"
								class="env-btn"
								class:active-prod={controlStore.selectedEnvKind === 'production'}
								onclick={() => selectEnvironmentKind('production')}
							>
								Production
							</button>
						</div>
					</div>
				{/if}
			</div>

			{#if controlStore.isAuthenticated && controlStore.developer}
				<div class="topbar-right">
					<Badge variant={controlStore.selectedEnvKind === 'production' ? 'success' : 'warning'}>
						{controlStore.selectedEnvKind === 'production' ? 'LIVE PROD' : 'TEST DEV'}
					</Badge>
					<span class="dev-email">{controlStore.developer.email}</span>
					<Button variant="ghost" size="sm" onclick={handleSignOut}>Sign Out</Button>
				</div>
			{/if}
		</div>

		{#if initializing}
			<div class="loading-box">Loading Developer Control Plane...</div>
		{:else}
			{@render children()}
		{/if}
	</div>
{/if}

<Modal
	open={controlStore.revealedSecret !== null}
	title={controlStore.revealedSecret?.title ?? 'One-Time Secret Key'}
	description="Copy and store this secret key now. For security, it will never be shown again after closing this dialog."
	confirmText="I Have Saved This Key"
	cancelText="Close"
	onconfirm={dismissOneTimeSecret}
	oncancel={dismissOneTimeSecret}
>
	{#if controlStore.revealedSecret}
		<div class="secret-reveal-stack">
			{#if controlStore.revealedSecret.publishableKey}
				<div class="key-box">
					<span class="key-label">Publishable Key</span>
					<div class="key-value-row">
						<code>{controlStore.revealedSecret.publishableKey}</code>
						<Button
							variant="outline"
							size="sm"
							onclick={() =>
								copyText(controlStore.revealedSecret?.publishableKey ?? '', 'Publishable key')}
						>
							Copy
						</Button>
					</div>
				</div>
			{/if}

			<div class="key-box key-box-secret">
				<span class="key-label">Secret Key (Shown Once)</span>
				<div class="key-value-row">
					<code>{controlStore.revealedSecret.secretKey}</code>
					<Button
						variant="primary"
						size="sm"
						onclick={() => copyText(controlStore.revealedSecret?.secretKey ?? '', 'Secret key')}
					>
						Copy
					</Button>
				</div>
			</div>
		</div>
	{/if}
</Modal>

<style>
	.container {
		max-width: 1200px;
		margin: 0 auto;
		padding: 1.5rem 1.5rem 4rem;
		width: 100%;
	}

	.control-topbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
		padding: 1rem 1.25rem;
		border-radius: 14px;
		background: rgba(15, 23, 42, 0.85);
		border: 1px solid rgba(255, 255, 255, 0.08);
		margin-bottom: 1.75rem;
	}

	.topbar-left,
	.topbar-right,
	.switcher-group {
		display: flex;
		align-items: center;
		gap: 0.85rem;
		flex-wrap: wrap;
	}

	.plane-label {
		font-weight: 700;
		font-size: 0.95rem;
		color: #e2e8f0;
		padding-right: 0.5rem;
		border-right: 1px solid rgba(255, 255, 255, 0.1);
	}

	.switcher-item {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.switcher-caption {
		font-size: 0.75rem;
		color: #94a3b8;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	select {
		background: rgba(30, 41, 59, 0.9);
		color: #f8fafc;
		border: 1px solid rgba(255, 255, 255, 0.14);
		border-radius: 8px;
		padding: 0.4rem 0.65rem;
		font-size: 0.875rem;
	}

	.env-toggle {
		display: inline-flex;
		background: rgba(15, 23, 42, 0.9);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 8px;
		padding: 2px;
	}

	.env-btn {
		background: transparent;
		border: none;
		color: #94a3b8;
		font-size: 0.8rem;
		font-weight: 600;
		padding: 0.35rem 0.7rem;
		border-radius: 6px;
		cursor: pointer;
	}

	.env-btn.active-dev {
		background: rgba(245, 158, 11, 0.2);
		color: #fbbf24;
	}

	.env-btn.active-prod {
		background: rgba(34, 197, 94, 0.2);
		color: #4ade80;
	}

	.dev-email {
		font-size: 0.85rem;
		color: #94a3b8;
	}

	.loading-box {
		padding: 3rem;
		text-align: center;
		color: #94a3b8;
	}

	.secret-reveal-stack {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin-top: 1rem;
	}

	.key-box {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		padding: 0.85rem;
		border-radius: 10px;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(255, 255, 255, 0.08);
	}

	.key-box-secret {
		border-color: rgba(245, 158, 11, 0.45);
		background: rgba(245, 158, 11, 0.06);
	}

	.key-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: #94a3b8;
		text-transform: uppercase;
	}

	.key-value-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.key-value-row code {
		font-size: 0.8rem;
		word-break: break-all;
		color: #f8fafc;
	}
</style>

