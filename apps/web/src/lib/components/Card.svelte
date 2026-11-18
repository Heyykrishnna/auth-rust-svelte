<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		title?: string;
		subtitle?: string;
		variant?: 'glass' | 'surface' | 'gradient' | 'light';
		class?: string;
		headerActions?: Snippet;
		footer?: Snippet;
		children: Snippet;
	}

	let {
		title,
		subtitle,
		variant = 'surface',
		class: className = '',
		headerActions,
		footer,
		children
	}: Props = $props();

	const variantStyles: Record<string, string> = {
		surface: 'bg-brand-surface-card border-brand-border text-brand-text-primary',
		glass: 'bg-brand-surface-card/90 backdrop-blur-md border-brand-border text-brand-text-primary',
		gradient: 'bg-brand-surface-card border-brand-border text-brand-text-primary',
		light: 'bg-[#e8f0b8] border-zinc-300 text-[#11130d]'
	};
</script>

<div
	class="rounded-xl overflow-hidden relative border {variantStyles[variant] ?? variantStyles.surface} {className}"
>
	{#if title || subtitle || headerActions}
		<div
			class="flex items-center justify-between gap-4 px-5 py-4 border-b {variant === 'light' ? 'border-zinc-200' : 'border-brand-border'}"
		>
			<div class="flex flex-col gap-0.5">
				{#if title}
					<h3
						class="m-0 text-[15px] font-semibold font-heading tracking-tight {variant === 'light' ? 'text-[#11130d]' : 'text-brand-text-primary'}"
					>
						{title}
					</h3>
				{/if}
				{#if subtitle}
					<p
						class="m-0 text-xs {variant === 'light' ? 'text-zinc-600' : 'text-brand-text-secondary'}"
					>
						{subtitle}
					</p>
				{/if}
			</div>
			{#if headerActions}
				<div class="flex items-center gap-2">
					{@render headerActions()}
				</div>
			{/if}
		</div>
	{/if}

	<div class="p-5">
		{@render children()}
	</div>

	{#if footer}
		<div
			class="px-5 py-3.5 bg-brand-surface border-t border-brand-border flex items-center justify-between"
		>
			{@render footer()}
		</div>
	{/if}
</div>
