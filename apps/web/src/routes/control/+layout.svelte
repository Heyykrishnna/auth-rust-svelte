<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import Button from '$lib/components/Button.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import {
		controlStore,
		dismissOneTimeSecret,
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
{:else if initializing}
	<div class="boot-screen">
		<span>Loading Developer Control Plane...</span>
	</div>
{:else}
	{@render children()}
{/if}

<Modal
	open={controlStore.revealedSecret !== null}
	title={controlStore.revealedSecret?.title ?? 'One-Time Secret Key'}
	description="Copy and store this secret now. For security, it will never be displayed again after closing this dialog."
	confirmText="I Have Saved This Secret"
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
				<span class="key-label">Secret Value (Shown Once)</span>
				<div class="key-value-row">
					<code>{controlStore.revealedSecret.secretKey}</code>
					<Button
						variant="primary"
						size="sm"
						onclick={() => copyText(controlStore.revealedSecret?.secretKey ?? '', 'Secret value')}
					>
						Copy
					</Button>
				</div>
			</div>
		</div>
	{/if}
</Modal>

<style>
	.boot-screen {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: #0b0c0e;
		color: #71717a;
		font-size: 0.88rem;
	}

	.secret-reveal-stack {
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
		margin-top: 1rem;
	}

	.key-box {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		padding: 0.85rem;
		border-radius: 8px;
		background-color: #0b0c0e;
		border: 1px solid #23262e;
	}

	.key-box-secret {
		border-color: #4b3d18;
	}

	.key-label {
		font-size: 0.72rem;
		font-weight: 600;
		color: #a1a1aa;
		text-transform: uppercase;
	}

	.key-value-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.65rem;
	}

	.key-value-row code {
		font-size: 0.78rem;
		word-break: break-all;
		color: #ffffff;
	}
</style>
