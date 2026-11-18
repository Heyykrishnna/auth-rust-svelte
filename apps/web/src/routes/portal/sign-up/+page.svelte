<script lang="ts">
  import { page } from "$app/state";
  import { onMount, tick } from "svelte";
  import OtpInput from "$lib/components/OtpInput.svelte";
  import AuthSplitShell from "$lib/components/AuthSplitShell.svelte";
  import { portalApi, type PortalTransactionContext } from "$lib/api/portal";
  import {
    isGlobalEmailCollisionError,
    resolvePortalBrandTheme,
  } from "$lib/portal/state";

  let txContext = $state<PortalTransactionContext | null>(null);
  let loading = $state(true);
  let submitting = $state(false);
  let step = $state<"form" | "otp">("form");
  let errorMessage = $state("");
  let emailCollision = $state(false);

  let displayName = $state("");
  let email = $state("");
  let password = $state("");
  let otpCode = $state("");
  let otpInput = $state<ReturnType<typeof OtpInput>>();

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
    } catch (err) {
      errorMessage =
        err instanceof Error
          ? err.message
          : "Unable to load registration session.";
    } finally {
      loading = false;
    }
  });

  async function handleSignUp(e: SubmitEvent) {
    e.preventDefault();
    if (!txContext || submitting) return;
    errorMessage = "";
    emailCollision = false;
    submitting = true;
    try {
      await portalApi.signUp(txContext.tx_id, {
        csrf_token: txContext.csrf_token,
        display_name: displayName,
        email,
        password,
      });
      otpCode = "";
      step = "otp";
      await tick();
      otpInput?.focus();
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Registration failed.";
      errorMessage = msg;
      emailCollision = isGlobalEmailCollisionError(msg);
    } finally {
      submitting = false;
    }
  }

  async function handleVerifyOtp(e: SubmitEvent) {
    e.preventDefault();
    if (!txContext || submitting) return;
    errorMessage = "";
    submitting = true;
    try {
      const res = await portalApi.verifySignUp(txContext.tx_id, {
        csrf_token: txContext.csrf_token,
        email,
        code: otpCode,
      });
      if (res.redirect_url) {
        window.location.href = res.redirect_url;
      }
    } catch (err) {
      errorMessage =
        err instanceof Error ? err.message : "Verification failed.";
    } finally {
      submitting = false;
    }
  }
</script>

<svelte:head>
  <title>Create Account — {brand.appName}</title>
</svelte:head>

<AuthSplitShell
  brandName={brand.appName}
  logoUrl={brand.logoUrl}
  footerText="Central Identity Provider · OIDC + PKCE"
  topActionLabel={txContext ? "Sign in" : undefined}
  topActionHref={txContext
    ? `/portal/login?tx=${encodeURIComponent(txContext.tx_id)}`
    : undefined}
>
  {#if loading}
    <p class="editorial-subtitle">Loading registration session...</p>
  {:else if !txContext}
    <h1 class="editorial-title">Create Account</h1>
    <div class="editorial-alert" role="alert">
      {errorMessage || "Invalid registration link."}
    </div>
  {:else}
    <h1 class="editorial-title">
      {step === "form" ? "Create Account" : "Verify Email"}
    </h1>
    <p class="editorial-subtitle">
      {step === "form"
        ? `Register once to access ${brand.appName} and connected applications.`
        : `Enter the 6-digit verification code sent to ${email}.`}
    </p>

    {#if errorMessage}
      <div class="editorial-alert" role="alert">
        <div>{errorMessage}</div>
        {#if emailCollision}
          <a
            class="inline-block mt-2 text-white underline"
            href={`/portal/login?tx=${encodeURIComponent(txContext.tx_id)}&email=${encodeURIComponent(email)}`}
          >
            Sign in with your existing account
          </a>
        {/if}
      </div>
    {/if}

    {#if step === "form"}
      <form class="editorial-stack" onsubmit={handleSignUp}>
        <div class="editorial-field">
          <label class="editorial-label" for="ps-name">Full name *</label>
          <input
            id="ps-name"
            type="text"
            class="editorial-input"
            bind:value={displayName}
            required
            placeholder="Alice Carter"
          />
        </div>

        <div class="editorial-field">
          <label class="editorial-label" for="ps-email">Email address *</label>
          <input
            id="ps-email"
            type="email"
            class="editorial-input"
            bind:value={email}
            required
            autocomplete="email"
            placeholder="e.g. alice@company.com"
          />
        </div>

        <div class="editorial-field">
          <label class="editorial-label" for="ps-password"
            >Password *</label
          >
          <input
            id="ps-password"
            type="password"
            class="editorial-input"
            bind:value={password}
            required
            minlength="8"
            autocomplete="new-password"
            placeholder="••••••••••••"
          />
        </div>

        <p class="text-xs text-zinc-500 m-0 -mt-1">
          Password must be at least 8 characters, including a number and a special character.
        </p>

        <div class="editorial-actions-footer">
          <button
            type="submit"
            class="action-button w-full min-h-11 shrink-0 inline-flex items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold cursor-pointer"
            disabled={submitting}
          >
            {submitting ? "..." : "Create account"}
          </button>

          <div class="text-center text-sm text-zinc-400">
            Already have an account? <a href={`/portal/login?tx=${encodeURIComponent(txContext.tx_id)}`} class="text-white hover:underline font-medium">Sign In</a>
          </div>

          <p class="text-xs text-zinc-500 text-center m-0">
            By creating an account, you agree to our Terms of Service and Privacy Policy.
          </p>
        </div>
      </form>
    {:else}
      <form class="editorial-stack" onsubmit={handleVerifyOtp}>
        <OtpInput bind:this={otpInput} bind:value={otpCode} disabled={submitting} invalid={Boolean(errorMessage)} />

        <div class="editorial-actions-footer">
          <button
            type="button"
            class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold"
            onclick={() => {
              step = "form";
              errorMessage = "";
            }}
          >
            Back to details
          </button>
          <button
            type="submit"
            class="action-button inline-flex min-h-10 shrink-0 items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold"
            disabled={submitting}
          >
            {submitting ? "..." : "VERIFY"}
          </button>
        </div>
      </form>
    {/if}
  {/if}
</AuthSplitShell>
