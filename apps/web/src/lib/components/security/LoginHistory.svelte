<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import RefreshButton from "$lib/components/RefreshButton.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import { securityApi } from "$lib/api/security";
  import type { LoginHistoryEntry } from "$lib/api/types";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";
  import { onMount } from "svelte";

  let history = $state<LoginHistoryEntry[]>([]);
  let loading = $state(false);

  onMount(async () => {
    await loadHistory();
  });

  export async function loadHistory() {
    loading = true;
    try {
      history = await securityApi.getLoginHistory(30);
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      }
    } finally {
      loading = false;
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

  function formatEvent(event: string): string {
    return event.replace(/_/g, " ").toLowerCase().replace(/\b\w/g, (c) => c.toUpperCase());
  }
</script>

<Card
  title="Login History & Audit Trail"
  subtitle="Cryptographically logged authentication attempts and session origin telemetry"
>
  {#snippet headerActions()}
    <RefreshButton
      onclick={loadHistory}
      {loading}
      label="Refresh authentication history"
    />
  {/snippet}

  {#if loading && history.length === 0}
    <div class="text-center py-8 text-sm text-brand-text-muted">Loading login history...</div>
  {:else if history.length === 0}
    <div class="text-center py-8 text-sm text-brand-text-muted">No historical login records found.</div>
  {:else}
    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs border-collapse">
        <thead>
          <tr class="border-b border-brand-border text-brand-text-muted font-medium">
            <th class="py-2.5 px-3">Event</th>
            <th class="py-2.5 px-3">Status</th>
            <th class="py-2.5 px-3">Device / Client</th>
            <th class="py-2.5 px-3">IP & Location</th>
            <th class="py-2.5 px-3">Time</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-white/5">
          {#each history as item (item.id)}
            <tr class="hover:bg-white/[0.02] transition-colors">
              <td class="py-3 px-3 font-semibold text-brand-text-primary whitespace-nowrap">
                <div class="flex items-center gap-2">
                  <span>{formatEvent(item.event)}</span>
                  {#if item.suspicious}
                    <Badge variant="danger" size="sm">Suspicious</Badge>
                  {/if}
                </div>
              </td>
              <td class="py-3 px-3 whitespace-nowrap">
                {#if item.status === "Success"}
                  <Badge variant="success" size="sm" dot>Success</Badge>
                {:else}
                  <Badge variant="danger" size="sm" dot>Failed</Badge>
                {/if}
              </td>
              <td class="py-3 px-3 text-brand-text-secondary whitespace-nowrap">
                {item.device_name || "Unknown Browser"}
              </td>
              <td class="py-3 px-3 text-brand-text-muted whitespace-nowrap">
                <div class="flex flex-col">
                  <span class="font-mono text-brand-text-secondary">{item.ip_address || "127.0.0.1"}</span>
                  <span class="text-[10px] text-brand-text-subtle">{item.location || "Local Network"}</span>
                </div>
              </td>
              <td class="py-3 px-3 text-brand-text-muted whitespace-nowrap font-mono text-[11px]">
                {formatTime(item.timestamp)}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</Card>

