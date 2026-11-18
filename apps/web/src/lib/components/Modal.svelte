<script lang="ts">
  import type { Snippet } from "svelte";
  import Button from "./Button.svelte";

  interface Props {
    open: boolean;
    title: string;
    description?: string;
    confirmText?: string;
    cancelText?: string;
    variant?: "danger" | "primary";
    loading?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
    children?: Snippet;
  }

  let {
    open,
    title,
    description,
    confirmText = "Confirm",
    cancelText = "Cancel",
    variant = "primary",
    loading = false,
    onconfirm,
    oncancel,
    children,
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
      class="w-full max-w-lg overflow-hidden rounded-xl border bg-brand-surface-card shadow-2xl {variant ===
      'danger'
        ? 'border-brand-danger-border'
        : 'border-brand-border-focus'}"
    >
      <div class="flex items-start justify-between gap-4 px-6 pt-6 pb-2">
        <div class="grid min-w-0 gap-1.5">
          <h3
            class="m-0 font-heading text-lg font-semibold text-brand-text-primary"
          >
            {title}
          </h3>
          {#if description}
            <p class="m-0 text-sm leading-relaxed text-brand-text-secondary">
              {description}
            </p>
          {/if}
        </div>
        <button
          type="button"
          class="inline-grid size-8 shrink-0 place-items-center rounded-lg text-zinc-400 hover:text-white transition-colors cursor-pointer bg-transparent border-0 p-0 focus-visible:outline-2 focus-visible:outline-[#296FF0]"
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

      {#if children}
        <div
          class="px-6 py-2 text-sm leading-relaxed text-brand-text-secondary"
        >
          {@render children()}
        </div>
      {/if}

      <div class="flex items-center justify-end gap-3 px-6 pt-4 pb-6">
        <button
          type="button"
          class="inline-flex min-h-10 items-center justify-center rounded-lg px-4 py-2 text-sm font-medium text-zinc-300 hover:text-white hover:bg-zinc-800 transition-colors cursor-pointer bg-transparent border-0"
          onclick={oncancel}
          disabled={loading}
        >
          {cancelText}
        </button>
        <Button {variant} onclick={onconfirm} disabled={loading} {loading}>
          {confirmText}
        </Button>
      </div>
    </div>
  {/if}
</dialog>
