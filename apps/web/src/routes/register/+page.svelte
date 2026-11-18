<script lang="ts">
  import AuthSplitShell from "$lib/components/AuthSplitShell.svelte";
  import { authApi } from "$lib/api/auth";
  import { setAuth, authStore } from "$lib/stores/auth.svelte";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";
  import { goto } from "$app/navigation";
  import { onDestroy, tick } from "svelte";
  import OtpInput from "$lib/components/OtpInput.svelte";

  $effect(() => {
    if (authStore.initialized && authStore.isAuthenticated) {
      goto("/dashboard");
    }
  });

  let step = $state<"form" | "otp">("form");
  let email = $state("");
  let password = $state("");
  let displayName = $state("");
  let confirmPassword = $state("");
  let showPassword = $state(false);
  let loading = $state(false);
  let verifyLoading = $state(false);
  let resending = $state(false);
  let errors = $state<Record<string, string>>({});

  let otpCode = $state("");
  let otpInput = $state<ReturnType<typeof OtpInput>>();
  let resendCooldown = $state(60);
  let timerInterval: ReturnType<typeof setInterval> | null = null;

  function startResendTimer() {
    if (timerInterval) clearInterval(timerInterval);
    resendCooldown = 60;
    timerInterval = setInterval(() => {
      if (resendCooldown > 0) {
        resendCooldown--;
      } else if (timerInterval) {
        clearInterval(timerInterval);
        timerInterval = null;
      }
    }, 1000);
  }

  onDestroy(() => {
    if (timerInterval) clearInterval(timerInterval);
  });

  function validate(): boolean {
    errors = {};
    if (!displayName.trim()) errors.displayName = "Display name is required";
    else if (displayName.trim().length < 2)
      errors.displayName = "Minimum 2 characters";

    if (!email.trim()) errors.email = "Email is required";
    else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email))
      errors.email = "Invalid email address";

    if (!password) errors.password = "Password is required";
    else if (password.length < 8) errors.password = "Minimum 8 characters";
    else if (!/[A-Z]/.test(password))
      errors.password = "Include at least one uppercase letter";
    else if (!/[0-9]/.test(password))
      errors.password = "Include at least one number";

    if (password !== confirmPassword)
      errors.confirmPassword = "Passwords do not match";
    return Object.keys(errors).length === 0;
  }

  async function handleInitiateRegister(e: SubmitEvent) {
    e.preventDefault();
    if (!validate()) return;

    loading = true;
    errors = {};
    try {
      await authApi.register({
        email: email.trim().toLowerCase(),
        password,
        display_name: displayName.trim(),
      });
      otpCode = "";
      step = "otp";
      startResendTimer();
      await tick();
      otpInput?.focus();
    } catch (err) {
      if (err instanceof ApiError) {
        if (err.status === 409)
          errors.email = "An account with this email already exists";
        else errors.form = err.message;
      } else {
        errors.form = "An unexpected error occurred. Please try again.";
      }
    } finally {
      loading = false;
    }
  }

  async function handleVerifyOtp(code = otpCode) {
    if (verifyLoading || resending) return;
    if (code.length !== 6) {
      errors.otp = "Enter all 6 verification digits";
      return;
    }

    verifyLoading = true;
    errors = {};
    try {
      const session = await authApi.verifyRegisterOtp({
        email: email.trim().toLowerCase(),
        code,
      });
      setAuth(session.user);
      addToast("Account verified and created", "success");
      goto("/dashboard");
    } catch (err) {
      errors.otp =
        err instanceof ApiError ? err.message : "Verification failed";
    } finally {
      verifyLoading = false;
      if (errors.otp) {
        await tick();
        otpInput?.focus();
      }
    }
  }

  async function handleResendOtp() {
    if (resendCooldown > 0 || resending || verifyLoading) return;
    resending = true;
    errors = {};
    try {
      await authApi.resendRegisterOtp({ email: email.trim().toLowerCase() });
      addToast("Verification code resent", "info");
      otpCode = "";
      startResendTimer();
      await tick();
    } catch (err) {
      errors.otp =
        err instanceof ApiError ? err.message : "Failed to resend code";
    } finally {
      resending = false;
      await tick();
      otpInput?.focus();
    }
  }

</script>

<svelte:head>
  <title>{step === "form" ? "Create Account" : "Verify Email"} — Signet</title>
</svelte:head>

<AuthSplitShell
  brandName="Signet"
  topActionLabel="Sign in"
  topActionHref="/login"
