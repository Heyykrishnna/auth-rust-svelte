<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title,
		description,
		children
	}: { title: string; description: string; children?: Snippet } = $props();
</script>

<header
	class="flex min-w-0 flex-wrap items-end justify-between gap-4 border-b border-brand-border pb-4"
>
	<div class="min-w-0 flex-1 basis-60">
		<h1
			class="m-0 font-heading text-2xl font-bold tracking-tight text-brand-text-primary sm:text-3xl"
		>
			{title}
		</h1>
		<p
			class="mt-1.5 max-w-2xl text-sm leading-relaxed text-brand-text-secondary"
		>
			{description}
		</p>
	</div>
	{#if children}
		<div class="flex min-w-0 flex-wrap items-center gap-2">
			{@render children()}
		</div>
	{/if}
</header>
