<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		label,
		description,
		checked = $bindable(false),
		disabled = false,
		onchange,
		children
	}: {
		label: string;
		description?: string;
		checked?: boolean;
		disabled?: boolean;
		onchange?: (checked: boolean) => void;
		children?: Snippet;
	} = $props();

	const id = $props.id();

	function select(event: Event & { currentTarget: HTMLInputElement }) {
		const next = event.currentTarget.checked;
		if (onchange) {
			event.currentTarget.checked = checked;
			onchange(next);
		} else {
			checked = next;
		}
	}
</script>

<label class="relative block min-w-0">
	<input
		type="checkbox"
		{id}
		aria-labelledby={`${id}-label`}
		aria-describedby={description ? `${id}-description` : undefined}
		{disabled}
		{checked}
		onchange={select}
		class="peer sr-only"
	/>
	<span class="flex min-h-9 min-w-0 cursor-pointer items-center gap-2.5 rounded-lg border border-brand-border bg-brand-surface-card px-2.5 py-2 text-brand-text-primary transition-colors hover:border-brand-border-strong peer-checked:border-action/65 peer-checked:bg-action/15 peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-action-focus peer-disabled:cursor-not-allowed peer-disabled:opacity-60">
		<span class="grid size-4 shrink-0 place-items-center rounded-sm border {checked ? 'border-action bg-action text-white' : 'border-brand-text-muted text-transparent'}" aria-hidden="true">
			<svg class="size-3" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3.5 8 3 3 6-6" /></svg>
		</span>
		<span class="grid min-w-0 gap-0.5">
			<span id={`${id}-label`} class="min-w-0 text-xs font-semibold break-words">
				{#if children}{@render children()}{:else}{label}{/if}
			</span>
			{#if description}
				<span id={`${id}-description`} class="text-xs leading-relaxed text-brand-text-secondary">{description}</span>
			{/if}
		</span>
	</span>
</label>