>
  {#if step === "form"}
    <h1 class="editorial-title">Create your account</h1>
    <p class="editorial-subtitle">
      Register a central identity with email verification.
    </p>

    {#if errors.form}
      <div class="editorial-alert" role="alert">{errors.form}</div>
    {/if}

    <form class="editorial-stack" onsubmit={handleInitiateRegister} novalidate>
      <div class="editorial-field">
        <label class="editorial-label" for="reg-name">Full name *</label>
        <input
          id="reg-name"
          type="text"
          class="editorial-input"
          class:has-error={Boolean(errors.displayName)}
          bind:value={displayName}
          placeholder="Andrew Thomas"
          autocomplete="name"
          required
        />
        {#if errors.displayName}
          <span class="editorial-field-error" role="alert"
            >{errors.displayName}</span
          >
        {/if}
      </div>

      <div class="editorial-field">
        <label class="editorial-label" for="reg-email">Email address *</label>
        <input
          id="reg-email"
          type="email"
          class="editorial-input"
          class:has-error={Boolean(errors.email)}
          bind:value={email}
          placeholder="e.g. andrew@example.com"
          autocomplete="email"
          required
        />
        {#if errors.email}
          <span class="editorial-field-error" role="alert"
            >{errors.email}</span
          >
        {/if}
      </div>

      <div class="editorial-grid-2">
        <div class="editorial-field">
          <label class="editorial-label" for="reg-password">Password</label>
          <div class="editorial-input-wrap">
            <input
              id="reg-password"
              type={showPassword ? "text" : "password"}
              class="editorial-input"
              class:has-error={Boolean(errors.password)}
              bind:value={password}
              placeholder="••••••••••••"
              autocomplete="new-password"
              required
            />
            <button
              type="button"
              class="action-button inline-grid size-8 shrink-0 place-items-center rounded-md absolute right-1.5 top-1/2 -translate-y-1/2"
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
          {#if errors.password}
            <span class="editorial-field-error" role="alert"
              >{errors.password}</span
            >
          {/if}
        </div>

        <div class="editorial-field">
          <label class="editorial-label" for="reg-confirm"
            >Confirm password</label
          >
          <div class="editorial-input-wrap">
            <input
              id="reg-confirm"
              type={showPassword ? "text" : "password"}
              class="editorial-input"
              class:has-error={Boolean(errors.confirmPassword)}
              bind:value={confirmPassword}
              placeholder="••••••••••••"
              autocomplete="new-password"
              required
            />
          </div>
          {#if errors.confirmPassword}
            <span class="editorial-field-error" role="alert"
              >{errors.confirmPassword}</span
            >
          {/if}
        </div>
      </div>

      <p class="text-xs text-zinc-500 m-0 -mt-1">
        Password must be at least 8 characters, including a number and a special character.
      </p>

      <div class="editorial-actions-footer">
        <button
          type="submit"
          class="action-button w-full min-h-11 shrink-0 inline-flex items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold cursor-pointer"
          disabled={loading}
        >
          {loading ? "..." : "Create account"}
        </button>

        <div class="text-center text-sm text-zinc-400">
          Already have an account? <a href="/login" class="text-white hover:underline font-medium">Sign In</a>
        </div>

        <p class="text-xs text-zinc-500 text-center m-0">
          By creating an account, you agree to our Terms of Service and Privacy Policy.
        </p>
      </div>
    </form>
  {:else}
    <h1 class="editorial-title">Verify Email</h1>
    <p class="editorial-subtitle">Enter the 6-digit code sent to {email}</p>

    {#if errors.otp}
      <div class="editorial-alert" role="alert">{errors.otp}</div>
    {/if}

    <form onsubmit={(event) => { event.preventDefault(); void handleVerifyOtp(); }} novalidate>
      <OtpInput bind:this={otpInput} bind:value={otpCode} disabled={verifyLoading || resending} invalid={Boolean(errors.otp)} oncomplete={handleVerifyOtp} />

      <div class="editorial-actions-footer">
        <div class="flex flex-col gap-1.5">
          {#if resendCooldown > 0}
            <span class="text-xs text-zinc-400 font-mono"
              >Resend in 0:{resendCooldown < 10
                ? "0"
                : ""}{resendCooldown}</span
            >
          {:else}
            <button
              type="button"
              class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold"
              onclick={handleResendOtp}
              disabled={resending}
            >
              {resending ? "Sending..." : "Resend code"}
            </button>
          {/if}
          <button
            type="button"
            class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold"
            onclick={() => {
              step = "form";
              errors = {};
              otpCode = "";
            }}
          >
            Edit details
          </button>
        </div>

        <button
          type="submit"
          class="action-button inline-flex min-h-10 shrink-0 items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold"
          disabled={verifyLoading || resending}
        >
          {verifyLoading ? "..." : "VERIFY"}
        </button>
      </div>
    </form>
  {/if}
</AuthSplitShell>
