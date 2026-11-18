<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";

  onMount(() => {
    goto("/control?section=analytics", { replaceState: true });
  });
</script>

<svelte:head>
  <title>Analytics — Signet Control Plane</title>
</svelte:head>

<div class="min-h-[60vh] flex items-center justify-center text-brand-text-secondary text-sm">
  <p class="m-0">Opening environment analytics...</p>
</div>
