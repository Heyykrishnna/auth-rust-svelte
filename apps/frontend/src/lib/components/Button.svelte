<script lang="ts">
	interface Props {
		variant?: 'primary' | 'outline' | 'ghost' | 'danger';
		size?: 'sm' | 'md' | 'lg';
		type?: 'button' | 'submit' | 'reset';
		disabled?: boolean;
		loading?: boolean;
		full?: boolean;
		href?: string;
		onclick?: () => void;
		children: import('svelte').Snippet;
	}

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
	}: Props = $props();

	const sizeClass = {
		sm: 'btn-sm',
		md: '',
		lg: 'btn-lg'
	}[size];
</script>

{#if href}
	<a
		{href}
		class="btn btn-{variant} {sizeClass} {full ? 'btn-full' : ''}"
		class:loading
	>
		{#if loading}
			<span class="spinner" aria-hidden="true"></span>
		{/if}
		{@render children()}
	</a>
{:else}
	<button
		{type}
		{disabled}
		{onclick}
		class="btn btn-{variant} {sizeClass} {full ? 'btn-full' : ''}"
		class:loading
		aria-busy={loading}
	>
		{#if loading}
			<span class="spinner" aria-hidden="true"></span>
		{/if}
		{@render children()}
	</button>
{/if}

<style>
	.btn-sm { padding: 0.375rem 0.875rem; font-size: 0.8125rem; }
	.btn-lg { padding: 0.875rem 2rem; font-size: 1rem; }
</style>
