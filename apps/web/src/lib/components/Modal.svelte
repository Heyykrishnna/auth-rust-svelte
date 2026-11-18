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

	let dialog = $state<HTMLDialogElement>();

	$effect(() => {
		if (open && dialog && !dialog.open) dialog.showModal();
		if (!open && dialog?.open) dialog.close();
	});

	function handleCancel(event: Event) {
		event.preventDefault();
		if (!loading) oncancel();
	}
</script>

<dialog
	bind:this={dialog}
	class="m-auto max-h-[calc(100dvh-2rem)] w-[calc(100%-2rem)] max-w-lg overflow-auto rounded-xl border-0 bg-transparent p-0 text-brand-text-primary backdrop:bg-black/80"
	aria-label={title}
	oncancel={handleCancel}
	onclick={handleBackdropClick}
>
	{#if open}
		<div
			class="w-full max-w-lg overflow-hidden rounded-xl border bg-brand-surface-card shadow-2xl {variant === 'danger' ? 'border-brand-danger-border' : 'border-brand-border-focus'}"
		>
			<div
				class="flex items-center justify-between border-b border-brand-border px-5 py-4"
			>
				<h3
					class="m-0 font-heading text-base font-semibold text-brand-text-primary"
				>
					{title}
				</h3>
				<button
					type="button"
					class="action-button inline-grid size-8 shrink-0 place-items-center rounded-md"
					onclick={oncancel}
					disabled={loading}
					aria-label="Close dialog"
				>
					<svg
						width="16"
						height="16"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.8"
						aria-hidden="true"
					>
						<line x1="18" y1="6" x2="6" y2="18" />
						<line x1="6" y1="6" x2="18" y2="18" />
					</svg>
				</button>
			</div>

			<div class="p-5 text-sm leading-relaxed text-brand-text-secondary">
				{#if description}
					<p class="m-0 mb-3 text-brand-text-secondary">{description}</p>
				{/if}
				{#if children}
					{@render children()}
				{/if}
			</div>

			<div
				class="flex items-center justify-end gap-2.5 border-t border-brand-border bg-brand-surface px-5 py-3.5"
			>
				<Button onclick={oncancel} disabled={loading}>
					{cancelText}
				</Button>
				<Button
					onclick={onconfirm}
					disabled={loading}
					{loading}
				>
					{confirmText}
				</Button>
			</div>
		</div>
	{/if}
</dialog>
