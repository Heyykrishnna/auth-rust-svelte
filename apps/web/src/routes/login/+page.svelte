<script lang="ts">
  import SelectionRow from '$lib/components/SelectionRow.svelte';
  import AuthSplitShell from "$lib/components/AuthSplitShell.svelte";
  import { authApi } from "$lib/api/auth";
  import { setAuth, authStore } from "$lib/stores/auth.svelte";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";
  import { goto } from "$app/navigation";

  $effect(() => {
    if (authStore.initialized && authStore.isAuthenticated) {
      goto("/dashboard");
    }
  });

  let email = $state("");
  let password = $state("");
  let rememberMe = $state(true);
  let showPassword = $state(false);
  let loading = $state(false);
  let errors = $state<{ email?: string; password?: string; form?: string }>({});

  function validate(): boolean {
    errors = {};
    if (!email.trim()) errors.email = "Email is required";
    else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email))
      errors.email = "Invalid email address";
    if (!password) errors.password = "Password is required";
    else if (password.length < 8) errors.password = "Minimum 8 characters";
    return Object.keys(errors).length === 0;
  }

  async function handleLogin(e: SubmitEvent) {
    e.preventDefault();
    if (!validate()) return;

    loading = true;
    try {
      const session = await authApi.login({ email: email.trim(), password });
      setAuth(session.user);
      addToast("Signed in successfully", "success");
      goto("/dashboard");
    } catch (err) {
      if (err instanceof ApiError) {
        errors.form =
          err.status === 401 ? "Invalid email or password" : err.message;
      } else {
        errors.form = "An unexpected error occurred. Please try again.";
      }
    } finally {
      loading = false;
    }
  }

  async function loginWithOidc(provider: "google" | "github") {
    try {
      const { authorization_url } = await authApi.getOidcUrl(provider);
      window.location.href = authorization_url;
    } catch {
      addToast("Failed to initiate OAuth sign-in", "error");
    }
  }
</script>

<svelte:head>
  <title>Sign In — Signet</title>
  <meta name="description" content="Sign in to your Signet account." />
</svelte:head>

<AuthSplitShell
  brandName="Signet"
  topActionLabel="Create an account"
  topActionHref="/register"
>
  <h1 class="editorial-title">Login</h1>
  <p class="editorial-subtitle">
    Authenticate with your credentials or connected provider.
  </p>

  {#if errors.form}
    <div class="editorial-alert" role="alert">{errors.form}</div>
  {/if}

  <form onsubmit={handleLogin} novalidate>
    <div class="editorial-grid-2">
      <div class="editorial-field">
        <label class="editorial-label" for="login-email">Email</label>
        <div class="editorial-input-wrap">
          <input
            id="login-email"
            type="email"
            class="editorial-input"
            class:has-error={Boolean(errors.email)}
            bind:value={email}
            placeholder="name@company.com"
            autocomplete="email"
            required
          />
        </div>
        {#if errors.email}
          <span class="editorial-field-error" role="alert">{errors.email}</span>
        {/if}
      </div>

      <div class="editorial-field">
        <label class="editorial-label" for="login-password">Password</label>
        <div class="editorial-input-wrap">
          <input
            id="login-password"
            type={showPassword ? "text" : "password"}
            class="editorial-input"
            class:has-error={Boolean(errors.password)}
            bind:value={password}
            placeholder="••••••••••"
            autocomplete="current-password"
            required
          />
          <button
            type="button"
            class="action-button inline-grid size-8 shrink-0 place-items-center rounded-md absolute right-0 top-1/2 -translate-y-1/2"
            onclick={() => (showPassword = !showPassword)}
            aria-label={showPassword ? "Hide password" : "Show password"}
          >
            {#if showPassword}
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.6"
                aria-hidden="true"
              >
                <path
                  d="M17.94 17.94A10.94 10.94 0 0 1 12 20C7 20 2.73 16.11 1 12c.74-1.76 1.94-3.36 3.47-4.66"
                />
                <path
                  d="M9.9 4.24A10.94 10.94 0 0 1 12 4c5 0 9.27 3.89 11 8a11.05 11.05 0 0 1-2.16 3.19"
                />
                <line x1="1" y1="1" x2="23" y2="23" />
              </svg>
            {:else}
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
            {/if}
          </button>
        </div>
        {#if errors.password}
          <span class="editorial-field-error" role="alert"
            >{errors.password}</span
          >
        {/if}
      </div>
    </div>

    <div class="editorial-meta-row">
      <SelectionRow label="Remember me" bind:checked={rememberMe} />
      <a href="/control/login" class="editorial-quiet-link">Developer Console</a
      >
    </div>

    <div class="editorial-actions-footer">
      <div class="editorial-oauth-group">
        <button
          type="button"
          class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold"
          onclick={() => loginWithOidc("google")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="currentColor"
            aria-hidden="true"
          >
            <path
              d="M21.8 12.23c0-.72-.06-1.41-.18-2.08H12v3.94h5.49a4.7 4.7 0 0 1-2.04 3.08v2.56h3.3c1.93-1.78 3.05-4.4 3.05-7.5zM12 22c2.76 0 5.07-.91 6.76-2.47l-3.3-2.56c-.91.61-2.08.98-3.46.98-2.66 0-4.91-1.8-5.72-4.21H2.87v2.64A10.21 10.21 0 0 0 12 22zM6.28 13.74A6.13 6.13 0 0 1 5.96 12c0-.6.11-1.19.32-1.74V7.62H2.87A10.21 10.21 0 0 0 1.8 12c0 1.65.4 3.21 1.07 4.38l3.41-2.64zM12 6.05c1.5 0 2.85.52 3.91 1.53l2.93-2.93C17.06 2.99 14.75 2 12 2A10.21 10.21 0 0 0 2.87 7.62l3.41 2.64c.81-2.41 3.06-4.21 5.72-4.21z"
            />
          </svg>
          Google
        </button>
        <button
          type="button"
          class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold"
          onclick={() => loginWithOidc("github")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="currentColor"
            aria-hidden="true"
          >
            <path
              d="M12 2C6.48 2 2 6.48 2 12.02c0 4.42 2.87 8.18 6.84 9.5.5.09.68-.22.68-.48 0-.24-.01-.87-.01-1.7-2.78.6-3.37-1.34-3.37-1.34-.45-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.61.07-.61 1 .07 1.53 1.03 1.53 1.03.89 1.53 2.34 1.09 2.91.83.09-.65.35-1.09.64-1.34-2.22-.25-4.56-1.11-4.56-4.95 0-1.09.39-1.99 1.03-2.69-.1-.25-.45-1.27.1-2.65 0 0 .84-.27 2.75 1.03a9.56 9.56 0 0 1 5 0c1.91-1.3 2.75-1.03 2.75-1.03.55 1.38.2 2.4.1 2.65.64.7 1.03 1.6 1.03 2.69 0 3.85-2.34 4.7-4.57 4.94.36.31.68.92.68 1.86 0 1.34-.01 2.42-.01 2.75 0 .27.18.58.69.48A10.02 10.02 0 0 0 22 12.02C22 6.48 17.52 2 12 2z"
            />
          </svg>
          GitHub
        </button>
      </div>

      <button type="submit" class="action-button inline-flex min-h-10 shrink-0 items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold" disabled={loading}>
        {loading ? "..." : "SIGN IN"}
      </button>
    </div>
  </form>
</AuthSplitShell>
