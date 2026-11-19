<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Button from "$lib/components/Button.svelte";
  import RefreshButton from "$lib/components/RefreshButton.svelte";
  import DeleteButton from "$lib/components/DeleteButton.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import StepUpModal from "./StepUpModal.svelte";
  import { sessionsApi } from "$lib/api/sessions";
  import { securityApi } from "$lib/api/security";
  import type { SessionItem } from "$lib/api/types";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";
  import { onMount } from "svelte";

  let sessions = $state<SessionItem[]>([]);
  let loading = $state(false);
  let sessionToRevoke = $state<SessionItem | null>(null);
  let revoking = $state(false);

  let stepUpOpen = $state(false);
  let revokingAllOthers = $state(false);

  onMount(async () => {
    await loadSessions();
  });

  export async function loadSessions() {
    loading = true;
    try {
      sessions = await sessionsApi.listSessions();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      }
    } finally {
      loading = false;
    }
  }

  function confirmRevokeSession(session: SessionItem) {
    sessionToRevoke = session;
  }

  async function executeRevokeSession() {
    if (!sessionToRevoke) return;
    revoking = true;
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
      revoking = false;
    }
  }

  function handleRevokeOthersClick() {
    stepUpOpen = true;
  }

  async function handleStepUpSuccess(token: string) {
    stepUpOpen = false;
    revokingAllOthers = true;
    try {
      await securityApi.revokeOtherSessions(token);
      await loadSessions();
      addToast("All other active sessions have been terminated", "success");
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to revoke other sessions", "error");
      }
    } finally {
      revokingAllOthers = false;
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

  function parseDevice(ua: string | null): string {
    if (!ua) return "Unknown Device";
    if (ua.includes("Macintosh")) return "macOS Device";
    if (ua.includes("Windows")) return "Windows PC";
    if (ua.includes("Linux")) return "Linux Device";
    if (ua.includes("iPhone")) return "iPhone";
    if (ua.includes("iPad")) return "iPad";
    if (ua.includes("Android")) return "Android Device";
    return "Web Client";
  }
</script>

<Card
  title="Active Login Sessions"
  subtitle="Connected clients currently authorized with valid session refresh tokens"
>
  {#snippet headerActions()}
    <div class="flex items-center gap-2">
      {#if sessions.filter((s) => !s.is_current).length > 0}
        <Button
          size="sm"
          variant="danger"
          onclick={handleRevokeOthersClick}
          loading={revokingAllOthers}
          disabled={revokingAllOthers}
        >
          Revoke Other Sessions
        </Button>
      {/if}
      <RefreshButton
        onclick={loadSessions}
        {loading}
        label="Refresh active sessions"
      />
    </div>
  {/snippet}

  {#if loading && sessions.length === 0}
    <div class="text-center py-8 text-sm text-brand-text-muted">Loading active sessions...</div>
  {:else if sessions.length === 0}
    <div class="text-center py-8 text-sm text-brand-text-muted">No active sessions detected.</div>
  {:else}
    <div class="flex flex-col gap-3">
      {#each sessions as session (session.id)}
        <div
          class="flex items-center justify-between p-4 rounded-xl border {session.is_current ? 'border-emerald-500/40 bg-emerald-950/10' : 'border-brand-border bg-brand-surface-card'}"
        >
          <div class="flex items-center gap-3.5 min-w-0">
            <div class="size-10 rounded-lg bg-white/5 border border-white/10 flex items-center justify-center shrink-0 text-brand-lime font-mono text-xs font-bold">
              SES
            </div>
            <div class="flex flex-col gap-1 min-w-0">
              <div class="flex items-center gap-2">
                <span class="text-sm font-semibold text-brand-text-primary">
                  {parseDevice(session.user_agent)}
                </span>
                {#if session.is_current}
                  <Badge variant="success" size="sm" dot>Current Session</Badge>
                {/if}
              </div>
              <div class="flex items-center gap-2 text-xs text-brand-text-muted flex-wrap">
                <span>IP: {session.ip_address || "127.0.0.1"}</span>
                <span>•</span>
                <span>Last active: {formatTime(session.last_used_at)}</span>
              </div>
              {#if session.user_agent}
                <div class="text-[11px] text-brand-text-subtle truncate max-w-md">
                  {session.user_agent}
                </div>
              {/if}
            </div>
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

<Modal
  open={sessionToRevoke !== null}
  title="Revoke Active Session?"
  description="This action will invalidate the refresh token on that device immediately, signing it out from the API."
  confirmText="Revoke Session"
  variant="danger"
  loading={revoking}
  onconfirm={executeRevokeSession}
  oncancel={() => (sessionToRevoke = null)}
/>

<StepUpModal
  open={stepUpOpen}
  title="Step-Up: Revoke All Other Sessions"
  description="Confirm your password or authenticator code to sign out all other active sessions across your devices."
  onSuccess={handleStepUpSuccess}
  onCancel={() => (stepUpOpen = false)}
/>

