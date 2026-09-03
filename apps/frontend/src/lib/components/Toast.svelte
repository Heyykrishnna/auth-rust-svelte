<script lang="ts">
	type ToastType = 'success' | 'error' | 'warning' | 'info';

	export interface Toast {
		id: string;
		type: ToastType;
		message: string;
		duration?: number;
	}

	// Global toast state
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

	const icons: Record<ToastType, string> = {
		success: '✓',
		error: '✕',
		warning: '⚠',
		info: 'ℹ'
	};
</script>

<div class="toast-container" aria-live="polite" aria-atomic="false">
	{#each toasts as toast (toast.id)}
		<div class="toast toast-{toast.type}" role="status">
			<span class="toast-icon" aria-hidden="true">{icons[toast.type]}</span>
			<span class="toast-message">{toast.message}</span>
			<button
				class="toast-close"
				onclick={() => removeToast(toast.id)}
				aria-label="Dismiss notification"
			>
				✕
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
		gap: 0.75rem;
		pointer-events: none;
	}

	.toast {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.875rem 1rem;
		border-radius: 0.75rem;
		border: 1px solid;
		backdrop-filter: blur(12px);
		min-width: 300px;
		max-width: 420px;
		pointer-events: all;
		animation: slideIn 0.3s ease both;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
	}

	@keyframes slideIn {
		from { opacity: 0; transform: translateX(100%); }
		to   { opacity: 1; transform: translateX(0); }
	}

	.toast-success {
		background: rgba(16, 185, 129, 0.12);
		border-color: rgba(16, 185, 129, 0.3);
		color: #6ee7b7;
	}

	.toast-error {
		background: rgba(239, 68, 68, 0.12);
		border-color: rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}

	.toast-warning {
		background: rgba(245, 158, 11, 0.12);
		border-color: rgba(245, 158, 11, 0.3);
		color: #fcd34d;
	}

	.toast-info {
		background: rgba(6, 182, 212, 0.12);
		border-color: rgba(6, 182, 212, 0.3);
		color: #67e8f9;
	}

	.toast-icon {
		font-size: 1rem;
		flex-shrink: 0;
		width: 1.5rem;
		height: 1.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.toast-message { flex: 1; font-size: 0.9375rem; font-weight: 500; }

	.toast-close {
		background: none;
		border: none;
		color: inherit;
		cursor: pointer;
		opacity: 0.6;
		font-size: 0.875rem;
		padding: 0.25rem;
		transition: opacity 150ms ease;
		flex-shrink: 0;
	}

	.toast-close:hover { opacity: 1; }
</style>
