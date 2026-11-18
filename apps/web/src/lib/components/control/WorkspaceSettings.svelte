<script lang="ts">
	import { controlApi } from '$lib/api/control';
	import type { WorkspaceSummary } from '$lib/api/types';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { canManageTeam } from '$lib/control/permissions';
	import {
		prepareWorkspaceLogo,
		workspaceLogoAccept
	} from '$lib/control/workspace-logo';
	import WorkspaceAvatar from './WorkspaceAvatar.svelte';

	let {
		workspace,
		onUpdated
	}: {
		workspace: WorkspaceSummary;
		onUpdated: (workspace: WorkspaceSummary) => void;
	} = $props();

	let draft = $state<string | undefined>();
	let nameError = $state('');
	let logoError = $state('');
	let pending = $state<'name' | 'logo' | 'remove'>();
	let fileInput = $state<HTMLInputElement>();
	const name = $derived(draft ?? workspace.name);
	const canEdit = $derived(canManageTeam(workspace.role));
	const dirty = $derived(name.trim() !== workspace.name);

	async function saveName(event: SubmitEvent) {
		event.preventDefault();
		if (!canEdit || !dirty || pending) return;
		pending = 'name';
		nameError = '';
		try {
			onUpdated(
				await controlApi.updateWorkspace(workspace.id, { name: name.trim() })
			);
			draft = undefined;
			addToast('Workspace name saved', 'success');
		} catch (err) {
			nameError =
				err instanceof Error
					? err.message
					: 'Could not save the workspace name.';
		} finally {
			pending = undefined;
		}
	}

	async function replaceLogo(
		event: Event & { currentTarget: HTMLInputElement }
	) {
		const file = event.currentTarget.files?.[0];
		event.currentTarget.value = '';
		if (!file || !canEdit || pending) return;
		pending = 'logo';
		logoError = '';
		try {
			const jpeg = await prepareWorkspaceLogo(file);
			onUpdated(
				await controlApi.updateWorkspace(workspace.id, {
					logo: { action: 'replace', jpeg }
				})
			);
			addToast('Workspace logo saved', 'success');
		} catch (err) {
			logoError =
				err instanceof Error
					? err.message
					: 'Could not upload the workspace logo.';
		} finally {
			pending = undefined;
		}
	}

	async function removeLogo() {
		if (!workspace.logo_version || !canEdit || pending) return;
		pending = 'remove';
		logoError = '';
		try {
			onUpdated(
				await controlApi.updateWorkspace(workspace.id, {
					logo: { action: 'remove' }
				})
			);
			addToast('Workspace logo removed', 'success');
		} catch (err) {
			logoError =
				err instanceof Error
					? err.message
					: 'Could not remove the workspace logo.';
		} finally {
			pending = undefined;
		}
	}

	async function copyId() {
		try {
			await navigator.clipboard.writeText(workspace.id);
			addToast('Workspace ID copied', 'success');
		} catch {
			addToast(
				'Could not copy the workspace ID. Select the ID to copy it manually.',
				'warning'
			);
		}
	}
</script>

<header class="border-b border-brand-border pb-4">
	<h1 class="font-heading text-2xl font-bold tracking-tight sm:text-3xl">
		Settings
	</h1>
</header>

<section
	class="min-w-0 task-surface"
	aria-labelledby="workspace-profile-heading"
