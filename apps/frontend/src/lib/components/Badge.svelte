<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		variant?: 'primary' | 'success' | 'warning' | 'danger' | 'neutral' | 'purple';
		size?: 'sm' | 'md';
		dot?: boolean;
		class?: string;
		children: Snippet;
	}

	let {
		variant = 'neutral',
		size = 'md',
		dot = false,
		class: className = '',
		children
	}: Props = $props();
</script>

<span class="badge badge-{variant} badge-{size} {className}">
	{#if dot}
		<span class="badge-dot" aria-hidden="true"></span>
	{/if}
	{@render children()}
</span>

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		font-weight: 500;
		border-radius: 9999px;
		letter-spacing: 0.02em;
		text-transform: uppercase;
		white-space: nowrap;
		line-height: 1;
	}

	.badge-sm {
		padding: 0.2rem 0.5rem;
		font-size: 0.6875rem;
	}

	.badge-md {
		padding: 0.3rem 0.65rem;
		font-size: 0.75rem;
	}

	.badge-dot {
		width: 0.4rem;
		height: 0.4rem;
		border-radius: 50%;
		background-color: currentColor;
	}

	.badge-neutral {
		background: rgba(148, 163, 184, 0.12);
		color: #cbd5e1;
		border: 1px solid rgba(148, 163, 184, 0.2);
	}

	.badge-primary {
		background: rgba(99, 102, 241, 0.15);
		color: #818cf8;
		border: 1px solid rgba(99, 102, 241, 0.3);
	}

	.badge-purple {
		background: rgba(168, 85, 247, 0.15);
		color: #c084fc;
		border: 1px solid rgba(168, 85, 247, 0.3);
	}

	.badge-success {
		background: rgba(34, 197, 94, 0.12);
		color: #4ade80;
		border: 1px solid rgba(34, 197, 94, 0.25);
	}

	.badge-warning {
		background: rgba(245, 158, 11, 0.12);
		color: #fbbf24;
		border: 1px solid rgba(245, 158, 11, 0.25);
	}

	.badge-danger {
		background: rgba(239, 68, 68, 0.12);
		color: #f87171;
		border: 1px solid rgba(239, 68, 68, 0.25);
	}
</style>
