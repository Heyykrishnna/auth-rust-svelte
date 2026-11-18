<script lang="ts">
	import type { WorkspaceSummary } from '$lib/api/types';
	import { getInitials } from '$lib/control/permissions';

	let {
		workspace,
		alt = '',
		class: className = ''
	}: { workspace: WorkspaceSummary; alt?: string; class?: string } = $props();

	let failedSource = $state<string>();
	const source = $derived(
		workspace.logo_version
			? `/v1/control/workspaces/${encodeURIComponent(workspace.id)}/logo?v=${encodeURIComponent(workspace.logo_version)}`
			: undefined
	);
</script>

<span
	class="relative grid shrink-0 place-items-center overflow-hidden rounded-lg border border-brand-success-border bg-brand-success-soft font-heading font-bold text-brand-lime {className}"
>
	{#if source && source !== failedSource}
		{#key source}
			<img
				class="block size-full object-contain"
				src={source}
				{alt}
				onerror={() => (failedSource = source)}
			/>
		{/key}
	{:else}
		<span aria-hidden="true">
			{getInitials(workspace.name, workspace.slug)}
		</span>
	{/if}
</span>
