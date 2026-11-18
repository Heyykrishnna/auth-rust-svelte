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
	<div class="min-h-screen flex items-center justify-center bg-[#0b0c0e] text-zinc-400 text-sm">
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
		<div class="flex flex-col gap-3.5 mt-4">
			{#if controlStore.revealedSecret.publishableKey}
				<div class="flex flex-col gap-1.5 p-3.5 rounded-lg bg-[#0b0c0e] border border-[#23262e]">
					<span class="text-[11px] font-semibold text-zinc-400 uppercase tracking-wider">Publishable Key</span>
					<div class="flex items-center justify-between gap-2.5">
						<code class="text-xs font-mono break-all text-white">{controlStore.revealedSecret.publishableKey}</code>
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

			<div class="flex flex-col gap-1.5 p-3.5 rounded-lg bg-[#0b0c0e] border border-[#4b3d18]">
				<span class="text-[11px] font-semibold text-zinc-400 uppercase tracking-wider">Secret Value (Shown Once)</span>
				<div class="flex items-center justify-between gap-2.5">
					<code class="text-xs font-mono break-all text-white">{controlStore.revealedSecret.secretKey}</code>
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
