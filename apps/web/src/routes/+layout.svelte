<script lang="ts">
  import "@fontsource-variable/inter";
  import "@fontsource-variable/manrope";
  import "@fontsource-variable/jetbrains-mono";
  import "../app.css";
  import { page } from "$app/state";
  import Navbar from "$lib/components/Navbar.svelte";
  import Toast from "$lib/components/Toast.svelte";
  import { initAuth, syncServerUser } from "$lib/stores/auth.svelte";
  import { onMount } from "svelte";
  import type { PageData } from "./$types";

  let {
    data,
    children,
  }: { data: PageData; children: import("svelte").Snippet } = $props();

  const isFullBleedRoute = $derived(
    page.url.pathname.startsWith("/control") ||
      page.url.pathname.startsWith("/portal") ||
      page.url.pathname === "/login" ||
      page.url.pathname === "/register",
  );

  $effect(() => {
    if (data?.user !== undefined) {
      syncServerUser(data.user);
    }
  });

  onMount(() => {
    initAuth();
  });
</script>

<div class="app-bg" aria-hidden="true"></div>

{#if isFullBleedRoute}
  <div class="min-h-screen w-full">
    {@render children()}
  </div>
{:else}
  <div class="flex flex-col min-h-screen relative z-10">
    <Navbar />

    <main class="flex-1 flex flex-col">
      {@render children()}
    </main>

    <footer class="border-t border-brand-border px-6 py-5 text-center text-xs text-brand-text-subtle bg-brand-surface">
      <div class="max-w-7xl mx-auto">
        <p class="m-0 text-brand-text-subtle">Signet — Authentication infrastructure for every application.</p>
      </div>
    </footer>
  </div>
{/if}

<Toast monochrome={page.url.pathname.startsWith("/control")} />
