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
		border-radius: 6px;
		letter-spacing: 0.02em;
		white-space: nowrap;
		line-height: 1;
	}

	.badge-sm {
		padding: 0.25rem 0.5rem;
		font-size: 0.7rem;
	}

	.badge-md {
		padding: 0.32rem 0.65rem;
		font-size: 0.75rem;
	}

	.badge-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background-color: currentColor;
	}

	.badge-neutral {
		background-color: #1f222a;
		color: #d4d4d8;
		border: 1px solid #2e333d;
	}

	.badge-primary {
		background-color: #ffffff;
		color: #0b0c0e;
		border: 1px solid #ffffff;
	}

	.badge-purple {
		background-color: #242731;
		color: #e4e4e7;
		border: 1px solid #3c4150;
	}

	.badge-success {
		background-color: #11241a;
		color: #4ade80;
		border: 1px solid #1d442e;
	}

	.badge-warning {
		background-color: #26200f;
		color: #facc15;
		border: 1px solid #4b3d18;
	}

	.badge-danger {
		background-color: #291416;
		color: #f87171;
		border: 1px solid #572327;
	}
</style>
