<script lang="ts">
  import { fly } from "svelte/transition";
  import { onMount } from "svelte";

  let {
    message,
    type,
    duration,
    onDismiss,
  }: {
    message: string;
    type: "success" | "error" | "warning" | "info";
    duration: number;
    onDismiss: () => void;
  } = $props();

  const styles = {
    success: {
      label: "Success",
      color: "text-emerald-400",
      border: "border-emerald-500/30",
      iconBg: "bg-emerald-500/15",
      bar: "bg-emerald-500",
    },
    error: {
      label: "Error",
      color: "text-red-400",
      border: "border-red-500/30",
      iconBg: "bg-red-500/15",
      bar: "bg-red-500",
    },
    warning: {
      label: "Warning",
      color: "text-amber-400",
      border: "border-amber-500/30",
      iconBg: "bg-amber-500/15",
      bar: "bg-amber-500",
    },
    info: {
      label: "Information",
      color: "text-[#296FF0]",
      border: "border-[#296FF0]/30",
      iconBg: "bg-[#296FF0]/15",
      bar: "bg-[#296FF0]",
    },
  };
  const style = $derived(styles[type]);
  let remaining = $state(0);
  let hovered = $state(false);
  let focused = $state(false);
  let motion = $state(false);

  onMount(() => {
    motion = !matchMedia("(prefers-reduced-motion: reduce)").matches;
    remaining = duration;
    if (duration <= 0) return;

    let previous = performance.now();
    const timer = setInterval(() => {
      const now = performance.now();
      if (!hovered && !focused) {
        remaining = Math.max(0, remaining - (now - previous));
        if (remaining === 0) {
          clearInterval(timer);
          onDismiss();
        }
      }
      previous = now;
    }, 80);
    return () => clearInterval(timer);
  });
</script>

<li
  class="pointer-events-auto relative isolate min-w-0 overflow-hidden rounded-xl border border-brand-border bg-[#181818] text-white shadow-2xl transition-all"
  onpointerenter={() => (hovered = true)}
  onpointerleave={() => (hovered = false)}
  onfocusin={() => (focused = true)}
  onfocusout={(event) =>
    (focused =
      event.relatedTarget instanceof Node &&
      event.currentTarget.contains(event.relatedTarget))}
  transition:fly={{ y: motion ? 12 : 0, duration: motion ? 180 : 0 }}
>
  <div class="flex min-w-0 items-start gap-3 p-4">
    <span
      class="grid size-8 shrink-0 place-items-center rounded-lg {style.iconBg} {style.color}"
      aria-hidden="true"
    >
      {#if type === "success"}
        <svg
          class="size-4"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M3.5 8.5L6.5 11.5L12.5 4.5" />
        </svg>
      {:else if type === "error"}
        <svg
          class="size-4"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M4 4L12 12M12 4L4 12" />
        </svg>
      {:else if type === "warning"}
        <svg
          class="size-4"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M8 3V9M8 12.5V13" />
        </svg>
      {:else}
        <svg
          class="size-4"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M8 7.5V12M8 4V4.5" />
        </svg>
      {/if}
    </span>
    <div
      class="min-w-0 flex-1 pt-0.5"
      role={type === "error" ? "alert" : "status"}
    >
      <p class="text-xs font-semibold {style.color}">{style.label}</p>
      <p
        class="mt-0.5 text-sm leading-snug text-brand-text-primary wrap-break-word"
      >
        {message}
      </p>
    </div>
    <button
      type="button"
      class="inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded text-zinc-400 hover:text-white transition-colors bg-transparent border-0 p-0"
      onclick={onDismiss}
      aria-label="Dismiss notification"
    >
      <svg
        class="size-3.5"
        viewBox="0 0 14 14"
        fill="none"
        stroke="currentColor"
        stroke-width="1.8"
        stroke-linecap="round"
      >
        <path d="M2 2L12 12M12 2L2 12" />
      </svg>
    </button>
  </div>
  {#if duration > 0}
    <div class="absolute inset-x-0 bottom-0 h-0.5 w-full bg-zinc-800">
      <div
        class="h-full {style.bar} transition-all ease-linear"
        style="width: {(remaining / duration) * 100}%"
      ></div>
    </div>
  {/if}
</li>
