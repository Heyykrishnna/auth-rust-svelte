<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import RefreshButton from "$lib/components/RefreshButton.svelte";
  import { securityApi } from "$lib/api/security";
  import type { ThreatIntelligenceReport } from "$lib/api/types";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";
  import { onMount } from "svelte";

  let report = $state<ThreatIntelligenceReport | null>(null);
  let loading = $state(false);

  onMount(async () => {
    await loadReport();
  });

  export async function loadReport() {
    loading = true;
    try {
      report = await securityApi.getThreatIntelligence();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      }
    } finally {
      loading = false;
    }
  }

  function getAtoBadgeVariant(level: string): "success" | "warning" | "danger" | "neutral" {
    switch (level) {
      case "low":
        return "success";
      case "guarded":
        return "neutral";
      case "elevated":
        return "warning";
      case "critical":
        return "danger";
      default:
        return "neutral";
    }
  }
</script>

<Card
  title="Real-Time Threat Intelligence & ATO Defense"
  subtitle="Automated anomaly detection, account takeover mitigation, and reputation heuristics"
>
  {#snippet headerActions()}
    <RefreshButton
      onclick={loadReport}
      {loading}
      label="Refresh threat telemetry"
    />
  {/snippet}

  {#if loading && !report}
    <div class="text-center py-8 text-sm text-brand-text-muted">Analyzing real-time security signals...</div>
  {:else if report}
    <div class="flex flex-col gap-6">
      <div class="p-5 rounded-xl border border-brand-border bg-brand-surface-card flex flex-col md:flex-row md:items-center justify-between gap-6">
        <div class="flex flex-col gap-2">
          <div class="flex items-center gap-2.5">
            <h3 class="text-base font-bold text-brand-text-primary m-0">Account Takeover (ATO) Threat Level</h3>
            <Badge variant={getAtoBadgeVariant(report.ato_risk_level)} size="sm">
              {report.ato_risk_level.toUpperCase()} RISK
            </Badge>
          </div>
          <p class="text-xs text-brand-text-secondary m-0 max-w-xl">
            Synthesized from impossible travel velocity, IP reputation classification, device fingerprint divergence, and failed authentication bursts.
          </p>
        </div>

        <div class="flex items-center gap-4">
          <div class="flex flex-col items-end">
            <span class="text-xs text-brand-text-muted">ATO Risk Index</span>
            <div class="flex items-baseline gap-1">
              <span class="text-3xl font-extrabold font-mono text-brand-lime">{report.ato_risk_score}</span>
              <span class="text-xs font-mono text-brand-text-muted">/100</span>
            </div>
          </div>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div class="p-4 rounded-xl border border-brand-border bg-brand-surface-card flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <div class="size-2 rounded-full bg-emerald-400"></div>
              <span class="text-sm font-semibold text-brand-text-primary">Brute Force Protection</span>
            </div>
            <Badge variant="success" size="sm">Enforced</Badge>
          </div>
          <p class="text-xs text-brand-text-secondary m-0">
            Consecutive failed attempts: <span class="font-mono text-brand-text-primary font-bold">{report.brute_force_status.consecutive_failures}</span> of {report.brute_force_status.max_attempts_threshold} maximum.
          </p>
          <div class="text-[11px] text-brand-text-muted">
            Lockout status: {report.brute_force_status.is_locked ? "Account Locked" : "Nominal (No Lockout)"}
          </div>
        </div>

        <div class="p-4 rounded-xl border border-brand-border bg-brand-surface-card flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <div class="size-2 rounded-full bg-emerald-400"></div>
              <span class="text-sm font-semibold text-brand-text-primary">Credential Stuffing Defense</span>
            </div>
            <Badge variant="success" size="sm">Active</Badge>
          </div>
          <p class="text-xs text-brand-text-secondary m-0">
            Monitoring distributed password-spraying and breached dictionary traffic across subnets.
          </p>
          <div class="text-[11px] text-brand-text-muted">
            Blocked velocity anomalies: {report.credential_stuffing_status.stuffing_events_blocked_7d} in past 7 days.
          </div>
        </div>

        <div class="p-4 rounded-xl border border-brand-border bg-brand-surface-card flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <div class="size-2 rounded-full bg-emerald-400"></div>
              <span class="text-sm font-semibold text-brand-text-primary">Impossible Travel Detection</span>
            </div>
            <Badge variant="purple" size="sm">&gt;800 km/h Alert</Badge>
          </div>
          <p class="text-xs text-brand-text-secondary m-0">
            Calculates geographic distance and velocity between consecutive logins via Haversine formula.
          </p>
          <div class="text-[11px] text-brand-text-muted">
            {report.impossible_travel_status.last_hop_description || "No geographic anomalies recorded."}
          </div>
        </div>

        <div class="p-4 rounded-xl border border-brand-border bg-brand-surface-card flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <div class="size-2 rounded-full bg-emerald-400"></div>
              <span class="text-sm font-semibold text-brand-text-primary">IP Reputation Analysis</span>
            </div>
            <Badge variant="neutral" size="sm">{report.ip_reputation.classification.replace("_", " ").toUpperCase()}</Badge>
          </div>
          <p class="text-xs text-brand-text-secondary m-0">
            Current IP: <span class="font-mono text-brand-text-primary">{report.ip_reputation.ip}</span> (Risk Score: {report.ip_reputation.score}/100)
          </p>
          <div class="text-[11px] text-brand-text-muted flex items-center gap-2">
            <span>Location: {report.ip_reputation.city || "Local"}, {report.ip_reputation.country || "Network"}</span>
            <span>•</span>
            <span>Proxy/Tor: {report.ip_reputation.is_vpn_proxy || report.ip_reputation.is_tor ? "Detected" : "Clean"}</span>
          </div>
        </div>
      </div>

      {#if report.recommendations.length > 0}
        <div class="p-4 rounded-xl border border-brand-border bg-brand-surface-raised flex flex-col gap-2">
          <span class="text-xs font-semibold text-brand-lime uppercase tracking-wider">Recommended Security Actions</span>
          <ul class="m-0 pl-4 text-xs text-brand-text-secondary flex flex-col gap-1.5">
            {#each report.recommendations as rec}
              <li>{rec}</li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  {/if}
</Card>

