<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Input from "$lib/components/Input.svelte";
  import Button from "$lib/components/Button.svelte";
  import { authApi } from "$lib/api/auth";
  import { setAuth, authStore } from "$lib/stores/auth.svelte";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";
  import { goto } from "$app/navigation";
  import { onDestroy } from "svelte";

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
  let loading = $state(false);
  let verifyLoading = $state(false);
  let resending = $state(false);
  let errors = $state<Record<string, string>>({});
  let shakeError = $state(false);

  let otpDigits = $state<string[]>(["", "", "", "", "", ""]);
  let otpInputRefs: (HTMLInputElement | null)[] = [];
  let resendCooldown = $state(60);
  let timerInterval: ReturnType<typeof setInterval> | null = null;

  function triggerShake() {
    shakeError = true;
    setTimeout(() => {
      shakeError = false;
    }, 600);
  }

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
    else if (displayName.length < 2)
      errors.displayName = "Must be at least 2 characters";

    if (!email) errors.email = "Email is required";
    else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email))
      errors.email = "Invalid email address";

    if (!password) errors.password = "Password is required";
    else if (password.length < 8)
      errors.password = "Must be at least 8 characters";
    else if (!/[A-Z]/.test(password))
      errors.password = "Must contain at least one uppercase letter";
    else if (!/[0-9]/.test(password))
      errors.password = "Must contain at least one number";

    if (password !== confirmPassword)
      errors.confirmPassword = "Passwords do not match";

    return Object.keys(errors).length === 0;
  }

  let passwordStrength = $derived(() => {
    let score = 0;
    if (password.length >= 8) score++;
    if (password.length >= 12) score++;
    if (/[A-Z]/.test(password)) score++;
    if (/[0-9]/.test(password)) score++;
    if (/[^A-Za-z0-9]/.test(password)) score++;
    return score;
  });

  let strengthLabel = $derived(() => {
    const s = passwordStrength();
    if (s <= 1) return { text: "Weak", color: "var(--color-error)" };
    if (s <= 2) return { text: "Fair", color: "var(--color-warning)" };
    if (s <= 3) return { text: "Good", color: "#84cc16" };
    return { text: "Strong", color: "var(--color-success)" };
  });

  let fullOtp = $derived(otpDigits.join(""));

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
      step = "otp";
      startResendTimer();
      setTimeout(() => {
        otpInputRefs[0]?.focus();
      }, 100);
    } catch (err) {
      if (err instanceof ApiError) {
        if (err.status === 409) {
          errors.email = "An account with this email already exists";
        } else {
          errors.form = err.message;
        }
      } else {
        errors.form = "An unexpected error occurred. Please try again.";
      }
      triggerShake();
    } finally {
      loading = false;
    }
  }

  async function handleVerifyOtp(e?: SubmitEvent) {
    if (e) e.preventDefault();
    if (fullOtp.length !== 6) {
      errors.otp = "Please enter all 6 digits";
      triggerShake();
      return;
    }

    verifyLoading = true;
    errors = {};
    try {
      const session = await authApi.verifyRegisterOtp({
        email: email.trim().toLowerCase(),
        code: fullOtp,
      });
      setAuth(session.user);
      addToast("Account verified and created successfully! 🎉", "success");
      goto("/dashboard");
    } catch (err) {
      if (err instanceof ApiError) {
        errors.otp = err.message;
      } else {
        errors.otp = "Verification failed. Please try again.";
      }
      triggerShake();
    } finally {
      verifyLoading = false;
    }
  }

  async function handleResendOtp() {
    if (resendCooldown > 0 || resending) return;
    resending = true;
    errors = {};
    try {
      await authApi.resendRegisterOtp({ email: email.trim().toLowerCase() });
      addToast("A fresh verification code was sent to your email", "info");
      otpDigits = ["", "", "", "", "", ""];
      startResendTimer();
      otpInputRefs[0]?.focus();
    } catch (err) {
      if (err instanceof ApiError) {
        errors.otp = err.message;
      } else {
        errors.otp = "Failed to resend code. Please try again.";
      }
      triggerShake();
    } finally {
      resending = false;
    }
  }

  function handleOtpInput(index: number, e: Event) {
    const target = e.target as HTMLInputElement;
    const val = target.value.replace(/\D/g, "");
    otpDigits[index] = val ? val.slice(-1) : "";

    if (val && index < 5) {
      otpInputRefs[index + 1]?.focus();
    }

    if (otpDigits.every((d) => d.length === 1)) {
      handleVerifyOtp();
    }
  }

  function handleOtpKeyDown(index: number, e: KeyboardEvent) {
    if (e.key === "Backspace") {
      if (!otpDigits[index] && index > 0) {
        otpDigits[index - 1] = "";
        otpInputRefs[index - 1]?.focus();
      } else {
        otpDigits[index] = "";
      }
    } else if (e.key === "ArrowLeft" && index > 0) {
      otpInputRefs[index - 1]?.focus();
    } else if (e.key === "ArrowRight" && index < 5) {
      otpInputRefs[index + 1]?.focus();
    }
  }

  function handleOtpPaste(e: ClipboardEvent) {
    e.preventDefault();
    const clipboardData = e.clipboardData?.getData("text") || "";
    const digits = clipboardData.replace(/\D/g, "").slice(0, 6).split("");
    if (digits.length === 0) return;

    for (let i = 0; i < 6; i++) {
      otpDigits[i] = digits[i] || "";
    }

    const nextFocusIndex = Math.min(digits.length, 5);
    otpInputRefs[nextFocusIndex]?.focus();

    if (digits.length === 6) {
      handleVerifyOtp();
    }
  }

  function handleBackToForm() {
    step = "form";
    errors = {};
  }
