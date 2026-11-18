<script lang="ts">
	import { controlApi } from '$lib/api/control';
	import type { WorkspaceSummary } from '$lib/api/types';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { canManageTeam } from '$lib/control/permissions';
	import SectionHeader from './SectionHeader.svelte';

	let {
		workspace,
		onUpdated
	}: {
		workspace: WorkspaceSummary;
		onUpdated: (workspace: WorkspaceSummary) => void;
	} = $props();

	let draft = $state<string | undefined>();
	let error = $state('');
	let saving = $state(false);
	const name = $derived(draft ?? workspace.name);
	const canEdit = $derived(canManageTeam(workspace.role));
	const dirty = $derived(name.trim() !== workspace.name);

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (!canEdit || !dirty || saving) return;
		saving = true;
		error = '';
		try {
			const updated = await controlApi.updateWorkspace(workspace.id, {
				name: name.trim()
			});
			onUpdated(updated);
			draft = undefined;
			addToast('Workspace name saved', 'success');
		} catch (err) {
			error =
				err instanceof Error
					? err.message
					: 'Could not save workspace settings.';
		} finally {
			saving = false;
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

<SectionHeader
	title="Settings"
	description="Manage this workspace’s display name and view its identifiers."
/>

<section
	class="grid gap-5 rounded-xl border border-brand-border bg-brand-surface-card p-5 sm:grid-cols-[minmax(0,1fr)_minmax(0,1.5fr)] sm:p-6"
	aria-label="Workspace profile"
>
	<div>
		<h2 class="font-heading text-lg font-bold">Workspace profile</h2>
		<p class="mt-1 text-sm leading-relaxed text-brand-text-secondary">
			Your team sees this name in the Developer Console.
		</p>
	</div>
	<form class="grid min-w-0 gap-4" onsubmit={save}>
		<label class="grid gap-1.5 text-sm font-medium">
			Workspace name
			<input
				class="min-h-10 min-w-0 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 py-2 text-sm text-brand-text-primary outline-none focus:border-brand-lime focus:ring-1 focus:ring-brand-lime"
				value={name}
				oninput={(event) => (draft = event.currentTarget.value)}
				readonly={!canEdit || saving}
				required
				minlength="2"
				maxlength="120"
			/>
		</label>
		{#if error}
			<p class="text-sm text-brand-danger-text" role="alert">{error}</p>
		{/if}
		{#if canEdit}
			<div class="flex flex-wrap justify-end gap-2">
				{#if dirty}
					<Button
						size="sm"
						variant="ghost"
						disabled={saving}
						onclick={() => {
							draft = undefined;
							error = '';
						}}
					>
						Discard
					</Button>
				{/if}
				<Button
					size="sm"
					type="submit"
					disabled={!dirty || saving}
					loading={saving}
				>
					Save changes
				</Button>
			</div>
		{:else}
			<p class="text-sm text-brand-text-secondary">
				Owners and admins can edit the workspace name.
			</p>
		{/if}
	</form>
</section>

<section
	class="grid gap-5 rounded-xl border border-brand-border bg-brand-surface-card p-5 sm:grid-cols-[minmax(0,1fr)_minmax(0,1.5fr)] sm:p-6"
	aria-label="Workspace identifiers"
>
	<div>
		<h2 class="font-heading text-lg font-bold">Workspace details</h2>
		<p class="mt-1 text-sm leading-relaxed text-brand-text-secondary">
			Use the workspace ID in control plane API requests.
		</p>
	</div>
	<dl class="grid min-w-0 gap-4 text-sm">
		<div class="grid gap-1.5">
			<dt class="font-medium">Workspace ID</dt>
			<dd
				class="flex min-w-0 flex-wrap items-center justify-between gap-2 rounded-lg border border-brand-border bg-brand-surface-input px-3 py-2"
			>
				<code
					class="min-w-0 text-xs break-all text-brand-text-secondary select-all"
				>
					{workspace.id}
				</code>
				<Button variant="outline" size="sm" onclick={copyId}>Copy ID</Button>
			</dd>
		</div>
		<div
			class="flex flex-wrap items-center justify-between gap-2 border-t border-brand-border pt-4"
		>
			<dt class="font-medium">Slug</dt>
			<dd class="font-mono break-all text-brand-text-secondary">
				{workspace.slug}
			</dd>
		</div>
	</dl>
</section>
