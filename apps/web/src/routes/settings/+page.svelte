<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Button from "$lib/components/Button.svelte";
  import RefreshButton from "$lib/components/RefreshButton.svelte";
  import DeleteButton from "$lib/components/DeleteButton.svelte";
  import Input from "$lib/components/Input.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { authStore, setAuth } from "$lib/stores/auth.svelte";
  import { addToast } from "$lib/components/Toast.svelte";
  import { usersApi } from "$lib/api/users";
  import { sessionsApi } from "$lib/api/sessions";
  import { authApi } from "$lib/api/auth";
  import type { SessionItem } from "$lib/api/types";
  import { ApiError } from "$lib/api/client";
  import { onMount } from "svelte";

  let displayName = $state("");
  let profileLoading = $state(false);
  let profileError = $state("");

  let sessions = $state<SessionItem[]>([]);
  let sessionsLoading = $state(false);
  let sessionToRevoke = $state<SessionItem | null>(null);
  let revokingSession = $state(false);

  let sendingReset = $state(false);

  onMount(async () => {
    if (authStore.user) {
      displayName = authStore.user.display_name;
    }
    await loadSessions();
  });

  $effect(() => {
    if (authStore.user && !displayName) {
      displayName = authStore.user.display_name;
    }
  });

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

  async function loadSessions() {
    sessionsLoading = true;
    try {
      sessions = await sessionsApi.listSessions();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      }
    } finally {
      sessionsLoading = false;
    }
  }

  function confirmRevokeSession(session: SessionItem) {
    sessionToRevoke = session;
  }

  async function executeRevokeSession() {
    if (!sessionToRevoke) return;
    revokingSession = true;
    try {
      await sessionsApi.revokeSession(sessionToRevoke.id);
      sessions = sessions.filter((s) => s.id !== sessionToRevoke?.id);
      addToast("Session revoked successfully", "success");
      sessionToRevoke = null;
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to revoke session", "error");
      }
    } finally {
      revokingSession = false;
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

  function formatTime(iso: string): string {
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }

  function parseDevice(ua: string | null): string {
    if (!ua) return "Unknown Device";
    if (ua.includes("Macintosh")) return "macOS Device";
    if (ua.includes("Windows")) return "Windows PC";
    if (ua.includes("Linux")) return "Linux Device";
    if (ua.includes("iPhone") || ua.includes("iPad")) return "iOS Device";
    if (ua.includes("Android")) return "Android Device";
    return "Web Browser";
  }
</script>

<svelte:head>
  <title>Settings — Signet</title>
  <meta
    name="description"
    content="Manage your account profile, active sessions, and security preferences."
  />
</svelte:head>

<div class="max-w-4xl mx-auto px-6 py-8 pb-16">
  <div class="mb-8">
    <h1 class="text-3xl font-bold font-heading text-brand-text-primary tracking-tight mb-1">Account Settings</h1>
    <p class="text-sm text-brand-text-secondary m-0">
      Manage your profile credentials and connected sessions.
    </p>
  </div>

  <div class="flex flex-col gap-8">
    <Card
      title="Profile Information"
      subtitle="Update your public display identity"
    >
      <form onsubmit={handleUpdateProfile} class="flex flex-col gap-5">
        {#if profileError}
          <div class="p-3.5 bg-rose-500/10 border border-rose-500/30 rounded-lg text-rose-300 text-sm" role="alert">{profileError}</div>
        {/if}

        <div class="flex flex-col gap-1.5">
          <label class="text-xs font-medium text-brand-text-muted" for="email">Account Email</label>
          <input
            id="email"
            type="email"
            value={authStore.user?.email || ""}
            disabled
            class="w-full px-3.5 py-2.5 bg-brand-surface-card border border-brand-border rounded-md text-brand-text-secondary font-sans text-sm outline-none opacity-60 cursor-not-allowed"
          />
          <span class="text-xs text-brand-text-subtle"
            >Email address cannot be changed directly.</span
          >
        </div>

        <Input
          id="displayName"
          label="Display Name"
          bind:value={displayName}
          required
          placeholder="Jane Doe"
          hint="This name appears in greetings and team interfaces."
        />

        <div class="pt-2">
          <Button type="submit" loading={profileLoading}>
            Save Changes
          </Button>
        </div>
      </form>
    </Card>

    <Card
      title="Security & Authentication"
      subtitle="Cryptographic credential management"
    >
      <div class="flex flex-col divide-y divide-white/5">
        <div class="flex items-center justify-between py-4 first:pt-0">
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-semibold text-brand-text-primary">Password Reset</span>
            <span class="text-xs text-brand-text-secondary"
              >Receive a secure one-time link to update your password.</span
            >
          </div>
          <Button
            size="sm"
            onclick={handleSendPasswordReset}
            loading={sendingReset}
          >
            Send Reset Email
          </Button>
        </div>

        <div class="flex items-center justify-between py-4">
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-semibold text-brand-text-primary">Email Verification</span>
            <span class="text-xs text-brand-text-secondary">
              {authStore.user?.email_verified
                ? "Your email address has been verified."
                : "Verification required to access all platform features."}
            </span>
          </div>
          {#if authStore.user?.email_verified}
            <Badge variant="success">Verified</Badge>
          {:else}
            <Badge variant="warning">Pending</Badge>
          {/if}
        </div>

        <div class="flex items-center justify-between py-4 last:pb-0">
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-semibold text-brand-text-primary">Session Tokens</span>
            <span class="text-xs text-brand-text-secondary"
              >Tokens are isolated in HttpOnly, SameSite=Lax cookies.</span
            >
          </div>
          <Badge variant="primary">HttpOnly Secure</Badge>
        </div>
      </div>
    </Card>

    <div>
      <Card
        title="Active Login Sessions"
        subtitle="Devices currently authenticated to your account via the Rust Auth API"
      >
        {#snippet headerActions()}
          <RefreshButton
            onclick={loadSessions}
            loading={sessionsLoading}
            label="Refresh sessions"
          />
        {/snippet}

        {#if sessionsLoading && sessions.length === 0}
          <div class="text-center py-8 text-sm text-brand-text-muted">Loading active sessions...</div>
        {:else if sessions.length === 0}
          <div class="text-center py-8 text-sm text-brand-text-muted">No other active sessions detected.</div>
        {:else}
          <div class="flex flex-col gap-4">
            {#each sessions as session (session.id)}
              <div
                class="flex items-center gap-4 p-4 rounded-xl border {session.is_current ? 'border-emerald-500/40 bg-emerald-950/10' : 'border-brand-border bg-brand-surface-card'}"
              >
                <div class="text-xs font-mono font-bold text-brand-lime w-10 h-10 rounded-lg bg-white/5 border border-white/10 flex items-center justify-center shrink-0">
                  SES
                </div>
                <div class="flex-1 flex flex-col gap-1 min-w-0">
                  <div class="flex items-center gap-2.5">
                    <span class="text-sm font-semibold text-brand-text-primary"
                      >{parseDevice(session.user_agent)}</span
                    >
                    {#if session.is_current}
                      <Badge variant="success" size="sm" dot
                        >Current Session</Badge
                      >
                    {/if}
                  </div>
                  <div class="flex items-center gap-2 text-xs text-brand-text-muted">
                    <span>IP: {session.ip_address || "127.0.0.1"}</span>
                    <span>•</span>
                    <span>Last active: {formatTime(session.last_used_at)}</span>
                  </div>
                  {#if session.user_agent}
                    <div class="text-[11px] text-brand-text-subtle truncate max-w-lg">{session.user_agent}</div>
                  {/if}
                </div>
                <div>
                  {#if !session.is_current}
                    <DeleteButton
                      label="Revoke session"
                      onclick={() => confirmRevokeSession(session)}
                    />
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </Card>
    </div>
  </div>
</div>

<Modal
  open={sessionToRevoke !== null}
  title="Revoke Active Session?"
  description="This action will terminate the session on that device immediately. The device will be signed out from the Rust API."
  confirmText="Revoke Session"
  variant="danger"
  loading={revokingSession}
  onconfirm={executeRevokeSession}
  oncancel={() => (sessionToRevoke = null)}
/>
