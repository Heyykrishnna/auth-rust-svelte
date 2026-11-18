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

	const baseClasses =
		'inline-flex items-center justify-center gap-2 rounded-md font-sans font-medium transition-colors select-none whitespace-nowrap border cursor-pointer no-underline disabled:opacity-45 disabled:cursor-not-allowed disabled:pointer-events-none focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-lime';

	const variantClasses: Record<string, string> = {
		primary:
			'bg-brand-lime text-brand-lime-ink border-brand-lime hover:bg-brand-lime-hover hover:border-brand-lime-hover',
		outline:
			'bg-transparent border-brand-border-hover text-brand-text-primary hover:bg-brand-surface-raised hover:border-brand-border-strong',
		ghost:
			'bg-transparent border-transparent text-brand-text-muted hover:bg-brand-surface-raised hover:text-brand-text-primary',
		danger:
			'bg-brand-danger-soft text-brand-danger-text border-brand-danger-border hover:bg-rose-950/70 hover:border-brand-danger'
	};

	const sizeClasses: Record<string, string> = {
		sm: 'px-3 py-1.5 text-xs',
		md: 'px-4 py-2 text-sm',
		lg: 'px-6 py-3 text-base'
	};

	const classes = $derived(
		`${baseClasses} ${variantClasses[variant] ?? variantClasses.primary} ${sizeClasses[size] ?? sizeClasses.md} ${full ? 'w-full' : ''}`
	);
</script>

{#if href}
	<a {href} class={classes}>
		{#if loading}
			<span
				class="w-4 h-4 border-2 border-current border-r-transparent rounded-full animate-spin"
				aria-hidden="true"
			></span>
		{/if}
		{@render children()}
	</a>
{:else}
	<button {type} {disabled} {onclick} class={classes} aria-busy={loading}>
		{#if loading}
			<span
				class="w-4 h-4 border-2 border-current border-r-transparent rounded-full animate-spin"
				aria-hidden="true"
			></span>
		{/if}
		{@render children()}
	</button>
{/if}
