<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Input from "$lib/components/Input.svelte";
  import OtpInput from "$lib/components/OtpInput.svelte";
  import { securityApi } from "$lib/api/security";
  import { ApiError } from "$lib/api/client";

  let {
    open = false,
    title = "Step-Up Verification",
    description = "Confirm your identity with your password or authenticator code to proceed with this sensitive action.",
    onSuccess,
    onCancel,
  }: {
    open: boolean;
    title?: string;
    description?: string;
    onSuccess: (token: string) => void;
    onCancel: () => void;
  } = $props();

  let mode = $state<"password" | "totp">("password");
  let password = $state("");
  let totpCode = $state("");
  let loading = $state(false);
  let error = $state("");
  let dialog = $state<HTMLDialogElement>();

  $effect(() => {
    if (open && dialog && !dialog.open) {
      password = "";
      totpCode = "";
      error = "";
      dialog.showModal();
    }
    if (!open && dialog?.open) {
      dialog.close();
    }
  });

  async function handleVerify(e: SubmitEvent) {
    e.preventDefault();
    error = "";
    loading = true;

    try {
      const res = await securityApi.stepUp({
        password: mode === "password" ? password : undefined,
        totp_code: mode === "totp" ? totpCode : undefined,
      });
      onSuccess(res.step_up_token);
    } catch (err) {
      if (err instanceof ApiError) {
        error = err.message;
      } else {
        error = "Verification failed. Please check your credentials.";
      }
    } finally {
      loading = false;
    }
  }

  function handleBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget && !loading) {
      onCancel();
    }
  }
</script>

<dialog
  bind:this={dialog}
  onclick={handleBackdrop}
  class="bg-transparent p-0 m-auto backdrop:bg-black/75 backdrop:backdrop-blur-sm max-w-md w-full border-0 outline-none"
>
  <div
    class="bg-brand-surface-card border border-brand-border rounded-xl shadow-2xl p-6 flex flex-col gap-5 text-brand-text-primary"
  >
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2.5">
        <div class="size-8 rounded-lg bg-brand-lime/10 border border-brand-lime/30 flex items-center justify-center text-brand-lime">
          <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
          </svg>
        </div>
        <h3 class="font-heading font-bold text-lg text-brand-text-primary m-0">{title}</h3>
      </div>
      <button
        type="button"
        onclick={onCancel}
        disabled={loading}
        class="text-brand-text-secondary hover:text-brand-text-primary bg-transparent border-0 p-1 cursor-pointer"
        aria-label="Close"
      >
        <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    </div>

    <p class="text-xs text-brand-text-secondary m-0 leading-relaxed">
      {description}
    </p>

    <div class="flex gap-2 p-1 bg-brand-surface-raised rounded-lg border border-brand-border">
      <button
        type="button"
        onclick={() => (mode = "password")}
        class="flex-1 py-1.5 text-xs font-semibold rounded-md border-0 cursor-pointer transition-colors {mode === 'password' ? 'bg-brand-surface-card text-brand-lime shadow' : 'bg-transparent text-brand-text-muted hover:text-brand-text-secondary'}"
      >
        Account Password
      </button>
      <button
        type="button"
        onclick={() => (mode = "totp")}
        class="flex-1 py-1.5 text-xs font-semibold rounded-md border-0 cursor-pointer transition-colors {mode === 'totp' ? 'bg-brand-surface-card text-brand-lime shadow' : 'bg-transparent text-brand-text-muted hover:text-brand-text-secondary'}"
      >
        Authenticator Code
      </button>
    </div>

    {#if error}
      <div class="p-3 bg-rose-500/10 border border-rose-500/30 rounded-lg text-rose-300 text-xs" role="alert">
        {error}
      </div>
    {/if}

    <form onsubmit={handleVerify} class="flex flex-col gap-4">
      {#if mode === "password"}
        <Input
          id="stepup-pwd"
          type="password"
          label="Current Password"
          placeholder="••••••••••••"
          bind:value={password}
          required
        />
      {:else}
        <div class="flex flex-col gap-2">
          <label class="text-xs font-medium text-brand-text-secondary" for="stepup-totp">
            6-Digit Authenticator Code
          </label>
          <div class="flex justify-center py-2">
            <OtpInput
              bind:value={totpCode}
              disabled={loading}
              oncomplete={(c) => (totpCode = c)}
            />
          </div>
        </div>
      {/if}

      <div class="flex items-center justify-end gap-3 pt-2">
        <button
          type="button"
          onclick={onCancel}
          disabled={loading}
          class="px-3 py-1.5 text-xs font-medium rounded-lg text-brand-text-secondary hover:text-brand-text-primary bg-transparent border-0 cursor-pointer"
        >
          Cancel
        </button>
        <Button type="submit" size="sm" {loading} disabled={loading}>
          Verify & Continue
        </Button>
      </div>
    </form>
  </div>
</dialog>
