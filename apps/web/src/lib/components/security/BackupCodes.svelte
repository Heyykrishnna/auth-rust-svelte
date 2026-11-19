<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Button from "$lib/components/Button.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import StepUpModal from "./StepUpModal.svelte";
  import { securityApi } from "$lib/api/security";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";

  let {
    remainingCount = 0,
    onUpdated,
  }: {
    remainingCount: number;
    onUpdated?: () => void;
  } = $props();

  let stepUpOpen = $state(false);
  let generating = $state(false);
  let revealedCodes = $state<string[]>([]);
  let showCodes = $state(false);

  function handleGenerateClick() {
    stepUpOpen = true;
  }

  async function handleStepUpSuccess(token: string) {
    stepUpOpen = false;
    generating = true;
    try {
      const res = await securityApi.generateBackupCodes(token);
      revealedCodes = res.codes;
      showCodes = true;
      addToast("New backup recovery codes generated", "success");
      onUpdated?.();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to generate backup codes", "error");
      }
    } finally {
      generating = false;
    }
  }

  async function copyAllCodes() {
    if (revealedCodes.length === 0) return;
    const text = `Signet Emergency Backup Codes:\n\n${revealedCodes.join("\n")}\n\nKeep these codes in a safe, offline location. Each code can only be used once.`;
    try {
      await navigator.clipboard.writeText(text);
      addToast("Backup codes copied to clipboard", "success");
    } catch {
      addToast("Failed to copy codes", "error");
    }
  }

  function downloadCodes() {
    if (revealedCodes.length === 0) return;
    const text = `Signet Emergency Backup Codes\nGenerated: ${new Date().toISOString()}\n\n${revealedCodes.join("\n")}\n\nEach code can be used once as a two-factor fallback.`;
    const blob = new Blob([text], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "signet-backup-codes.txt";
    a.click();
    URL.revokeObjectURL(url);
    addToast("Downloaded signet-backup-codes.txt", "success");
  }
</script>

<Card
  title="Backup Recovery Codes"
  subtitle="Single-use emergency codes to access your account if your authenticator device is lost"
>
  <div class="flex flex-col gap-6">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 rounded-xl border border-brand-border bg-brand-surface-card">
      <div class="flex items-start sm:items-center gap-3">
        <div class="size-10 rounded-lg bg-white/5 border border-white/10 flex items-center justify-center shrink-0 text-brand-lime">
          <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="4" width="18" height="16" rx="2"></rect>
            <line x1="7" y1="8" x2="17" y2="8"></line>
            <line x1="7" y1="12" x2="17" y2="12"></line>
            <line x1="7" y1="16" x2="13" y2="16"></line>
          </svg>
        </div>
        <div class="flex flex-col gap-1">
          <div class="flex items-center gap-2">
            <span class="text-sm font-semibold text-brand-text-primary">Emergency Codes</span>
            {#if remainingCount > 3}
              <Badge variant="success" size="sm">{remainingCount} Active</Badge>
            {:else if remainingCount > 0}
              <Badge variant="warning" size="sm">{remainingCount} Left</Badge>
            {:else}
              <Badge variant="danger" size="sm">Exhausted</Badge>
            {/if}
          </div>
          <span class="text-xs text-brand-text-secondary">
            Store these codes in a password manager or printed safe. Each code can be used once.
          </span>
        </div>
      </div>

      <div>
        <Button
          size="sm"
          onclick={handleGenerateClick}
          loading={generating}
          disabled={generating}
        >
          {remainingCount > 0 ? "Regenerate Codes" : "Generate Codes"}
        </Button>
      </div>
    </div>

    {#if showCodes && revealedCodes.length > 0}
      <div class="p-5 rounded-xl border border-brand-lime/30 bg-brand-surface-raised flex flex-col gap-4">
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <h4 class="text-sm font-semibold text-brand-text-primary m-0">Your Single-Use Backup Codes</h4>
            <span class="text-xs text-amber-300/90">
              Save these now. Once you leave this page, you cannot view them again without re-authenticating.
            </span>
          </div>
          <div class="flex items-center gap-2">
            <button
              type="button"
              onclick={copyAllCodes}
              class="px-2.5 py-1 text-xs font-semibold rounded-lg bg-white/5 hover:bg-white/10 text-brand-text-secondary hover:text-brand-text-primary border border-white/10 cursor-pointer transition-colors"
            >
              Copy All
            </button>
            <button
              type="button"
              onclick={downloadCodes}
              class="px-2.5 py-1 text-xs font-semibold rounded-lg bg-white/5 hover:bg-white/10 text-brand-text-secondary hover:text-brand-text-primary border border-white/10 cursor-pointer transition-colors"
            >
              Download .txt
            </button>
            <button
              type="button"
              onclick={() => (showCodes = false)}
              class="px-2.5 py-1 text-xs font-semibold rounded-lg bg-white/5 hover:bg-white/10 text-brand-text-secondary hover:text-brand-text-primary border border-white/10 cursor-pointer transition-colors"
            >
              Hide
            </button>
          </div>
        </div>

        <div class="grid grid-cols-2 sm:grid-cols-5 gap-2.5 p-4 bg-brand-surface-input rounded-lg border border-brand-border">
          {#each revealedCodes as code}
            <div class="font-mono text-xs font-bold text-brand-lime bg-white/5 py-2 px-3 rounded text-center tracking-wider border border-white/5 select-all">
              {code}
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</Card>

<StepUpModal
  open={stepUpOpen}
  title="Step-Up: Access Backup Codes"
  description="Generating or viewing backup recovery codes requires confirming your identity with your password or authenticator code."
  onSuccess={handleStepUpSuccess}
  onCancel={() => (stepUpOpen = false)}
/>
