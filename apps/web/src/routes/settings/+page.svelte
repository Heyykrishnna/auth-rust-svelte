<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Button from "$lib/components/Button.svelte";
  import Input from "$lib/components/Input.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import TwoFactorAuth from "$lib/components/security/TwoFactorAuth.svelte";
  import PasskeyManager from "$lib/components/security/PasskeyManager.svelte";
  import BackupCodes from "$lib/components/security/BackupCodes.svelte";
  import ActiveSessions from "$lib/components/security/ActiveSessions.svelte";
  import DeviceManagement from "$lib/components/security/DeviceManagement.svelte";
  import ThreatIntelligence from "$lib/components/security/ThreatIntelligence.svelte";
  import SecurityNotifications from "$lib/components/security/SecurityNotifications.svelte";
  import AccountRecovery from "$lib/components/security/AccountRecovery.svelte";
  import LoginHistory from "$lib/components/security/LoginHistory.svelte";
  import { authStore, setAuth } from "$lib/stores/auth.svelte";
  import { addToast } from "$lib/components/Toast.svelte";
  import { usersApi } from "$lib/api/users";
  import { authApi } from "$lib/api/auth";
  import { securityApi } from "$lib/api/security";
  import type { SecuritySummaryResponse } from "$lib/api/types";
  import { ApiError } from "$lib/api/client";
  import { onMount } from "svelte";

  let displayName = $state("");
  let profileLoading = $state(false);
  let profileError = $state("");

  let sendingReset = $state(false);

  type SecurityTab = "overview" | "credentials" | "sessions" | "recovery" | "history";
  let activeTab = $state<SecurityTab>("overview");

  let summary = $state<SecuritySummaryResponse | null>(null);
  let summaryLoading = $state(false);

  onMount(async () => {
    if (authStore.user) {
      displayName = authStore.user.display_name;
    }
    await loadSecuritySummary();
  });

  $effect(() => {
    if (authStore.user && !displayName) {
      displayName = authStore.user.display_name;
    }
  });

  async function loadSecuritySummary() {
    summaryLoading = true;
    try {
      summary = await securityApi.getSummary();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      }
    } finally {
      summaryLoading = false;
    }
  }

  async function handleUpdateProfile(e: SubmitEvent) {
    e.preventDefault();
    if (!displayName.trim() || displayName.trim().length < 2) {
      profileError = "Display name must be at least 2 characters";
      return;
    }

    profileError = "";
    profileLoading = true;
    try {
      const updated = await usersApi.updateProfile({
        display_name: displayName.trim(),
      });
      setAuth(updated);
      addToast("Profile updated successfully", "success");
    } catch (err) {
      if (err instanceof ApiError) {
        profileError = err.message;
      } else {
        profileError = "Failed to update profile";
      }
    } finally {
      profileLoading = false;
    }
  }

  async function handleSendPasswordReset() {
    if (!authStore.user?.email) return;
    sendingReset = true;
    try {
      await authApi.forgotPassword({ email: authStore.user.email });
      addToast("Password reset link sent to your email", "success");
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to request password reset", "error");
      }
    } finally {
      sendingReset = false;
    }
  }
</script>

<svelte:head>
  <title>Account Security & Settings — Signet</title>
  <meta
    name="description"
    content="Manage your account profile, passkeys, two-factor authentication, trusted devices, active sessions, and threat telemetry."
  />
</svelte:head>

