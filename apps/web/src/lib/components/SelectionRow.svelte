<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		label,
		description,
		checked = $bindable(false),
		disabled = false,
		compact = false,
		onchange,
		children
	}: {
		label: string;
		description?: string;
		checked?: boolean;
		disabled?: boolean;
		compact?: boolean;
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

<label class="group relative block min-w-0 select-none">
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
	<span
		class="flex min-w-0 cursor-pointer items-center transition-all duration-200 ease-[cubic-bezier(0.16,1,0.3,1)] peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-[#296FF0] peer-disabled:cursor-not-allowed peer-disabled:opacity-50 {compact
			? 'h-8 min-h-8 gap-2 rounded-lg border border-brand-border-hover bg-brand-surface-input px-2.5 text-brand-text-primary hover:border-brand-border-strong hover:bg-brand-surface-raised'
			: 'min-h-11 gap-3.5 rounded-xl border border-brand-border bg-brand-surface-card px-3.5 py-2.5 text-brand-text-primary hover:border-brand-border-strong hover:bg-brand-surface-raised'}"
	>
		<span
			class="grid shrink-0 place-items-center rounded-full transition-all duration-200 ease-[cubic-bezier(0.16,1,0.3,1)] {compact ? 'size-4' : 'size-5'} {checked
				? 'checkbox-circle-checked'
				: 'border border-zinc-600 bg-zinc-900/60 text-transparent group-hover:border-zinc-400'}"
			aria-hidden="true"
		>
			<svg
				class={compact ? 'size-2.5' : 'size-3'}
				viewBox="0 0 12 12"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M2.5 6.5L4.5 8.5L9.5 3.5" />
			</svg>
		</span>
		<span class="grid min-w-0 gap-0.5">
			<span
				id={`${id}-label`}
				class="min-w-0 tracking-tight {compact ? 'truncate text-xs font-medium' : 'break-words text-sm font-medium'} {checked ? 'text-white' : 'text-brand-text-primary'}"
			>
				{#if children}{@render children()}{:else}{label}{/if}
			</span>
			{#if description && !compact}
				<span id={`${id}-description`} class="text-xs leading-relaxed text-brand-text-secondary">{description}</span>
			{/if}
		</span>
	</span>
</label>
