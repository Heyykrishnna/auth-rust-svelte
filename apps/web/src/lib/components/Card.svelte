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
</script>

<div class="card card-{variant} {className}">
	{#if title || subtitle || headerActions}
		<div class="card-header">
			<div class="card-headings">
				{#if title}
					<h3 class="card-title">{title}</h3>
				{/if}
				{#if subtitle}
					<p class="card-subtitle">{subtitle}</p>
				{/if}
			</div>
			{#if headerActions}
				<div class="card-actions">
					{@render headerActions()}
				</div>
			{/if}
		</div>
	{/if}

	<div class="card-body">
		{@render children()}
	</div>

	{#if footer}
		<div class="card-footer">
			{@render footer()}
		</div>
	{/if}
</div>

<style>
	.card {
		border-radius: 12px;
		overflow: hidden;
		position: relative;
		background-color: var(--color-bg-surface);
		border: 1px solid var(--color-border);
	}

	.card-light {
		background-color: var(--color-bg-light);
		border-color: #d4d4d8;
		color: var(--color-text-dark);
	}

	.card-light .card-title {
		color: var(--color-text-dark);
	}

	.card-light .card-subtitle {
		color: #52525b;
	}

	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		padding: 1.15rem 1.35rem;
		border-bottom: 1px solid var(--color-border);
	}

	.card-light .card-header {
		border-bottom-color: #e4e4e7;
	}

	.card-headings {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.card-title {
		margin: 0;
		font-size: 0.98rem;
		font-weight: 600;
		color: var(--color-text);
		letter-spacing: -0.01em;
	}

	.card-subtitle {
		margin: 0;
		font-size: 0.82rem;
		color: var(--color-text-muted);
	}

	.card-actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.card-body {
		padding: 1.35rem;
	}

	.card-footer {
		padding: 0.95rem 1.35rem;
		background-color: var(--color-bg-sidebar);
		border-top: 1px solid var(--color-border);
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
</style>