<div class="max-w-5xl mx-auto px-4 sm:px-6 py-8 pb-20">
  <div class="mb-8 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h1 class="text-3xl font-bold font-heading text-brand-text-primary tracking-tight mb-1">
        Account Settings & Security
      </h1>
      <p class="text-sm text-brand-text-secondary m-0">
        Manage your profile identity, cryptographic credentials, connected sessions, and automated threat defenses.
      </p>
    </div>

    {#if summaryLoading}
      <div class="flex items-center gap-2 p-2 px-3 rounded-lg border border-brand-border bg-brand-surface-card shrink-0 text-xs text-brand-text-muted">
        Syncing security state...
      </div>
    {:else if summary}
      <div class="flex items-center gap-2 p-2 px-3 rounded-lg border border-brand-border bg-brand-surface-card shrink-0">
        <span class="text-xs text-brand-text-secondary">Security Health:</span>
        <Badge
          variant={summary.threat_report.ato_risk_level === "low" ? "success" : summary.threat_report.ato_risk_level === "guarded" ? "neutral" : "danger"}
          size="sm"
          dot
        >
          {summary.threat_report.ato_risk_level.toUpperCase()} RISK
        </Badge>
      </div>
    {/if}
  </div>

  <div class="flex flex-col gap-8">
    <Card
      title="Profile Information"
      subtitle="Public display credentials and account identification"
    >
      <form onsubmit={handleUpdateProfile} class="flex flex-col gap-5">
        {#if profileError}
          <div class="p-3.5 bg-rose-500/10 border border-rose-500/30 rounded-lg text-rose-300 text-sm" role="alert">
            {profileError}
          </div>
        {/if}

        <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
          <div class="flex flex-col gap-1.5">
            <label class="text-xs font-medium text-brand-text-muted" for="email">Account Email</label>
            <input
              id="email"
              type="email"
              value={authStore.user?.email || ""}
              disabled
              class="w-full px-3.5 py-2.5 bg-brand-surface-card border border-brand-border rounded-md text-brand-text-secondary font-sans text-sm outline-none opacity-60 cursor-not-allowed"
            />
            <span class="text-xs text-brand-text-subtle">
              Primary email address bound to your Signet identity.
            </span>
          </div>

          <Input
            id="displayName"
            label="Display Name"
            bind:value={displayName}
            required
            placeholder="Jane Doe"
            hint="This name appears across sessions, devices, and team spaces."
          />
        </div>

        <div class="flex items-center justify-between pt-2">
          <div class="flex items-center gap-2">
            {#if authStore.user?.email_verified}
              <Badge variant="success" size="sm" dot>Email Verified</Badge>
            {:else}
              <Badge variant="warning" size="sm">Verification Pending</Badge>
            {/if}
            <Badge variant="neutral" size="sm">HttpOnly Cookies</Badge>
          </div>
          <Button type="submit" size="sm" loading={profileLoading}>
            Save Profile
          </Button>
        </div>
      </form>
    </Card>

    <div class="flex items-center gap-1.5 p-1 bg-brand-surface-raised rounded-xl border border-brand-border overflow-x-auto">
      <button
        type="button"
        onclick={() => (activeTab = "overview")}
        class="py-2 px-3.5 text-xs font-semibold rounded-lg border-0 cursor-pointer transition-colors whitespace-nowrap {activeTab === 'overview' ? 'bg-brand-surface-card text-brand-lime shadow-sm' : 'bg-transparent text-brand-text-muted hover:text-brand-text-secondary'}"
      >
        Threat Intel & Alerts
      </button>
      <button
        type="button"
        onclick={() => (activeTab = "credentials")}
        class="py-2 px-3.5 text-xs font-semibold rounded-lg border-0 cursor-pointer transition-colors whitespace-nowrap {activeTab === 'credentials' ? 'bg-brand-surface-card text-brand-lime shadow-sm' : 'bg-transparent text-brand-text-muted hover:text-brand-text-secondary'}"
      >
        2FA, Passkeys & Backup
      </button>
      <button
        type="button"
        onclick={() => (activeTab = "sessions")}
        class="py-2 px-3.5 text-xs font-semibold rounded-lg border-0 cursor-pointer transition-colors whitespace-nowrap {activeTab === 'sessions' ? 'bg-brand-surface-card text-brand-lime shadow-sm' : 'bg-transparent text-brand-text-muted hover:text-brand-text-secondary'}"
      >
        Sessions & Devices
      </button>
      <button
        type="button"
        onclick={() => (activeTab = "recovery")}
        class="py-2 px-3.5 text-xs font-semibold rounded-lg border-0 cursor-pointer transition-colors whitespace-nowrap {activeTab === 'recovery' ? 'bg-brand-surface-card text-brand-lime shadow-sm' : 'bg-transparent text-brand-text-muted hover:text-brand-text-secondary'}"
      >
        Account Recovery
      </button>
      <button
        type="button"
        onclick={() => (activeTab = "history")}
        class="py-2 px-3.5 text-xs font-semibold rounded-lg border-0 cursor-pointer transition-colors whitespace-nowrap {activeTab === 'history' ? 'bg-brand-surface-card text-brand-lime shadow-sm' : 'bg-transparent text-brand-text-muted hover:text-brand-text-secondary'}"
      >
        Login History
      </button>
    </div>

    {#if activeTab === "overview"}
      <div class="flex flex-col gap-8">
        <ThreatIntelligence />
        <SecurityNotifications
          settings={summary?.settings}
          onSettingsUpdated={loadSecuritySummary}
        />
      </div>
    {:else if activeTab === "credentials"}
      <div class="flex flex-col gap-8">
        <Card
          title="Account Password"
          subtitle="One-time password recovery and cryptographic reset credentials"
        >
          <div class="flex items-center justify-between py-2">
            <div class="flex flex-col gap-0.5">
              <span class="text-sm font-semibold text-brand-text-primary">Password Reset</span>
              <span class="text-xs text-brand-text-secondary">
                Request an encrypted time-limited email token to update your login password.
              </span>
            </div>
            <Button
              size="sm"
              onclick={handleSendPasswordReset}
              loading={sendingReset}
            >
              Send Reset Email
            </Button>
          </div>
        </Card>

        <TwoFactorAuth
          mfaEnabled={summary?.settings.mfa_enabled ?? false}
          onStatusChange={loadSecuritySummary}
        />

        <PasskeyManager
          passkeys={summary?.passkeys ?? []}
          onChanged={loadSecuritySummary}
        />

        <BackupCodes
          remainingCount={summary?.backup_codes_count ?? 0}
          onUpdated={loadSecuritySummary}
        />
      </div>
    {:else if activeTab === "sessions"}
      <div class="flex flex-col gap-8">
        <ActiveSessions />
        <DeviceManagement />
      </div>
    {:else if activeTab === "recovery"}
      <div class="flex flex-col gap-8">
        <AccountRecovery
          settings={summary?.settings}
          onUpdated={loadSecuritySummary}
        />
      </div>
    {:else if activeTab === "history"}
      <div class="flex flex-col gap-8">
        <LoginHistory />
      </div>
    {/if}
  </div>
</div>
