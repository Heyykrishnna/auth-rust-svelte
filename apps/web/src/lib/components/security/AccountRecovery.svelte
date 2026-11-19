<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Button from "$lib/components/Button.svelte";
  import Input from "$lib/components/Input.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import RefreshButton from "$lib/components/RefreshButton.svelte";
  import { securityApi } from "$lib/api/security";
  import type { AccountRecoveryStatus, UserSecuritySettings } from "$lib/api/types";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";
  import { onMount } from "svelte";

  let {
    settings = null,
    onUpdated,
  }: {
    settings?: UserSecuritySettings | null;
    onUpdated?: () => void;
  } = $props();

  let recoveryStatus = $state<AccountRecoveryStatus | null>(null);
  let loading = $state(false);
  let saving = $state(false);

  let recoveryEmail = $state("");
  let recoveryPhone = $state("");

  $effect(() => {
    if (settings) {
      if (recoveryEmail === "" && settings.recovery_email) {
        recoveryEmail = settings.recovery_email;
      }
      if (recoveryPhone === "" && settings.recovery_phone) {
        recoveryPhone = settings.recovery_phone;
      }
    }
  });

  onMount(async () => {
    await loadRecoveryStatus();
  });

  export async function loadRecoveryStatus() {
    loading = true;
    try {
      recoveryStatus = await securityApi.getRecoveryStatus();
      if (recoveryStatus.recovery_email) recoveryEmail = recoveryStatus.recovery_email;
      if (recoveryStatus.recovery_phone) recoveryPhone = recoveryStatus.recovery_phone;
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      }
    } finally {
      loading = false;
    }
  }

  async function handleSaveRecovery(e: SubmitEvent) {
    e.preventDefault();
    saving = true;
    try {
      await securityApi.updateSettings({
        recovery_email: recoveryEmail.trim() || null,
        recovery_phone: recoveryPhone.trim() || null,
      });
      addToast("Account recovery settings updated", "success");
      await loadRecoveryStatus();
      onUpdated?.();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to save recovery configuration", "error");
      }
    } finally {
      saving = false;
    }
  }
</script>

<Card
  title="Account Recovery & Failover Channels"
  subtitle="Configure emergency recovery contacts and inspect account restoration health"
>
  {#snippet headerActions()}
    <RefreshButton
      onclick={loadRecoveryStatus}
      {loading}
      label="Refresh recovery status"
    />
  {/snippet}

  {#if loading && !recoveryStatus}
    <div class="text-center py-8 text-sm text-brand-text-muted">Evaluating recovery readiness...</div>
  {:else if recoveryStatus}
    <div class="flex flex-col gap-6">
      <div class="p-5 rounded-xl border border-brand-border bg-brand-surface-card flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div class="flex flex-col gap-1.5">
          <div class="flex items-center gap-2">
            <span class="text-sm font-semibold text-brand-text-primary">Recovery Readiness Index</span>
            {#if recoveryStatus.recovery_readiness_score >= 80}
              <Badge variant="success" size="sm">High Resilience</Badge>
            {:else if recoveryStatus.recovery_readiness_score >= 50}
              <Badge variant="warning" size="sm">Moderate</Badge>
            {:else}
              <Badge variant="danger" size="sm">At Risk</Badge>
            {/if}
          </div>
          <span class="text-xs text-brand-text-secondary">
            Multi-channel recovery mechanisms prevent permanent lockout if you lose primary credentials.
          </span>
        </div>

        <div class="flex items-baseline gap-1 self-end sm:self-center">
          <span class="text-3xl font-extrabold font-mono text-brand-lime">
            {recoveryStatus.recovery_readiness_score}%
          </span>
        </div>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <div class="p-3.5 rounded-lg border border-brand-border bg-brand-surface-raised flex items-center justify-between text-xs">
          <span class="text-brand-text-secondary">Primary Email</span>
          <Badge variant="success" size="sm">Verified</Badge>
        </div>
        <div class="p-3.5 rounded-lg border border-brand-border bg-brand-surface-raised flex items-center justify-between text-xs">
          <span class="text-brand-text-secondary">Secondary Recovery Email</span>
          {#if recoveryStatus.recovery_email}
            <Badge variant="success" size="sm">Configured</Badge>
          {:else}
            <Badge variant="warning" size="sm">Missing</Badge>
          {/if}
        </div>
        <div class="p-3.5 rounded-lg border border-brand-border bg-brand-surface-raised flex items-center justify-between text-xs">
          <span class="text-brand-text-secondary">Backup Recovery Codes</span>
          {#if recoveryStatus.backup_codes_remaining > 0}
            <Badge variant="success" size="sm">{recoveryStatus.backup_codes_remaining} Stored</Badge>
          {:else}
            <Badge variant="danger" size="sm">0 Remaining</Badge>
          {/if}
        </div>
        <div class="p-3.5 rounded-lg border border-brand-border bg-brand-surface-raised flex items-center justify-between text-xs">
          <span class="text-brand-text-secondary">Passkeys Registered</span>
          {#if recoveryStatus.passkeys_count > 0}
            <Badge variant="purple" size="sm">{recoveryStatus.passkeys_count} Active</Badge>
          {:else}
            <Badge variant="neutral" size="sm">None</Badge>
          {/if}
        </div>
      </div>

      <form onsubmit={handleSaveRecovery} class="flex flex-col gap-4 pt-2">
        <Input
          id="recoveryEmail"
          type="email"
          label="Secondary Recovery Email"
          placeholder="recovery@example.com"
          bind:value={recoveryEmail}
          hint="Alternative email address to receive password reset links if primary email is compromised."
        />

        <Input
          id="recoveryPhone"
          type="text"
          label="Emergency Contact Phone"
          placeholder="+1 (555) 000-0000"
          bind:value={recoveryPhone}
          hint="Phone number for critical emergency security notifications and recovery verification."
        />

        <div class="flex justify-end pt-2">
          <Button type="submit" size="sm" loading={saving} disabled={saving}>
            Save Recovery Options
          </Button>
        </div>
      </form>
    </div>
  {/if}
</Card>
