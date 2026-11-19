<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import RefreshButton from "$lib/components/RefreshButton.svelte";
  import DeleteButton from "$lib/components/DeleteButton.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { securityApi } from "$lib/api/security";
  import type { DeviceInfo } from "$lib/api/types";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";
  import { onMount } from "svelte";

  let devices = $state<DeviceInfo[]>([]);
  let loading = $state(false);
  let deviceToDelete = $state<DeviceInfo | null>(null);
  let deleting = $state(false);

  onMount(async () => {
    await loadDevices();
  });

  export async function loadDevices() {
    loading = true;
    try {
      devices = await securityApi.listDevices();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      }
    } finally {
      loading = false;
    }
  }

  async function handleToggleTrust(device: DeviceInfo) {
    try {
      if (device.trusted) {
        await securityApi.untrustDevice(device.id);
        addToast("Device trust revoked", "success");
      } else {
        await securityApi.trustDevice(device.id);
        addToast("Device marked as trusted for 30 days", "success");
      }
      await loadDevices();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to update device trust status", "error");
      }
    }
  }

  async function executeRemoveDevice() {
    if (!deviceToDelete) return;
    deleting = true;
    try {
      await securityApi.removeDevice(deviceToDelete.id);
      addToast("Device removed from account", "success");
      deviceToDelete = null;
      await loadDevices();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to remove device", "error");
      }
    } finally {
      deleting = false;
    }
  }

  function formatTime(iso: string): string {
    try {
      return new Date(iso).toLocaleDateString(undefined, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return iso;
    }
  }
</script>

<Card
  title="Device Management & Trusted Devices"
  subtitle="Manage recognized hardware profiles and bypass step-up challenges on trusted devices"
>
  {#snippet headerActions()}
    <RefreshButton
      onclick={loadDevices}
      {loading}
      label="Refresh recognized devices"
    />
  {/snippet}

  {#if loading && devices.length === 0}
    <div class="text-center py-8 text-sm text-brand-text-muted">Loading recognized devices...</div>
  {:else if devices.length === 0}
    <div class="text-center py-8 text-sm text-brand-text-muted">No devices recorded yet.</div>
  {:else}
    <div class="flex flex-col gap-3">
      {#each devices as dev (dev.id)}
        <div class="flex flex-col sm:flex-row sm:items-center justify-between p-4 rounded-xl border border-brand-border bg-brand-surface-card gap-4">
          <div class="flex items-start sm:items-center gap-3.5 min-w-0">
            <div class="size-10 rounded-lg bg-brand-lime/10 border border-brand-lime/20 flex items-center justify-center shrink-0 text-brand-lime">
              {#if dev.device_type === "mobile"}
                <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="5" y="2" width="14" height="20" rx="2" ry="2"></rect>
                  <line x1="12" y1="18" x2="12.01" y2="18"></line>
                </svg>
              {:else}
                <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
                  <line x1="8" y1="21" x2="16" y2="21"></line>
                  <line x1="12" y1="17" x2="12" y2="21"></line>
                </svg>
              {/if}
            </div>

            <div class="flex flex-col gap-1 min-w-0">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="text-sm font-semibold text-brand-text-primary">{dev.device_name}</span>
                {#if dev.is_current}
                  <Badge variant="purple" size="sm">This Device</Badge>
                {/if}
                {#if dev.trusted}
                  <Badge variant="success" size="sm" dot>Trusted Device</Badge>
                {:else}
                  <Badge variant="neutral" size="sm">Standard Trust</Badge>
                {/if}
              </div>
              <div class="flex items-center gap-2 text-xs text-brand-text-muted flex-wrap">
                <span>IP: {dev.last_ip || "127.0.0.1"}</span>
                <span>•</span>
                <span>Location: {dev.last_location || "Localhost"}</span>
                <span>•</span>
                <span>Active: {formatTime(dev.last_active_at)}</span>
              </div>
            </div>
          </div>

          <div class="flex items-center gap-2 self-end sm:self-center">
            <button
              type="button"
              onclick={() => handleToggleTrust(dev)}
              class="px-3 py-1.5 text-xs font-semibold rounded-lg {dev.trusted ? 'bg-white/5 hover:bg-white/10 text-brand-text-secondary hover:text-brand-text-primary border border-white/10' : 'bg-brand-lime hover:bg-brand-lime-hover text-brand-lime-ink border border-brand-lime'} cursor-pointer transition-colors"
            >
              {dev.trusted ? "Revoke Trust" : "Trust Device"}
            </button>
            {#if !dev.is_current}
              <DeleteButton
                label="Remove device"
                onclick={() => (deviceToDelete = dev)}
              />
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</Card>

<Modal
  open={deviceToDelete !== null}
  title="Remove Recognized Device?"
  description="Removing this device will revoke its trusted status and any associated active sessions."
  confirmText="Remove Device"
  variant="danger"
  loading={deleting}
  onconfirm={executeRemoveDevice}
  oncancel={() => (deviceToDelete = null)}
/>
