<script lang="ts">
  import Button from "./Button.svelte";

  let {
    application,
    account,
    scopes,
    pending = false,
    onDecision,
  }: {
    application: string;
    account?: string;
    scopes: { title: string; description: string }[];
    pending?: boolean;
    onDecision: (approved: boolean) => void;
  } = $props();
</script>

<section
  aria-label="Access request"
  aria-busy={pending}
  class="task-surface grid min-w-0 gap-5 rounded-2xl border border-brand-border bg-brand-surface-card p-5 sm:p-6 shadow-xl"
>
  <div
    class="flex min-w-0 items-center justify-between gap-4 border-b border-brand-border/60 pb-5"
  >
    <div class="flex min-w-0 items-center gap-3.5">
      <span
        class="grid size-12 shrink-0 place-items-center rounded-xl border border-[#296FF0]/40 bg-[#296FF0]/15 text-[#296FF0]"
        aria-hidden="true"
      >
        <svg
          class="size-6"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <rect x="5" y="11" width="14" height="10" rx="2" />
          <path d="M8 11V7a4 4 0 0 1 8 0v4" />
        </svg>
      </span>
      <div class="min-w-0">
        <h2 class="font-heading text-lg font-bold text-white wrap-break-word">
          {application}
        </h2>
        {#if account}
          <p class="mt-0.5 text-xs text-brand-text-secondary truncate">
            Signed in as <strong class="text-white">{account}</strong>
          </p>
        {/if}
      </div>
    </div>
    <span
      class="hidden sm:inline-flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-1 text-[11px] font-medium text-emerald-400"
    >
      <span class="size-1.5 rounded-full bg-emerald-400"></span>
      OAuth 2.0
    </span>
  </div>

  <div class="grid min-w-0 gap-3">
    <div class="flex items-center justify-between">
      <span
        class="text-xs font-semibold tracking-wider text-brand-text-muted uppercase"
        >Requested Permissions</span
      >
      <span class="text-xs text-brand-text-secondary"
        >{scopes.length} {scopes.length === 1 ? "scope" : "scopes"}</span
      >
    </div>

    {#if scopes.length > 0}
      <ul class="m-0 grid list-none gap-2 p-0">
        {#each scopes as scope}
          <li
            class="flex items-start gap-3 rounded-xl border border-brand-border bg-brand-surface-input/60 p-3.5 transition-colors"
          >
            <span
              class="mt-0.5 grid size-5 shrink-0 place-items-center rounded-full bg-[#296FF0]/15 text-[#296FF0]"
              aria-hidden="true"
            >
              <svg
                class="size-3"
                viewBox="0 0 12 12"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M2.5 6.5L4.5 8.5L9.5 3.5" />
              </svg>
            </span>
            <div class="grid min-w-0 gap-0.5">
              <strong class="text-sm font-semibold text-white"
                >{scope.title}</strong
              >
              <span class="text-xs leading-relaxed text-brand-text-secondary"
                >{scope.description}</span
              >
            </div>
          </li>
        {/each}
      </ul>
    {:else}
      <p
        class="rounded-xl border border-brand-border bg-brand-surface-input/60 p-3.5 text-xs text-brand-text-secondary"
      >
        No additional permissions requested.
      </p>
    {/if}
  </div>

  <div
    class="flex flex-wrap items-center justify-end gap-3 border-t border-brand-border/60 pt-4"
  >
    <button
      type="button"
      class="inline-flex min-h-10 cursor-pointer items-center justify-center rounded-lg border border-brand-border bg-transparent px-4 py-2 text-sm font-semibold text-brand-text-secondary transition-colors hover:border-brand-border-strong hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
      disabled={pending}
      onclick={() => onDecision(false)}
    >
      Decline
    </button>
    <Button disabled={pending} onclick={() => onDecision(true)}>
      Approve Access
    </Button>
  </div>
  {#if pending}
    <p role="status" class="text-right text-xs text-brand-text-secondary">
      Saving your decision…
    </p>
  {/if}
</section>
