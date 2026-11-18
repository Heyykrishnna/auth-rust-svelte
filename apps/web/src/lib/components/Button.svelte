<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		variant = 'primary',
		size = 'md',
		type = 'button',
		disabled = false,
		loading = false,
		full = false,
		href,
		onclick,
		children
	}: {
		variant?: 'primary' | 'danger';
		size?: 'sm' | 'md' | 'lg';
		type?: 'button' | 'submit' | 'reset';
		disabled?: boolean;
		loading?: boolean;
		full?: boolean;
		href?: string;
		onclick?: () => void;
		children: Snippet;
	} = $props();

	const sizes = {
		sm: 'h-8 min-h-8 px-3 text-xs',
		md: 'h-10 min-h-10 px-4 text-sm',
		lg: 'h-11 min-h-11 px-6 text-base'
	};
	const variantClass = $derived(variant === 'danger' ? 'danger-button' : 'action-button');
	const classes = $derived(
		`${variantClass} inline-flex items-center justify-center gap-2 rounded-lg text-center font-semibold no-underline select-none ${sizes[size]} ${full ? 'w-full' : ''}`
	);
	const unavailable = $derived(disabled || loading);
</script>

{#snippet content()}
	{#if loading}
		<span class="size-4 shrink-0 rounded-full border-2 border-current border-r-transparent motion-safe:animate-spin" aria-hidden="true"></span>
	{/if}
	{@render children()}
{/snippet}

{#if href}
	{#if unavailable}
		<span class={classes} role="link" aria-busy={loading} aria-disabled="true">{@render content()}</span>
	{:else}
		<a {href} class={classes} {onclick}>{@render content()}</a>
	{/if}
{:else}
	<button {type} disabled={unavailable} {onclick} class={classes} aria-busy={loading}>{@render content()}</button>
{/if}
