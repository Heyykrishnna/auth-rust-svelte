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
					✕
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
		background: rgba(0, 0, 0, 0.75);
		backdrop-filter: blur(8px);
		-webkit-backdrop-filter: blur(8px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		padding: 1rem;
		animation: fadeIn 0.15s ease-out;
	}

	.modal-dialog {
		background: #131927;
		border: 1px solid rgba(255, 255, 255, 0.1);
		box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.6);
		border-radius: var(--radius-lg, 16px);
		max-width: 480px;
		width: 100%;
		overflow: hidden;
		animation: scaleUp 0.2s cubic-bezier(0.16, 1, 0.3, 1);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1.25rem 1.5rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.modal-title {
		margin: 0;
		font-size: 1.15rem;
		font-weight: 600;
		color: #f8fafc;
	}

	.modal-close {
		background: transparent;
		border: none;
		color: #94a3b8;
		font-size: 1.1rem;
		cursor: pointer;
		padding: 0.25rem;
		border-radius: 4px;
		transition: color 0.15s ease;
	}

	.modal-close:hover {
		color: #ffffff;
	}

	.modal-content {
		padding: 1.5rem;
		color: #cbd5e1;
		font-size: 0.95rem;
		line-height: 1.5;
	}

	.modal-desc {
		margin: 0;
		color: #94a3b8;
	}

	.modal-footer {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1rem 1.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		background: rgba(0, 0, 0, 0.15);
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes scaleUp {
		from {
			opacity: 0;
			transform: scale(0.95);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}
</style>
