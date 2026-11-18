<script module lang="ts">
	type ToastType = 'success' | 'error' | 'warning' | 'info';

	export interface Toast {
		id: string;
		type: ToastType;
		message: string;
		duration?: number;
	}

	let toasts = $state<Toast[]>([]);

	export function addToast(message: string, type: ToastType = 'info', duration = 4000) {
		const id = crypto.randomUUID();
		const toast: Toast = { id, type, message, duration };
		toasts = [...toasts, toast];

		if (duration > 0) {
			setTimeout(() => removeToast(id), duration);
		}

		return id;
	}

	export function removeToast(id: string) {
		toasts = toasts.filter((t) => t.id !== id);
	}
</script>

<div class="toast-container" aria-live="polite" aria-atomic="false">
	{#each toasts as toast (toast.id)}
		<div class="toast toast-{toast.type}" role="status">
			<span class="toast-dot" aria-hidden="true"></span>
			<span class="toast-message">{toast.message}</span>
			<button
				type="button"
				class="toast-close"
				onclick={() => removeToast(toast.id)}
				aria-label="Dismiss notification"
			>
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
					<line x1="18" y1="6" x2="6" y2="18" />
					<line x1="6" y1="6" x2="18" y2="18" />
				</svg>
			</button>
		</div>
	{/each}
</div>

<style>
	.toast-container {
		position: fixed;
		bottom: 1.5rem;
		right: 1.5rem;
		z-index: 9999;
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
		pointer-events: none;
	}

	.toast {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.8rem 1rem;
		border-radius: 8px;
		background-color: var(--color-bg-surface);
		border: 1px solid var(--color-border-hover);
		color: var(--color-text);
		min-width: 280px;
		max-width: 420px;
		pointer-events: all;
	}

	.toast-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex-shrink: 0;
		background-color: var(--color-info);
	}

	.toast-success .toast-dot { background-color: var(--color-success); }
	.toast-error .toast-dot { background-color: var(--color-error); }
	.toast-warning .toast-dot { background-color: var(--color-warning); }

	.toast-message {
		flex: 1;
		font-size: 0.85rem;
		font-weight: 500;
	}

	.toast-close {
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0.2rem;
	}

	.toast-close:hover { color: var(--color-text); }
</style>
