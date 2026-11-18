<script lang="ts">
  import { onMount } from "svelte";
  import AuthSplitShell from "$lib/components/AuthSplitShell.svelte";
  import { portalApi, type PortalAccountView } from "$lib/api/portal";

  let account = $state<PortalAccountView | null>(null);
  let loading = $state(true);
  let busy = $state(false);
  let message = $state("");
  let errorMessage = $state("");

  async function loadAccount() {
    loading = true;
    errorMessage = "";
    try {
      account = await portalApi.getAccount();
    } catch (err) {
      account = null;
      errorMessage =
        err instanceof Error ? err.message : "Not signed in to Central IdP.";
    } finally {
      loading = false;
    }
  }

  onMount(loadAccount);

  async function handleSignOut(global: boolean) {
    busy = true;
    errorMessage = "";
    try {
      const res = await portalApi.logout({ global });
      message = res.global_logout
        ? `Signed out everywhere (${res.revoked_app_sessions} app sessions revoked).`
        : "Local sign-out completed.";
      if (res.global_logout) account = null;
      else await loadAccount();
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : "Sign-out failed.";
    } finally {
      busy = false;
    }
  }

  async function handleRevokeSso(ssoId: string) {
    busy = true;
    errorMessage = "";
    try {
      await portalApi.revokeSsoSession(ssoId);
      await loadAccount();
    } catch (err) {
      errorMessage =
        err instanceof Error ? err.message : "Failed to revoke SSO session.";
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head>
  <title>SSO Account — Signet</title>
</svelte:head>

<AuthSplitShell
  brandName="Signet"
  footerText="Signet Central Identity Provider · Session Control"
  topActionLabel="Control Plane"
  topActionHref="/control"
>
  <h1 class="editorial-title">SSO Sessions</h1>
  <p class="editorial-subtitle">
    Inspect connected applications and revoke active sessions.
  </p>

  {#if message}
    <div class="p-3.5 border border-emerald-900/50 bg-emerald-950/20 text-emerald-400 rounded-md text-xs mb-5" role="status">{message}</div>
  {/if}
  {#if errorMessage}
    <div class="editorial-alert" role="alert">{errorMessage}</div>
  {/if}

  {#if loading}
    <p class="editorial-subtitle">Checking active SSO session...</p>
  {:else if account}
    <div class="flex flex-col pb-4 mb-5 border-b border-[#23262e]">
      <strong class="text-white text-sm font-semibold">{account.user.display_name}</strong>
      <span class="text-zinc-400 text-xs">{account.user.email}</span>
    </div>

    <div class="mb-5">
      <h2 class="text-xs uppercase tracking-wider text-zinc-400 mb-2 font-medium">
        Connected Applications ({account.connected_apps.length})
      </h2>
      {#if account.connected_apps.length === 0}
        <p class="text-xs text-zinc-400 m-0">No connected applications.</p>
      {:else}
        <ul class="list-none p-0 m-0 border-t border-[#23262e]">
          {#each account.connected_apps as app}
            <li class="flex justify-between items-center py-2.5 border-b border-[#23262e]">
              <div class="flex flex-col">
                <strong class="text-zinc-100 text-sm font-medium">{app.application_name}</strong>
                <span class="text-zinc-400 text-xs">{app.environment_kind}</span>
              </div>
              <span class="text-xs text-zinc-300 border border-[#2e2e34] px-2 py-0.5 rounded">{app.active_sessions} active</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="mb-5">
      <h2 class="text-xs uppercase tracking-wider text-zinc-400 mb-2 font-medium">
        Active SSO Sessions ({account.sso_sessions.length})
      </h2>
      <ul class="list-none p-0 m-0 border-t border-[#23262e]">
        {#each account.sso_sessions as sess}
          <li class="flex justify-between items-center py-2.5 border-b border-[#23262e]">
            <div class="flex flex-col">
              <strong class="text-zinc-100 text-sm font-medium"
                >{sess.is_current ? "Current browser" : "Other device"}</strong
              >
              <span class="text-zinc-400 text-xs">{sess.ip_address ?? "Unknown IP"}</span>
            </div>
            <button
              type="button"
              class="bg-transparent border border-rose-900/60 text-rose-400 hover:border-rose-600 rounded px-2.5 py-1 text-xs cursor-pointer transition-colors"
              disabled={busy}
              onclick={() => handleRevokeSso(sess.id)}
            >
              Revoke
            </button>
          </li>
        {/each}
      </ul>
    </div>

    <div class="editorial-actions-footer">
      <span class="text-xs text-zinc-400">Global sign-out revokes all app sessions</span>
      <button
        type="button"
        class="editorial-circle-cta"
        disabled={busy}
        onclick={() => handleSignOut(true)}
      >
        {busy ? "..." : "SIGN OUT"}
      </button>
    </div>
  {/if}
</AuthSplitShell>
