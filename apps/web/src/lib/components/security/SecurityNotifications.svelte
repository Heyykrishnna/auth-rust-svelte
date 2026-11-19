<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Button from "$lib/components/Button.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import RefreshButton from "$lib/components/RefreshButton.svelte";
  import { securityApi } from "$lib/api/security";
  import type { SecurityAlertInfo, UserSecuritySettings } from "$lib/api/types";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";
  import { onMount } from "svelte";

  let {
    settings = null,
    onSettingsUpdated,
  }: {
    settings?: UserSecuritySettings | null;
    onSettingsUpdated?: () => void;
  } = $props();

  let alerts = $state<SecurityAlertInfo[]>([]);
  let loading = $state(false);
  let savingPrefs = $state(false);

  let notifyNewLogin = $state(true);
  let notifySuspicious = $state(true);
  let notifySecurityChanges = $state(true);

  $effect(() => {
    if (settings) {
      notifyNewLogin = settings.notify_new_login;
      notifySuspicious = settings.notify_suspicious_activity;
      notifySecurityChanges = settings.notify_security_changes;
    }
  });

  onMount(async () => {
    await loadAlerts();
  });

  export async function loadAlerts() {
    loading = true;
    try {
      alerts = await securityApi.listAlerts();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      }
    } finally {
      loading = false;
    }
  }

  async function handleSavePreferences() {
    savingPrefs = true;
    try {
      await securityApi.updateSettings({
        notify_new_login: notifyNewLogin,
        notify_suspicious_activity: notifySuspicious,
        notify_security_changes: notifySecurityChanges,
      });
      addToast("Notification preferences updated", "success");
      onSettingsUpdated?.();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to save preferences", "error");
      }
    } finally {
      savingPrefs = false;
    }
  }

  async function handleResolveAlert(alertId: string) {
    try {
      await securityApi.resolveAlert(alertId);
      alerts = alerts.map((a) => (a.id === alertId ? { ...a, resolved: true } : a));
      addToast("Alert marked as resolved", "success");
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to resolve alert", "error");
      }
    }
  }

  function getSeverityVariant(sev: string): "danger" | "warning" | "neutral" | "primary" {
    switch (sev) {
      case "critical":
        return "danger";
      case "high":
        return "danger";
      case "medium":
        return "warning";
      default:
        return "neutral";
    }
  }

  function formatTime(iso: string): string {
    try {
      return new Date(iso).toLocaleString(undefined, {
        dateStyle: "short",
        timeStyle: "short",
      });
    } catch {
      return iso;
    }
  }
</script>

<div class="flex flex-col gap-6">
  <Card
    title="Security Notifications"
    subtitle="Choose which security events trigger immediate email dispatches"
  >
    <div class="flex flex-col gap-5">
      <div class="flex flex-col divide-y divide-white/5">
        <label class="flex items-center justify-between py-3.5 cursor-pointer">
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-semibold text-brand-text-primary">New Device Sign-In</span>
            <span class="text-xs text-brand-text-secondary">
              Email alert whenever a session is created from an unrecognized device or browser.
            </span>
          </div>
          <input
            type="checkbox"
            bind:checked={notifyNewLogin}
            class="size-4 accent-brand-lime cursor-pointer rounded"
          />
        </label>

        <label class="flex items-center justify-between py-3.5 cursor-pointer">
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-semibold text-brand-text-primary">Suspicious Activity & ATO Flags</span>
            <span class="text-xs text-brand-text-secondary">
              Immediate alert for impossible travel anomalies, high-risk IPs, or brute-force locks.
            </span>
          </div>
          <input
            type="checkbox"
            bind:checked={notifySuspicious}
            class="size-4 accent-brand-lime cursor-pointer rounded"
          />
        </label>

        <label class="flex items-center justify-between py-3.5 cursor-pointer">
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-semibold text-brand-text-primary">Security Credential Changes</span>
            <span class="text-xs text-brand-text-secondary">
              Notifications when passwords, two-factor credentials, or recovery keys are modified.
            </span>
          </div>
          <input
            type="checkbox"
            bind:checked={notifySecurityChanges}
            class="size-4 accent-brand-lime cursor-pointer rounded"
          />
        </label>
      </div>

      <div class="flex justify-end pt-2">
        <Button size="sm" onclick={handleSavePreferences} loading={savingPrefs}>
          Save Preferences
        </Button>
      </div>
    </div>
  </Card>

  <Card
    title="Suspicious Login Alerts & Incidents"
    subtitle="Security notifications generated by automated behavioral analysis and threat rules"
  >
    {#snippet headerActions()}
      <RefreshButton
        onclick={loadAlerts}
        {loading}
        label="Refresh incident log"
      />
    {/snippet}

    {#if loading && alerts.length === 0}
      <div class="text-center py-8 text-sm text-brand-text-muted">Loading incident alerts...</div>
    {:else if alerts.length === 0}
      <div class="text-center py-8 text-sm text-brand-text-muted">No security alerts or incidents reported.</div>
    {:else}
      <div class="flex flex-col gap-3">
        {#each alerts as alert (alert.id)}
          <div class="flex flex-col sm:flex-row sm:items-center justify-between p-4 rounded-xl border {alert.resolved ? 'border-brand-border bg-brand-surface-card opacity-60' : 'border-rose-500/30 bg-rose-950/10'} gap-4">
            <div class="flex items-start sm:items-center gap-3.5 min-w-0">
              <div class="size-10 rounded-lg {alert.resolved ? 'bg-white/5 text-brand-text-muted' : 'bg-rose-500/10 text-rose-400 border border-rose-500/20'} flex items-center justify-center shrink-0">
                <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
                  <line x1="12" y1="9" x2="12" y2="13"></line>
                  <line x1="12" y1="17" x2="12.01" y2="17"></line>
                </svg>
              </div>

              <div class="flex flex-col gap-1 min-w-0">
                <div class="flex items-center gap-2 flex-wrap">
                  <span class="text-sm font-semibold text-brand-text-primary">{alert.title}</span>
                  <Badge variant={getSeverityVariant(alert.severity)} size="sm">
                    {alert.severity.toUpperCase()}
                  </Badge>
                  {#if alert.resolved}
                    <Badge variant="neutral" size="sm">Resolved</Badge>
                  {/if}
                </div>
                <p class="text-xs text-brand-text-secondary m-0">{alert.description}</p>
                <div class="flex items-center gap-2 text-xs text-brand-text-muted flex-wrap">
                  {#if alert.ip_address}
                    <span>IP: {alert.ip_address}</span>
                    <span>•</span>
                  {/if}
                  {#if alert.location}
                    <span>{alert.location}</span>
                    <span>•</span>
                  {/if}
                  <span>{formatTime(alert.created_at)}</span>
                </div>
              </div>
            </div>

            {#if !alert.resolved}
              <div class="self-end sm:self-center">
                <button
                  type="button"
                  onclick={() => handleResolveAlert(alert.id)}
                  class="px-2.5 py-1 text-xs font-semibold rounded-lg bg-white/5 hover:bg-white/10 text-brand-text-secondary hover:text-brand-text-primary border border-white/10 cursor-pointer transition-colors"
                >
                  Mark Resolved
                </button>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </Card>
</div>
