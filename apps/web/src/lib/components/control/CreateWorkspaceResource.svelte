<script lang="ts">
	import { controlApi } from '$lib/api/control';
	import type {
		CreatedApplicationResponse,
		WorkspaceSummary
	} from '$lib/api/types';
	import Modal from '$lib/components/Modal.svelte';
	import {
		validateRedirectUriForEnv,
		validateSlug
	} from '$lib/control/validation';

	let {
		workspaceId,
		onApplicationCreated,
		onWorkspaceCreated,
		onClose
	}: {
		workspaceId?: string;
		onApplicationCreated: (app: CreatedApplicationResponse) => void;
		onWorkspaceCreated: (workspace: WorkspaceSummary) => void;
		onClose: () => void;
	} = $props();

	let name = $state('');
	let slug = $state('');
	let customSlug = $state(false);
	let redirect = $state('');
	let error = $state('');
	let saving = $state(false);
	let form: HTMLFormElement;

	const kind = $derived(workspaceId ? 'application' : 'workspace');
	const defaultSlug = $derived(
		name
			.normalize('NFKD')
			.replace(/\p{M}/gu, '')
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+/, '')
			.slice(0, 64)
			.replace(/-+$/, '')
	);
	const fieldClass =
		'min-h-10 w-full min-w-0 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 py-2 text-sm text-brand-text-primary outline-none placeholder:text-brand-text-muted focus:border-brand-lime focus:ring-1 focus:ring-brand-lime';

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (saving) return;
		const cleanName = name.trim();
		const cleanSlug = (customSlug ? slug : defaultSlug).trim().toLowerCase();
		const cleanRedirect = redirect.trim();
		error = validateSlug(cleanSlug) ?? '';
		if ([...cleanName].length < 2)
			error = 'Enter a name with at least 2 characters.';
		if (!error && workspaceId && cleanRedirect) {
			error = validateRedirectUriForEnv('development', cleanRedirect) ?? '';
		}
		if (error) return;

		saving = true;
		try {
			if (workspaceId) {
				const created = await controlApi.createApplication({
					workspace_id: workspaceId,
					name: cleanName,
					slug: cleanSlug,
					initial_dev_redirect_uri: cleanRedirect || undefined
				});
				onApplicationCreated(created);
			} else {
				onWorkspaceCreated(
					await controlApi.createWorkspace({ name: cleanName, slug: cleanSlug })
				);
			}
		} catch (err) {
			error =
				err instanceof Error ? err.message : `Could not create the ${kind}.`;
		} finally {
			saving = false;
		}
	}
</script>

<Modal
	open
	title={`Create ${kind}`}
	description={workspaceId
		? 'Each application gets separate development and production environments.'
		: 'Create a workspace for a new team or a separate group of applications.'}
	confirmText={`Create ${kind}`}
	loading={saving}
	onconfirm={() => form.requestSubmit()}
	oncancel={onClose}
>
	<form bind:this={form} class="grid gap-4 pt-2" onsubmit={submit}>
		<label class="grid gap-1.5 font-medium text-brand-text-primary">
			{workspaceId ? 'Application name' : 'Workspace name'}
			<input class={fieldClass} bind:value={name} required maxlength="120" />
		</label>
		<label class="grid gap-1.5 font-medium text-brand-text-primary">
			{workspaceId ? 'Application slug' : 'Workspace slug'}
			<input
				class={fieldClass}
				value={customSlug ? slug : defaultSlug}
				oninput={(event) => {
					customSlug = true;
					slug = event.currentTarget.value;
				}}
				required
				maxlength="64"
				autocapitalize="none"
				spellcheck="false"
			/>
		</label>
		{#if workspaceId}
			<label class="grid gap-1.5 font-medium text-brand-text-primary">
				<span>
					Development redirect URL <span
						class="font-normal text-brand-text-secondary"
					>
						(optional)
					</span>
				</span>
				<input
					class={fieldClass}
					bind:value={redirect}
					type="url"
					placeholder="http://localhost:3000/auth/callback"
				/>
			</label>
		{/if}
		{#if error}
			<p role="alert" class="text-sm text-brand-danger-text">{error}</p>
		{/if}
		<button type="submit" hidden>Submit</button>
	</form>
</Modal>
