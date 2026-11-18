<script lang="ts">
  import type { Snippet } from "svelte";
  import Button from "./Button.svelte";
  import SignetLogo from "./SignetLogo.svelte";
  import SignetMark from "./SignetMark.svelte";

  interface Props {
    brandName?: string;
    logoUrl?: string | null;
    footerText?: string;
    topActionLabel?: string;
    topActionHref?: string;
    onTopAction?: () => void;
    compact?: boolean;
    children: Snippet;
  }

  let {
    brandName = "Signet",
    logoUrl = null,
    footerText = "Signet · Authentication infrastructure for every application.",
    topActionLabel,
    topActionHref,
    onTopAction,
    compact = false,
    children,
  }: Props = $props();
</script>

<div
  class="grid grid-cols-1 lg:grid-cols-2 w-full bg-black text-zinc-100 {compact ? 'min-h-[460px] border border-[#23262e] rounded-xl overflow-hidden' : 'min-h-screen'}"
>
  <aside
    class="relative bg-black border-b lg:border-b-0 lg:border-r border-[#1b1b1d] flex flex-col justify-between overflow-hidden {compact ? 'p-6 min-h-[200px]' : 'p-8 lg:p-10 min-h-[220px] lg:min-h-screen'}"
    aria-label="Brand showcase"
  >
    <div class="relative z-10">
      {#if logoUrl}
        <img src={logoUrl} alt={brandName} class="max-h-[30px] w-auto block" />
      {:else}
        <SignetLogo variant="light" height={28} label={brandName} />
      {/if}
    </div>

    <div class="relative flex-1 flex items-center justify-center min-h-[140px] lg:min-h-[240px]" aria-hidden="true">
      <div class="absolute w-px h-[200%] left-1/2 -top-1/2 bg-[#1a1e16] pointer-events-none"></div>
      <div class="absolute h-px w-[200%] top-1/2 -left-1/2 bg-[#1a1e16] pointer-events-none"></div>
      <div class="relative z-10 flex flex-col items-center text-center gap-5 max-w-xs">
        <div class="text-white">
          <SignetMark size={compact ? 76 : 116} />
        </div>
        <p class="font-heading text-sm font-medium leading-snug text-[#d8decf] tracking-tight m-0">
          Authentication infrastructure for every application.
        </p>
      </div>
    </div>

    <div class="relative z-10 text-xs text-zinc-500 tracking-wide">
      <span>{footerText}</span>
    </div>
  </aside>

  <section class="bg-[#111113] flex flex-col justify-between relative {compact ? 'p-6 lg:p-8' : 'p-6 sm:p-10 lg:p-14'}">
    <div class="flex justify-end min-h-[1.5rem]">
      {#if topActionLabel && topActionHref}
        <Button size="sm" href={topActionHref}>{topActionLabel}</Button>
      {:else if topActionLabel && onTopAction}
        <Button size="sm" onclick={onTopAction}>{topActionLabel}</Button>
      {/if}
    </div>

    <div class="flex-1 flex flex-col justify-center max-w-lg w-full mx-auto my-6 lg:my-0">
      {@render children()}
    </div>
  </section>
</div>
