<script lang="ts">
  import CopyButton from "$lib/components/CopyButton.svelte";
  import DeleteButton from "$lib/components/DeleteButton.svelte";

  interface Props {
    title: string;
    placeholder?: string;
    items: string[];
    disabled?: boolean;
    kind?: "origin" | "redirect" | "logout";
    onchange?: (items: string[]) => void;
  }

  let {
    title,
    placeholder,
    items = $bindable([]),
    disabled = false,
    kind = "origin",
    onchange,
  }: Props = $props();

  let inputValue = $state("");

  function addRaw(raw: string) {
    const parts = raw
      .split(/[\n,]/)
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    if (parts.length === 0) return;
    const next = [...items];
    for (const p of parts) {
      if (!next.includes(p)) {
        next.push(p);
      }
    }
    items = next;
    onchange?.(items);
  }

  export function flushPending() {
    if (inputValue.trim()) {
      addRaw(inputValue);
      inputValue = "";
    }
  }

  function handleAddClick() {
    if (disabled || !inputValue.trim()) return;
    addRaw(inputValue);
    inputValue = "";
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      handleAddClick();
    }
  }

  function handlePaste(event: ClipboardEvent) {
    const pasted = event.clipboardData?.getData("text");
    if (pasted && (pasted.includes("\n") || pasted.includes(","))) {
      event.preventDefault();
      addRaw(pasted);
    }
  }

  function removeItem(index: number) {
    if (disabled) return;
    const next = items.filter((_, i) => i !== index);
    items = next;
    onchange?.(items);
  }
</script>

<div class="uri-manager task-surface flex flex-col gap-2.5 p-3.5 sm:p-4">
  <div class="flex items-center justify-between gap-2">
    <div class="flex items-center gap-1.5 min-w-0">
      {#if kind === "origin"}
        <svg
          class="size-3.5 shrink-0 text-[#296FF0]"
          viewBox="0 0 256 256"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            d="M128 24a104 104 0 1 0 104 104A104.11 104.11 0 0 0 128 24Zm-16 191.63A88.39 88.39 0 0 1 40.37 144H80a142.39 142.39 0 0 0 32 71.63Zm-40-87.63H40a88 88 0 0 1 72-87.63A142.39 142.39 0 0 0 72 128Zm56 88a126.44 126.44 0 0 1-23.76-72h47.52A126.44 126.44 0 0 1 128 216Zm-23.76-88a126.44 126.44 0 0 1 23.76-72 126.44 126.44 0 0 1 23.76 72Zm39.76-72A88 88 0 0 1 216 128h-32a142.39 142.39 0 0 0-40-72Zm0 88h32a88.39 88.39 0 0 1-71.63 71.63A142.39 142.39 0 0 0 144 144Z"
          />
        </svg>
      {:else if kind === "redirect"}
        <svg
          class="size-3.5 shrink-0 text-[#296FF0]"
          viewBox="0 0 256 256"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            d="M224 104a8 8 0 0 1-16 0V59.31l-66.34 66.35a8 8 0 0 1-11.32-11.32L196.69 48H152a8 8 0 0 1 0-16h72a8 8 0 0 1 8 8Zm-40 40a8 8 0 0 0-8 8v56H48V80h56a8 8 0 0 0 0-16H48a16 16 0 0 0-16 16v128a16 16 0 0 0 16 16h128a16 16 0 0 0 16-16v-56a8 8 0 0 0-8-8Z"
          />
        </svg>
      {:else}
        <svg
          class="size-3.5 shrink-0 text-[#296FF0]"
          viewBox="0 0 256 256"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            d="M120 128a8 8 0 0 1-8 8H40a8 8 0 0 1 0-16h72a8 8 0 0 1 8 8Zm-69.66-42.34a8 8 0 0 0 11.32-11.32l-32-32a8 8 0 0 0-11.32 0l-32 32a8 8 0 0 0 11.32 11.32L24 59.31V96a8 8 0 0 0 16 0V59.31Zm173.66-45.66h-88a16 16 0 0 0-16 16v32a8 8 0 0 0 16 0V56h88v144h-88v-32a8 8 0 0 0-16 0v32a16 16 0 0 0 16 16h88a16 16 0 0 0 16-16V56a16 16 0 0 0-16-16Z"
          />
        </svg>
      {/if}
      <span class="text-xs font-semibold text-white tracking-tight truncate"
        >{title}</span
      >
    </div>
  </div>

  <div class="flex items-center w-full">
    <input
      type="text"
      bind:value={inputValue}
      {placeholder}
      {disabled}
      onkeydown={handleKeydown}
      onpaste={handlePaste}
      class="h-8 flex-1 min-w-0 rounded-l-md border border-r-0 border-brand-border-hover bg-brand-surface-input px-2.5 font-mono text-xs text-brand-text-primary placeholder:text-brand-text-muted outline-none focus:border-[#296FF0] focus:ring-1 focus:ring-[#296FF0]"
    />
    <button
      type="button"
      class="action-button h-8 shrink-0 rounded-r-md px-2.5 text-xs font-semibold inline-flex items-center gap-1"
      {disabled}
      onclick={handleAddClick}
    >
      <svg
        width="12"
        height="12"
        viewBox="0 0 12 12"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M6 2V10M2 6H10"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
        />
      </svg>
      <span>Add</span>
    </button>
  </div>

  <div
    class="uri-list min-h-24 max-h-38 overflow-y-auto overscroll-contain rounded-md border border-brand-border bg-brand-surface/70 p-1.5 flex flex-col gap-1.5"
  >
    {#if items.length === 0}
      <div
        class="flex flex-col items-center justify-center h-full py-4 text-center"
      >
        <span class="text-xs text-brand-text-muted font-normal"
          >No URIs added</span
        >
        <span class="text-[10px] text-brand-text-subtle mt-0.5"
          >Type above and click Add or press Enter</span
        >
      </div>
    {:else}
      {#each items as item, index (item + index)}
        <div
          class="flex items-center justify-between gap-1.5 rounded border border-brand-border bg-brand-surface-card px-2 py-1 text-xs hover:border-brand-border-hover transition-colors"
        >
          <span
            class="font-mono text-xs text-white truncate min-w-0 flex-1"
            title={item}
          >
            {item}
          </span>
          <div class="flex items-center gap-0.5 shrink-0">
            <CopyButton value={item} label={`Copy ${item}`} />
            <DeleteButton
              label={`Remove ${item}`}
              {disabled}
              onclick={() => removeItem(index)}
            />
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>
