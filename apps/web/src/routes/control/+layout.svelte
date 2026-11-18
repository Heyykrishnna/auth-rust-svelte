<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import Button from '$lib/components/Button.svelte';
	import CopyButton from '$lib/components/CopyButton.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import { ApiError } from '$lib/api/client';
	import {
		controlStore,
		dismissOneTimeSecret,
		setDeveloper,
		setWorkspaces
	} from '$lib/stores/control.svelte';

	let { children }: { children: import('svelte').Snippet } = $props();
	let initializing = $state(true);
	let loadError = $state('');

	async function hydrateControlPlane() {
		initializing = true;
		loadError = '';
		try {
			const [developer, workspaces] = await Promise.all([
				controlApi.me(),
				controlApi.listWorkspaces()
			]);
			setDeveloper(developer);
			setWorkspaces(workspaces);
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) {
				setDeveloper(null);
				await goto('/control/login', { replace: true });
			} else {
				loadError =
					err instanceof Error
						? err.message
						: 'Could not load your workspaces.';
			}
		} finally {
			initializing = false;
		}
	}

	onMount(() => {
		if (page.url.pathname !== '/control/login') void hydrateControlPlane();
		else initializing = false;
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
	<div
		class="flex min-h-screen items-center justify-center bg-[#0b0c0e] text-sm text-zinc-400"
	>
		<span role="status">Loading your workspaces…</span>
	</div>
{:else if loadError}
	<div
		class="workspace-theme workspace-accent grid min-h-dvh content-center justify-items-center gap-4 bg-brand-surface p-6 text-brand-text-primary"
	>
		<p role="alert">{loadError}</p>
		<Button onclick={hydrateControlPlane}>Try again</Button>
	</div>
{:else}
	{@render children()}
{/if}

<div class="workspace-theme workspace-accent">
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
			<div class="mt-4 flex flex-col gap-3.5">
				{#if controlStore.revealedSecret.publishableKey}
					<div
						class="flex flex-col gap-1.5 rounded-lg border border-[#23262e] bg-[#0b0c0e] p-3.5"
					>
						<span
							class="text-[11px] font-semibold tracking-wider text-zinc-400 uppercase"
						>
							Publishable Key
						</span>
						<div class="flex items-center justify-between gap-2.5">
							<code class="font-mono text-xs break-all text-white">
								{controlStore.revealedSecret.publishableKey}
							</code>
							<CopyButton
								label="Copy publishable key"
								onclick={() =>
									copyText(
										controlStore.revealedSecret?.publishableKey ?? '',
										'Publishable key'
									)}
							/>
						</div>
					</div>
				{/if}

				<div
					class="flex flex-col gap-1.5 rounded-lg border border-[#4b3d18] bg-[#0b0c0e] p-3.5"
				>
					<span
						class="text-[11px] font-semibold tracking-wider text-zinc-400 uppercase"
					>
						Secret Value (Shown Once)
					</span>
					<div class="flex items-center justify-between gap-2.5">
						<code class="font-mono text-xs break-all text-white">
							{controlStore.revealedSecret.secretKey}
						</code>
						<CopyButton
							label="Copy secret value"
							onclick={() =>
								copyText(
									controlStore.revealedSecret?.secretKey ?? '',
									'Secret value'
								)}
						/>
					</div>
				</div>
			</div>
		{/if}
	</Modal>
</div>
