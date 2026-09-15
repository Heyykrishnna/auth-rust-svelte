<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		title?: string;
		subtitle?: string;
		variant?: 'glass' | 'surface' | 'gradient';
		class?: string;
		headerActions?: Snippet;
		footer?: Snippet;
		children: Snippet;
	}

	let {
		title,
		subtitle,
		variant = 'glass',
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
		border-radius: var(--radius-lg, 16px);
		transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
		overflow: hidden;
		position: relative;
	}

	.card-glass {
		background: rgba(18, 24, 38, 0.7);
		backdrop-filter: blur(16px);
		-webkit-backdrop-filter: blur(16px);
		border: 1px solid rgba(255, 255, 255, 0.08);
		box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.37);
	}

	.card-surface {
		background: var(--color-surface, #161c2e);
		border: 1px solid rgba(255, 255, 255, 0.06);
		box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
	}

	.card-gradient {
		background: linear-gradient(135deg, rgba(30, 41, 59, 0.7) 0%, rgba(15, 23, 42, 0.8) 100%);
		border: 1px solid rgba(99, 102, 241, 0.2);
		box-shadow: 0 10px 40px -10px rgba(99, 102, 241, 0.15);
	}

	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem 1.75rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
	}

	.card-headings {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.card-title {
		margin: 0;
		font-size: 1.15rem;
		font-weight: 600;
		color: #f8fafc;
		letter-spacing: -0.01em;
	}

	.card-subtitle {
		margin: 0;
		font-size: 0.875rem;
		color: #94a3b8;
	}

	.card-actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.card-body {
		padding: 1.75rem;
	}

	.card-footer {
		padding: 1.25rem 1.75rem;
		background: rgba(0, 0, 0, 0.15);
		border-top: 1px solid rgba(255, 255, 255, 0.05);
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
</style>