</script>

<svelte:head>
  <title>{step === "form" ? "Create Account" : "Verify Email"} — Dradix</title>
  <meta
    name="description"
    content="Secure registration with email OTP verification."
  />
</svelte:head>

<div class="auth-layout">
  <div class="auth-card glass-card" class:shake={shakeError}>
    <div class="auth-logo">
      <div class="auth-logo-icon">🔐</div>
      <span class="auth-logo-text">Dradix</span>
    </div>

    {#if step === "form"}
      <div
        in:fly={{ y: 20, duration: 300, easing: cubicOut }}
        out:fade={{ duration: 150 }}
      >
        <h1 class="auth-title">Create an account</h1>
        <p class="auth-subtitle">Get started with secure authentication</p>

        <form class="auth-form" onsubmit={handleInitiateRegister} novalidate>
          {#if errors.form}
            <div class="form-error" role="alert">⚠ {errors.form}</div>
          {/if}

          <Input
            id="displayName"
            label="Display name"
            type="text"
            placeholder="Jane Doe"
            bind:value={displayName}
            error={errors.displayName}
            autocomplete="name"
            required
          />

          <Input
            id="email"
            label="Email address"
            type="email"
            placeholder="you@example.com"
            bind:value={email}
            error={errors.email}
            autocomplete="email"
            required
          />

          <div>
            <Input
              id="password"
              label="Password"
              type="password"
              placeholder="••••••••"
              bind:value={password}
              error={errors.password}
              autocomplete="new-password"
              required
            />
            {#if password.length > 0}
              <div class="strength-bar">
                <div class="strength-segments">
                  {#each [1, 2, 3, 4, 5] as level}
                    <div
                      class="segment"
                      style="background: {level <= passwordStrength()
                        ? strengthLabel().color
                        : 'var(--color-border)'}"
                    ></div>
                  {/each}
                </div>
                <span
                  class="strength-text"
                  style="color: {strengthLabel().color}"
                >
                  {strengthLabel().text}
                </span>
              </div>
            {/if}
          </div>

          <Input
            id="confirmPassword"
            label="Confirm password"
            type="password"
            placeholder="••••••••"
            bind:value={confirmPassword}
            error={errors.confirmPassword}
            autocomplete="new-password"
            required
          />

          <Button type="submit" variant="primary" full {loading}>
            {loading ? "Sending verification code…" : "Continue"}
          </Button>

          <p class="terms">
            By creating an account, you agree to our
            <a href="/terms">Terms of Service</a> and
            <a href="/privacy">Privacy Policy</a>.
          </p>
        </form>

        <p class="auth-footer">
          Already have an account? <a href="/login">Sign in →</a>
        </p>
      </div>
    {:else}
      <div
        in:fly={{ y: 20, duration: 300, easing: cubicOut }}
        out:fade={{ duration: 150 }}
      >
        <div class="otp-header">
          <div class="otp-badge">✉</div>
          <h1 class="auth-title">Verify your email</h1>
          <p class="auth-subtitle">
            We sent a 6-digit code to<br />
            <strong class="highlight-email">{email}</strong>
          </p>
        </div>

        <form class="auth-form" onsubmit={handleVerifyOtp} novalidate>
          {#if errors.otp}
            <div class="form-error" role="alert">⚠ {errors.otp}</div>
          {/if}

          <div class="otp-inputs-grid" onpaste={handleOtpPaste}>
            {#each [0, 1, 2, 3, 4, 5] as idx}
              <input
                bind:this={otpInputRefs[idx]}
                type="text"
                inputmode="numeric"
                pattern="[0-9]*"
                maxlength="1"
                class="otp-digit-input"
                class:has-val={otpDigits[idx].length > 0}
                class:input-error={errors.otp}
                value={otpDigits[idx]}
                oninput={(e) => handleOtpInput(idx, e)}
                onkeydown={(e) => handleOtpKeyDown(idx, e)}
                aria-label={`Digit ${idx + 1}`}
              />
            {/each}
          </div>

          <Button type="submit" variant="primary" full loading={verifyLoading}>
            {verifyLoading ? "Verifying…" : "Verify & Create Account"}
          </Button>

          <div class="otp-actions">
            {#if resendCooldown > 0}
              <span class="cooldown-text">
                Resend code in <span class="mono-timer"
                  >0:{resendCooldown < 10 ? "0" : ""}{resendCooldown}</span
                >
              </span>
            {:else}
              <button
                type="button"
                class="action-btn resend-link"
                onclick={handleResendOtp}
                disabled={resending}
              >
                {resending ? "Sending…" : "Resend verification code"}
              </button>
            {/if}

            <button
              type="button"
              class="action-btn back-link"
              onclick={handleBackToForm}
            >
              ← Change email or details
            </button>
          </div>
        </form>

        <p class="auth-footer">
          Already have an account? <a href="/login">Sign in →</a>
        </p>
      </div>
    {/if}
  </div>
</div>

<style>
  .shake {
    animation: shake 0.5s cubic-bezier(0.36, 0.07, 0.19, 0.97) both;
  }

  @keyframes shake {
    10%,
    90% {
      transform: translate3d(-1px, 0, 0);
    }
    20%,
    80% {
      transform: translate3d(2px, 0, 0);
    }
    30%,
    50%,
    70% {
      transform: translate3d(-4px, 0, 0);
    }
    40%,
    60% {
      transform: translate3d(4px, 0, 0);
    }
  }

  .form-error {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: var(--radius-md);
    color: #fca5a5;
    font-size: 0.9rem;
  }

  .strength-bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-2);
  }

  .strength-segments {
    display: flex;
    gap: 4px;
    flex: 1;
  }

  .segment {
    height: 4px;
    flex: 1;
    border-radius: 2px;
    transition: background 0.3s ease;
  }

  .strength-text {
    font-size: 0.8125rem;
    font-weight: 600;
    min-width: 3.5rem;
    text-align: right;
  }

  .terms {
    font-size: 0.8125rem;
    color: var(--color-text-subtle);
    text-align: center;
  }

  .terms a {
    color: var(--color-primary-light);
  }

  .otp-header {
    text-align: center;
    margin-bottom: var(--space-4);
  }

  .otp-badge {
    width: 52px;
    height: 52px;
    border-radius: 50%;
    background: rgba(139, 92, 246, 0.12);
    border: 1px solid rgba(139, 92, 246, 0.3);
    color: var(--color-primary-light);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.5rem;
    margin: 0 auto var(--space-4) auto;
    box-shadow: 0 0 24px rgba(139, 92, 246, 0.25);
    animation: pulseGlow 3s ease-in-out infinite;
  }

  @keyframes pulseGlow {
    0%,
    100% {
      transform: scale(1);
      box-shadow: 0 0 20px rgba(139, 92, 246, 0.2);
    }
    50% {
      transform: scale(1.05);
      box-shadow: 0 0 35px rgba(139, 92, 246, 0.4);
    }
  }

  .highlight-email {
    color: var(--color-text);
    font-weight: 600;
  }

  .otp-inputs-grid {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: var(--space-2);
    margin: var(--space-2) 0 var(--space-4) 0;
  }

  .otp-digit-input {
    width: 100%;
    height: 58px;
    text-align: center;
    font-family: var(--font-mono);
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-text);
    background: var(--color-bg-surface);
    border: 1.5px solid var(--color-border);
    border-radius: var(--radius-md);
    outline: none;
    transition: all var(--transition-fast);
  }

  .otp-digit-input:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-glow);
    background: var(--color-bg-elevated);
    transform: translateY(-2px);
  }

  .otp-digit-input.has-val {
    border-color: rgba(139, 92, 246, 0.45);
    background: rgba(139, 92, 246, 0.05);
  }

  .otp-digit-input.input-error {
    border-color: var(--color-error);
  }

  .otp-actions {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-2);
  }

  .action-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.875rem;
    transition: color var(--transition-fast);
    padding: 0;
  }

  .resend-link {
    color: var(--color-primary-light);
    font-weight: 500;
  }

  .resend-link:hover:not(:disabled) {
    color: var(--color-primary);
    text-decoration: underline;
  }

  .resend-link:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .cooldown-text {
    font-size: 0.875rem;
    color: var(--color-text-subtle);
  }

  .mono-timer {
    font-family: var(--font-mono);
    color: var(--color-text-muted);
    font-weight: 600;
  }

  .back-link {
    color: var(--color-text-subtle);
  }

  .back-link:hover {
    color: var(--color-text);
  }
</style>
