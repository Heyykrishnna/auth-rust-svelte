<script lang="ts">
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { authStore, logout } from "$lib/stores/auth.svelte";
  import { getUserInitials, formatUserDisplayName } from "$lib/auth/session";
  import { addToast } from "$lib/components/Toast.svelte";
  import Badge from "./Badge.svelte";
  import Button from "./Button.svelte";
  import SignetLogo from "./SignetLogo.svelte";

  let loggingOut = $state(false);

  async function handleLogout() {
    loggingOut = true;
    try {
      await logout();
      addToast("Signed out successfully", "info");
      goto("/login");
    } finally {
      loggingOut = false;
    }
  }

  function isActive(path: string): boolean {
    return (
      page.url.pathname === path || page.url.pathname.startsWith(`${path}/`)
    );
  }
</script>

<header
  class="sticky top-0 z-50 bg-brand-surface/95 backdrop-blur-md border-b border-brand-border"
>
  <div class="max-w-7xl mx-auto px-6 h-15 flex items-center justify-between">
    <a
      href={authStore.isAuthenticated ? "/dashboard" : "/"}
      class="flex items-center gap-2.5 text-brand-text-primary no-underline font-semibold text-base tracking-tight"
      aria-label="Signet home"
    >
      <SignetLogo variant="light" height={24} />
    </a>

    <nav class="flex items-center gap-1.5" aria-label="Main Navigation">
      {#if authStore.isAuthenticated}
        <a
          href="/dashboard"
          class="text-sm font-medium px-3 py-1.5 rounded-md transition-colors border no-underline {isActive(
            '/dashboard',
          )
            ? 'text-brand-text-primary bg-brand-surface-raised border-brand-border-hover'
            : 'text-brand-text-muted hover:text-brand-text-primary hover:bg-brand-surface-raised border-transparent'}"
        >
          Dashboard
        </a>
        <a
          href="/settings"
          class="text-sm font-medium px-3 py-1.5 rounded-md transition-colors border no-underline {isActive(
            '/settings',
          )
            ? 'text-brand-text-primary bg-brand-surface-raised border-brand-border-hover'
            : 'text-brand-text-muted hover:text-brand-text-primary hover:bg-brand-surface-raised border-transparent'}"
        >
          Settings
        </a>
        <a
          href="/control"
          class="text-sm font-medium px-3 py-1.5 rounded-md transition-colors border no-underline {isActive(
            '/control',
          )
            ? 'text-brand-text-primary bg-brand-surface-raised border-brand-border-hover'
            : 'text-brand-text-muted hover:text-brand-text-primary hover:bg-brand-surface-raised border-transparent'}"
        >
          Control Plane
        </a>
        {#if authStore.isAdmin}
          <a
            href="/admin"
            class="text-sm font-medium px-3 py-1.5 rounded-md transition-colors border no-underline {isActive(
              '/admin',
            )
              ? 'text-brand-text-primary bg-brand-surface-raised border-brand-border-hover'
              : 'text-brand-text-muted hover:text-brand-text-primary hover:bg-brand-surface-raised border-transparent'}"
          >
            Admin
          </a>
        {/if}
      {:else}
        <a
          href="/"
          class="text-sm font-medium px-3 py-1.5 rounded-md transition-colors border no-underline {page
            .url.pathname === '/'
            ? 'text-brand-text-primary bg-brand-surface-raised border-brand-border-hover'
            : 'text-brand-text-muted hover:text-brand-text-primary hover:bg-brand-surface-raised border-transparent'}"
        >
          Overview
        </a>
        <a
          href="/control"
          class="text-sm font-medium px-3 py-1.5 rounded-md transition-colors border no-underline {isActive(
            '/control',
          )
            ? 'text-brand-text-primary bg-brand-surface-raised border-brand-border-hover'
            : 'text-brand-text-muted hover:text-brand-text-primary hover:bg-brand-surface-raised border-transparent'}"
        >
          Control Plane
        </a>
        <a
          href="/login"
          class="text-sm font-medium px-3 py-1.5 rounded-md transition-colors border no-underline {isActive(
            '/login',
          )
            ? 'text-brand-text-primary bg-brand-surface-raised border-brand-border-hover'
            : 'text-brand-text-muted hover:text-brand-text-primary hover:bg-brand-surface-raised border-transparent'}"
        >
          Sign In
        </a>
        <a
          href="/register"
          class="text-sm font-medium px-3 py-1.5 rounded-md transition-colors border no-underline {isActive(
            '/register',
          )
            ? 'text-brand-text-primary bg-brand-surface-raised border-brand-border-hover'
            : 'text-brand-text-muted hover:text-brand-text-primary hover:bg-brand-surface-raised border-transparent'}"
        >
          Create Account
        </a>
      {/if}
    </nav>

    <div class="flex items-center gap-3">
      {#if authStore.isAuthenticated && authStore.user}
        <div
          class="flex items-center gap-2.5 pl-1 pr-2.5 py-1 rounded-full bg-brand-surface-card border border-brand-border"
        >
          <div
            class="size-7 rounded-full bg-brand-lime text-brand-lime-ink flex items-center justify-center font-heading font-bold text-xs select-none"
            title={authStore.user.email}
          >
            {getUserInitials(authStore.user)}
          </div>
          <div class="hidden md:flex items-center gap-2">
            <span
              class="text-xs font-medium text-brand-text-primary max-w-35 truncate"
            >
              {formatUserDisplayName(authStore.user)}
            </span>
            <Badge variant="neutral" size="sm">
              {authStore.isAdmin ? "Admin" : "User"}
            </Badge>
          </div>
        </div>

        <Button
          variant="danger"
          size="sm"
          onclick={handleLogout}
          loading={loggingOut}
          disabled={loggingOut}
        >
          Sign Out
        </Button>
      {:else}
        <Button size="sm" href="/control/login">Console</Button>
      {/if}
    </div>
  </div>
</header>
