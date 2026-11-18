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

<div class="fixed bottom-6 right-6 z-50 flex flex-col gap-2.5 pointer-events-none" aria-live="polite" aria-atomic="false">
	{#each toasts as toast (toast.id)}
		<div
			class="flex items-center gap-3 px-4 py-3 rounded-lg bg-brand-surface-card border border-brand-border-hover text-brand-text-primary min-w-[280px] max-w-[420px] pointer-events-auto shadow-xl"
			role="status"
		>
			<span
				class="w-2 h-2 rounded-full shrink-0 {toast.type === 'success' ? 'bg-brand-success' : toast.type === 'error' ? 'bg-brand-danger' : toast.type === 'warning' ? 'bg-brand-warning' : 'bg-brand-info'}"
				aria-hidden="true"
			></span>
			<span class="flex-1 text-xs font-medium">{toast.message}</span>
			<button
				type="button"
				class="bg-transparent border-none text-brand-text-muted hover:text-brand-text-primary cursor-pointer inline-flex items-center justify-center p-1 rounded transition-colors"
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
