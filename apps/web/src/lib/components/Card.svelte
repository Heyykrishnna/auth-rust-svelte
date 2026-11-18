<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title,
		subtitle,
		class: className = '',
		headerActions,
		footer,
		children
	}: {
		title?: string;
		subtitle?: string;
		class?: string;
		headerActions?: Snippet;
		footer?: Snippet;
		children: Snippet;
	} = $props();
</script>

<div class="task-surface relative min-w-0 overflow-hidden text-brand-text-primary {className}">
	{#if title || subtitle || headerActions}
		<header class="flex flex-wrap items-start justify-between gap-3 border-b border-brand-border px-4 py-4 sm:px-5">
			<div class="min-w-0 flex-1">
				{#if title}
					<h3 class="font-heading text-base font-bold tracking-tight">{title}</h3>
				{/if}
				{#if subtitle}
					<p class="mt-1 text-xs leading-relaxed text-brand-text-secondary">{subtitle}</p>
				{/if}
			</div>
			{#if headerActions}
				<div class="flex flex-wrap items-center gap-2">{@render headerActions()}</div>
			{/if}
		</header>
	{/if}

	<div class="min-w-0 p-4 sm:p-5">{@render children()}</div>

	{#if footer}
		<footer class="flex flex-wrap items-center justify-between gap-3 border-t border-brand-border bg-brand-surface-input px-4 py-3.5 sm:px-5">
			{@render footer()}
		</footer>
	{/if}
</div>
