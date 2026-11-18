<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import AuthSplitShell from "$lib/components/AuthSplitShell.svelte";
  import { portalApi, type PortalTransactionContext } from "$lib/api/portal";
  import { resolvePortalBrandTheme } from "$lib/portal/state";

  let txContext = $state<PortalTransactionContext | null>(null);
  let loading = $state(true);
  let submitting = $state(false);
  let showPassword = $state(false);
  let errorMessage = $state("");
  let email = $state("");
  let password = $state("");

  const txId = $derived(page.url.searchParams.get("tx") ?? "");
  const brand = $derived(
    txContext
      ? resolvePortalBrandTheme(txContext.branding, txContext.application_name)
      : resolvePortalBrandTheme(null, "Signet"),
  );

  onMount(async () => {
    if (!txId) {
      errorMessage = "Missing OIDC authorization transaction ID.";
      loading = false;
      return;
    }
    try {
      txContext = await portalApi.getTransaction(txId);
      const prefillEmail = page.url.searchParams.get("email");
      if (prefillEmail) email = prefillEmail;
    } catch (err) {
      errorMessage =
        err instanceof Error ? err.message : "Unable to load sign-in session.";
    } finally {
      loading = false;
    }
  });

  async function handleLogin(e: SubmitEvent) {
    e.preventDefault();
    if (!txContext) return;
    errorMessage = "";
    submitting = true;
    try {
      const res = await portalApi.login(txContext.tx_id, {
        csrf_token: txContext.csrf_token,
        email,
        password,
      });
      if (res.action === "consent") {
        await goto(`/portal/consent?tx=${encodeURIComponent(txContext.tx_id)}`);
      } else if (res.redirect_url) {
        window.location.href = res.redirect_url;
      }
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : "Sign-in failed.";
    } finally {
      submitting = false;
    }
  }
</script>

<svelte:head>
  <title>Sign in — {brand.appName}</title>
</svelte:head>

<AuthSplitShell
  brandName={brand.appName}
  logoUrl={brand.logoUrl}
  footerText="Central Identity Provider · OIDC + PKCE"
  topActionLabel={txContext ? "Create an account" : undefined}
  topActionHref={txContext
    ? `/portal/sign-up?tx=${encodeURIComponent(txContext.tx_id)}`
    : undefined}
>
  {#if loading}
    <p class="editorial-subtitle">Loading sign-in request...</p>
  {:else if !txContext}
    <h1 class="editorial-title">Sign In</h1>
    <div class="editorial-alert" role="alert">
      {errorMessage || "Invalid sign-in link."}
    </div>
  {:else}
    <h1 class="editorial-title">Login</h1>
    <p class="editorial-subtitle">
      Continue to {txContext.application_name} with your central identity.
    </p>

    {#if errorMessage}
      <div class="editorial-alert" role="alert">{errorMessage}</div>
    {/if}

    <form onsubmit={handleLogin}>
      <div class="editorial-grid-2">
        <div class="editorial-field">
          <label class="editorial-label" for="portal-email">Email</label>
          <input
            id="portal-email"
            type="email"
            class="editorial-input"
            bind:value={email}
            required
            autocomplete="email"
            placeholder="name@company.com"
          />
        </div>

        <div class="editorial-field">
          <label class="editorial-label" for="portal-password">Password</label>
          <div class="editorial-input-wrap">
            <input
              id="portal-password"
              type={showPassword ? "text" : "password"}
              class="editorial-input"
              bind:value={password}
              required
              autocomplete="current-password"
              placeholder="••••••••••"
            />
            <button
              type="button"
              class="action-button inline-grid size-8 shrink-0 place-items-center rounded-md absolute right-0 top-1/2 -translate-y-1/2"
              onclick={() => (showPassword = !showPassword)}
              aria-label={showPassword ? "Hide password" : "Show password"}
            >
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.6"
                aria-hidden="true"
              >
                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                <circle cx="12" cy="12" r="3" />
              </svg>
            </button>
          </div>
        </div>
      </div>

      <div class="editorial-meta-row">
        <a href="/portal/logout" class="editorial-quiet-link"
          >Manage SSO sessions</a
        >
        {#if txContext.social_providers.length > 0}
          <span class="editorial-quiet-link"
            >Providers: {txContext.social_providers.join(", ")}</span
          >
        {/if}
      </div>

      <div class="editorial-actions-footer">
        <a
          href={`/portal/sign-up?tx=${encodeURIComponent(txContext.tx_id)}`}
          class="editorial-quiet-link"
        >
          Create an account
        </a>

        <button
          type="submit"
          class="action-button inline-flex min-h-10 shrink-0 items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold"
          disabled={submitting}
        >
          {submitting ? "..." : "SIGN IN"}
        </button>
      </div>
    </form>
  {/if}
</AuthSplitShell>
