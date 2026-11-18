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
		class="fixed inset-0 bg-black/80 flex items-center justify-center z-50 p-4"
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
		<div
			class="bg-brand-surface-card border border-brand-border-focus rounded-xl max-w-lg w-full overflow-hidden shadow-2xl"
		>
			<div class="flex items-center justify-between px-5 py-4 border-b border-brand-border">
				<h3 class="m-0 text-base font-semibold font-heading text-brand-text-primary">
					{title}
				</h3>
				<button
					type="button"
					class="bg-transparent border-none text-brand-text-muted hover:text-brand-text-primary hover:bg-brand-surface-raised cursor-pointer p-1 inline-flex items-center justify-center rounded-md transition-colors"
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

			<div class="p-5 text-sm text-brand-text-secondary leading-relaxed">
				{#if description}
					<p class="m-0 mb-3 text-brand-text-secondary">{description}</p>
				{/if}
				{#if children}
					{@render children()}
				{/if}
			</div>

			<div
				class="flex items-center justify-end gap-2.5 px-5 py-3.5 border-t border-brand-border bg-brand-surface"
			>
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
