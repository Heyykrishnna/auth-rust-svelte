<script lang="ts">
	import type { Snippet } from 'svelte';
	import Button from './Button.svelte';

	interface Props {
		open: boolean;
		title: string;
		description?: string;
		confirmText?: string;
		cancelText?: string;
		variant?: 'danger' | 'primary';
		loading?: boolean;
		onconfirm: () => void;
		oncancel: () => void;
		children?: Snippet;
	}

	let {
		open,
		title,
		description,
		confirmText = 'Confirm',
		cancelText = 'Cancel',
		variant = 'primary',
		loading = false,
		onconfirm,
		oncancel,
		children
	}: Props = $props();

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget && !loading) {
			oncancel();
		}
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape' && open && !loading) {
			oncancel();
		}
	}
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if open}
	<div
		class="modal-backdrop"
		onclick={handleBackdropClick}
		onkeydown={(e) => {
			if ((e.key === 'Escape' || e.key === 'Enter') && e.target === e.currentTarget && !loading) {
				oncancel();
			}
		}}
		role="dialog"
		aria-modal="true"
		aria-label={title}
		tabindex="-1"
	>
		<div class="modal-dialog">
			<div class="modal-header">
				<h3 class="modal-title">{title}</h3>
				<button
					type="button"
					class="modal-close"
					onclick={oncancel}
					disabled={loading}
					aria-label="Close dialog"
				>
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
						<line x1="18" y1="6" x2="6" y2="18" />
						<line x1="6" y1="6" x2="18" y2="18" />
					</svg>
				</button>
			</div>

			<div class="modal-content">
				{#if description}
					<p class="modal-desc">{description}</p>
				{/if}
				{#if children}
					{@render children()}
				{/if}
			</div>

			<div class="modal-footer">
				<Button variant="ghost" onclick={oncancel} disabled={loading}>
					{cancelText}
				</Button>
				<Button
					variant={variant === 'danger' ? 'danger' : 'primary'}
					onclick={onconfirm}
					{loading}
				>
					{confirmText}
				</Button>
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background-color: rgba(0, 0, 0, 0.78);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		padding: 1rem;
	}

	.modal-dialog {
		background-color: var(--color-bg-surface);
		border: 1px solid var(--color-border-hover);
		border-radius: 12px;
		max-width: 520px;
		width: 100%;
		overflow: hidden;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1.15rem 1.35rem;
		border-bottom: 1px solid var(--color-border);
	}

	.modal-title {
		margin: 0;
		font-size: 1.05rem;
		font-weight: 600;
		color: var(--color-text);
	}

	.modal-close {
		background: transparent;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: 0.25rem;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: 6px;
	}

	.modal-close:hover {
		color: var(--color-text);
		background-color: var(--color-bg-elevated);
	}

	.modal-content {
		padding: 1.35rem;
		color: var(--color-text-muted);
		font-size: 0.9rem;
		line-height: 1.5;
	}

	.modal-desc {
		margin: 0;
		color: var(--color-text-muted);
	}

	.modal-footer {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.65rem;
		padding: 0.95rem 1.35rem;
		border-top: 1px solid var(--color-border);
		background-color: var(--color-bg-sidebar);
	}
</style>
