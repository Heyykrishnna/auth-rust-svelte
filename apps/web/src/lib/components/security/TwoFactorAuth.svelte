<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Button from "$lib/components/Button.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import CopyButton from "$lib/components/CopyButton.svelte";
  import OtpInput from "$lib/components/OtpInput.svelte";
  import StepUpModal from "./StepUpModal.svelte";
  import { securityApi } from "$lib/api/security";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";

  let {
    mfaEnabled = false,
    onStatusChange,
  }: {
    mfaEnabled: boolean;
    onStatusChange?: (enabled: boolean) => void;
  } = $props();

  let enrolling = $state(false);
  let enrollLoading = $state(false);
  let enrollSecret = $state("");
  let enrollQrUri = $state("");
  let verifyCode = $state("");
  let verifying = $state(false);
  let enrollError = $state("");

  let stepUpOpen = $state(false);
  let disabling = $state(false);

  async function startEnrollment() {
    enrollError = "";
    enrollLoading = true;
    try {
      const res = await securityApi.enrollTotp();
      enrollSecret = res.secret;
      enrollQrUri = res.qr_uri;
      verifyCode = "";
      enrolling = true;
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to initialize two-factor enrollment", "error");
      }
    } finally {
      enrollLoading = false;
    }
  }

  async function handleConfirmEnrollment(e: SubmitEvent) {
    e.preventDefault();
    if (verifyCode.length !== 6) {
      enrollError = "Please enter all 6 digits of your authenticator code.";
      return;
    }

    enrollError = "";
    verifying = true;
    try {
      await securityApi.confirmTotp({ code: verifyCode });
      addToast("Two-Factor Authentication is now active", "success");
      enrolling = false;
      onStatusChange?.(true);
    } catch (err) {
      if (err instanceof ApiError) {
        enrollError = err.message;
      } else {
        enrollError = "Verification failed. Check the code and try again.";
      }
    } finally {
      verifying = false;
    }
  }

  function handleDisableClick() {
    stepUpOpen = true;
  }

  async function handleStepUpSuccess(token: string) {
    stepUpOpen = false;
    disabling = true;
    try {
      await securityApi.disableTotp(token);
      addToast("Two-Factor Authentication has been disabled", "success");
      onStatusChange?.(false);
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to disable Two-Factor Authentication", "error");
      }
    } finally {
      disabling = false;
    }
  }

  function formatSecret(secret: string): string {
    return secret.match(/.{1,4}/g)?.join(" ") || secret;
  }
</script>

<Card
  title="Two-Factor Authentication (2FA)"
  subtitle="Protect your account with RFC 6238 time-based one-time passwords (TOTP)"
>
  <div class="flex flex-col gap-6">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 rounded-xl border border-brand-border bg-brand-surface-card">
      <div class="flex items-start sm:items-center gap-3">
        <div class="size-10 rounded-lg bg-white/5 border border-white/10 flex items-center justify-center shrink-0 text-brand-lime">
          <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
          </svg>
        </div>
        <div class="flex flex-col gap-1">
          <div class="flex items-center gap-2">
            <span class="text-sm font-semibold text-brand-text-primary">Authenticator App</span>
            {#if mfaEnabled}
              <Badge variant="success" size="sm" dot>Enrolled & Active</Badge>
            {:else}
              <Badge variant="warning" size="sm">Not Configured</Badge>
            {/if}
          </div>
          <span class="text-xs text-brand-text-secondary">
            Compatible with Google Authenticator, 1Password, Bitwarden, and Apple Keychain.
          </span>
        </div>
      </div>

      <div class="flex items-center gap-3">
        {#if mfaEnabled}
          <Button
            size="sm"
            variant="danger"
            onclick={handleDisableClick}
            loading={disabling}
            disabled={disabling}
          >
            Disable 2FA
          </Button>
        {:else if !enrolling}
          <Button
            size="sm"
            onclick={startEnrollment}
            loading={enrollLoading}
            disabled={enrollLoading}
          >
            Setup Authenticator
          </Button>
        {/if}
      </div>
    </div>

    {#if enrolling}
      <div class="p-5 rounded-xl border border-brand-lime/30 bg-brand-surface-raised flex flex-col gap-5">
        <div class="flex items-center justify-between">
          <h4 class="text-sm font-semibold text-brand-text-primary m-0">Setup Authenticator Application</h4>
          <button
            type="button"
            onclick={() => (enrolling = false)}
            class="text-xs text-brand-text-muted hover:text-brand-text-primary bg-transparent border-0 cursor-pointer"
          >
            Cancel
          </button>
        </div>

        {#if enrollError}
          <div class="p-3 bg-rose-500/10 border border-rose-500/30 rounded-lg text-rose-300 text-xs" role="alert">
            {enrollError}
          </div>
        {/if}

        <div class="grid grid-cols-1 md:grid-cols-2 gap-6 items-center">
          <div class="flex flex-col gap-3">
            <span class="text-xs font-medium text-brand-text-secondary">
              1. Scan the QR code or enter this manual secret in your authenticator app:
            </span>
            <div class="flex items-center gap-2 p-3 bg-brand-surface-input rounded-lg border border-brand-border">
              <span class="font-mono text-sm tracking-wider text-brand-lime font-bold flex-1 select-all break-all">
                {formatSecret(enrollSecret)}
              </span>
              <CopyButton value={enrollSecret} label="Copy secret key" />
            </div>
            <a
              href={enrollQrUri}
              class="text-xs text-brand-lime hover:underline inline-flex items-center gap-1"
            >
              Open directly in authenticator app →
            </a>
          </div>

          <form onsubmit={handleConfirmEnrollment} class="flex flex-col gap-4">
            <span class="text-xs font-medium text-brand-text-secondary">
              2. Enter the 6-digit code shown in your app to activate:
            </span>
            <div class="flex justify-center py-2">
              <OtpInput
                bind:value={verifyCode}
                disabled={verifying}
                oncomplete={(c) => (verifyCode = c)}
              />
            </div>
            <div class="flex justify-end gap-2">
              <Button type="submit" size="sm" loading={verifying} disabled={verifyCode.length !== 6 || verifying}>
                Verify & Activate
              </Button>
            </div>
          </form>
        </div>
      </div>
    {/if}
  </div>
</Card>

<StepUpModal
  open={stepUpOpen}
  title="Step-Up: Disable Two-Factor"
  description="To disable two-factor authentication, verify your password or authenticator code."
  onSuccess={handleStepUpSuccess}
  onCancel={() => (stepUpOpen = false)}
/>