>
	<header
		class="rounded-t-xl border-b border-brand-border bg-brand-surface-raised px-4 py-4 sm:px-6"
	>
		<h2 class="font-heading text-lg font-bold" id="workspace-profile-heading">
			Workspace profile
		</h2>
		{#if !canEdit}
			<p class="mt-1 text-sm text-brand-text-secondary">
				Owners and admins can edit the workspace profile.
			</p>
		{/if}
	</header>

	<div class="divide-y divide-brand-border">
		<div
			class="grid min-w-0 gap-3 px-4 py-5 sm:grid-cols-[11rem_minmax(0,1fr)] sm:gap-6 sm:px-6 sm:py-6"
		>
			<div>
				<h3 class="text-sm font-semibold">Logo</h3>
				{#if canEdit}
					<p
						class="mt-1 text-xs leading-relaxed text-brand-text-secondary"
						id="workspace-logo-hint"
					>
						Square PNG, JPEG, or WebP. Up to 5 MB.
					</p>
				{/if}
			</div>
			<div class="grid min-w-0 gap-2">
				<div class="flex flex-wrap items-center gap-3 sm:gap-4">
					<WorkspaceAvatar
						{workspace}
						alt="Workspace logo"
						class="size-20 text-2xl"
					/>
					{#if canEdit}
						<div class="flex flex-wrap items-center gap-1.5">
							<input
								bind:this={fileInput}
								type="file"
								accept={workspaceLogoAccept}
								onchange={replaceLogo}
								aria-label="Workspace logo file"
								aria-describedby="workspace-logo-hint"
								disabled={Boolean(pending)}
								hidden
							/>
							<Button
								size="sm"
								onclick={() => fileInput?.click()}
								disabled={Boolean(pending)}
								loading={pending === 'logo'}
							>
								{#if pending !== 'logo'}
									<svg
										class="size-4"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="1.7"
										stroke-linecap="round"
										stroke-linejoin="round"
										aria-hidden="true"
									>
										<path
											d="M12 16V4m-4 4 4-4 4 4M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"
										/>
									</svg>
								{/if}
								{workspace.logo_version ? 'Replace image' : 'Upload image'}
							</Button>
							{#if workspace.logo_version}
								<Button
									size="sm"
									onclick={removeLogo}
									disabled={Boolean(pending)}
									loading={pending === 'remove'}
								>
									Remove
								</Button>
							{/if}
						</div>
					{/if}
				</div>
				{#if logoError}
					<p class="text-sm text-brand-danger-text" role="alert">{logoError}</p>
				{/if}
			</div>
		</div>

		<form
			class="grid min-w-0 gap-3 px-4 py-5 sm:grid-cols-[11rem_minmax(0,1fr)] sm:gap-6 sm:px-6 sm:py-6"
			onsubmit={saveName}
		>
			<label
				for="workspace-profile-name"
				class="text-sm font-semibold sm:pt-2.5"
			>
				Workspace name
			</label>
			<div class="grid max-w-lg min-w-0 gap-3">
				<input
					id="workspace-profile-name"
					class="min-h-10 min-w-0 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 py-2 text-sm text-brand-text-primary outline-none focus:border-brand-lime focus:ring-1 focus:ring-brand-lime"
					value={name}
					oninput={(event) => (draft = event.currentTarget.value)}
					readonly={!canEdit || Boolean(pending)}
					required
					minlength="2"
					maxlength="120"
				/>
				{#if nameError}
					<p class="text-sm text-brand-danger-text" role="alert">{nameError}</p>
				{/if}
				{#if canEdit && dirty}
					<div class="flex flex-wrap justify-end gap-2">
						<Button
							size="sm"
							disabled={Boolean(pending)}
							onclick={() => {
								draft = undefined;
								nameError = '';
							}}
						>
							Discard
						</Button>
						<Button
							size="sm"
							type="submit"
							disabled={Boolean(pending)}
							loading={pending === 'name'}
						>
							Save changes
						</Button>
					</div>
				{/if}
			</div>
		</form>

		<div
			class="grid min-w-0 gap-3 px-4 py-5 sm:grid-cols-[11rem_minmax(0,1fr)] sm:gap-6 sm:px-6 sm:py-6"
		>
			<div>
				<h3 class="text-sm font-semibold">Workspace ID</h3>
				<p class="mt-1 text-xs leading-relaxed text-brand-text-secondary">
					Use this ID in Signet API requests.
				</p>
			</div>
			<div class="grid max-w-lg min-w-0 gap-3">
				<div
					class="flex min-h-10 min-w-0 items-center gap-2 rounded-lg border border-brand-border-hover bg-brand-surface-input pl-3"
				>
					<code
						class="min-w-0 flex-1 text-xs break-all text-brand-text-primary select-all"
					>
						{workspace.id}
					</code>
					<button
						type="button"
						class="action-button inline-grid size-8 shrink-0 place-items-center rounded-md mr-1.5"
						aria-label="Copy workspace ID"
						title="Copy workspace ID"
						onclick={copyId}
					>
						<svg
							class="size-4"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="1.7"
							stroke-linecap="round"
							stroke-linejoin="round"
							aria-hidden="true"
						>
							<rect x="8" y="8" width="12" height="12" rx="2" /><path
								d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"
							/>
						</svg>
					</button>
				</div>
				<div
					class="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-1 text-xs text-brand-text-secondary"
				>
					<span>Slug</span>
					<code class="min-w-0 break-all">{workspace.slug}</code>
				</div>
			</div>
		</div>
	</div>
</section>
