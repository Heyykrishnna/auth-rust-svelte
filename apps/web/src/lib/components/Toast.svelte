<script module lang="ts">
  type ToastType = "success" | "error" | "warning" | "info";

  interface Toast {
    id: string;
    type: ToastType;
    message: string;
    duration: number;
  }

  let toasts = $state<Toast[]>([]);

  export function addToast(
    message: string,
    type: ToastType = "info",
    duration = 4000,
  ) {
    const id = crypto.randomUUID();
    toasts = [...toasts.slice(-4), { id, type, message, duration }];
    return id;
  }

  export function removeToast(id: string) {
    toasts = toasts.filter((toast) => toast.id !== id);
  }
</script>

<script lang="ts">
  import ToastItem from "./ToastItem.svelte";

  let { monochrome = false }: { monochrome?: boolean } = $props();
</script>

<section
  aria-label="Notifications"
  class="pointer-events-none fixed inset-x-3 bottom-3 z-50 sm:right-5 sm:bottom-5 sm:left-auto sm:w-96"
>
  <ol class="m-0 grid list-none gap-2.5 p-0">
    {#each toasts as { id, message, type, duration } (id)}
      <ToastItem
        {message}
        {type}
        {duration}
        {monochrome}
        onDismiss={() => removeToast(id)}
      />
    {/each}
  </ol>
</section>
