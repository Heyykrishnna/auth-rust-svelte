<script lang="ts">
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { onMount, tick } from "svelte";
  import { goto } from "$app/navigation";
  import Button from "$lib/components/Button.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import SignetMark from "$lib/components/SignetMark.svelte";
  import { addToast } from "$lib/components/Toast.svelte";
  import SectionHeader from "$lib/components/control/SectionHeader.svelte";
  import OverviewPanel from "$lib/components/control/OverviewPanel.svelte";
  import AnalyticsPanel from "$lib/components/control/AnalyticsPanel.svelte";
  import AuthConfigPanel from "$lib/components/control/AuthConfigPanel.svelte";
  import UsersDirectoryPanel from "$lib/components/control/UsersDirectoryPanel.svelte";
  import OrganizationsPanel from "$lib/components/control/OrganizationsPanel.svelte";
  import RolesPermissionsPanel from "$lib/components/control/RolesPermissionsPanel.svelte";
  import WebhooksPanel from "$lib/components/control/WebhooksPanel.svelte";
  import KeysDomainsPanel from "$lib/components/control/KeysDomainsPanel.svelte";
  import AuditLogsPanel from "$lib/components/control/AuditLogsPanel.svelte";
  import TeamSecurityPanel from "$lib/components/control/TeamSecurityPanel.svelte";
  import { controlApi } from "$lib/api/control";
  import type {
    ApplicationDomainDto,
    CredentialSummaryDto,
    EnvironmentOverviewDto,
    OAuthProviderConfigDto,
  } from "$lib/api/types";
  import {
    canDeleteApplication,
    canEditEnvironmentSettings,
    canMutateEnvironment,
    getInitials,
  } from "$lib/control/permissions";
  import { validateSlug } from "$lib/control/validation";
  import {
    controlStore,
    selectApplication,
    selectEnvironmentKind,
    selectWorkspace,
    setApplications,
    setDeveloper,
    showOneTimeSecret,
  } from "$lib/stores/control.svelte";

  type SectionId =
    | "overview"
    | "analytics"
    | "auth"
    | "keys"
    | "users"
    | "organizations"
    | "roles"
    | "webhooks"
    | "audit"
    | "team"
    | "security";

  let activeSection = $state<SectionId>("overview");
  let mobileSidebarOpen = $state(false);
  let mobileMenuButton: HTMLButtonElement;
  let mobileCloseButton: HTMLButtonElement;

  let overview = $state<EnvironmentOverviewDto | null>(null);
  let providers = $state<OAuthProviderConfigDto[]>([]);
  let credentials = $state<CredentialSummaryDto[]>([]);
  let domains = $state<ApplicationDomainDto[]>([]);
  let loadingEnv = $state(false);

  let createAppModalOpen = $state(false);
  let newAppName = $state("");
  let newAppSlug = $state("");
  let newAppRedirect = $state("http://localhost:5174/auth/callback");
  let createAppError = $state("");
  let creatingApp = $state(false);

  const activeRole = $derived(controlStore.activeWorkspace?.role ?? "viewer");
  const allowMutate = $derived(canMutateEnvironment(activeRole));
  const allowPolicyEdit = $derived(canEditEnvironmentSettings(activeRole));
  const allowDeleteApp = $derived(canDeleteApplication(activeRole));

  const sectionTitles: Record<SectionId, string> = {
    overview: "Overview",
    analytics: "Analytics",
    auth: "Authentication & SSO",
    keys: "API Keys & Domains",
    users: "Users & Sessions",
    organizations: "Organizations",
    roles: "Roles & Permissions",
    webhooks: "Webhooks & Sync",
    audit: "Audit Logs",
    team: "Team & Roles",
    security: "Account Security",
  };

  onMount(() => {
    const requested = new URLSearchParams(window.location.search).get(
      "section",
    );
    if (requested && requested in sectionTitles) {
      activeSection = requested as SectionId;
    }
  });

  async function loadActiveEnvironmentData() {
    const app = controlStore.activeApplication?.application;
    if (!app) {
      overview = null;
      providers = [];
      credentials = [];
      domains = [];
      return;
    }
    loadingEnv = true;
    try {
      const [ov, prov, creds, doms] = await Promise.all([
        controlApi.getOverview(app.id, controlStore.selectedEnvKind),
        controlApi.listOAuthProviders(app.id, controlStore.selectedEnvKind),
        controlApi.listCredentials(app.id, controlStore.selectedEnvKind),
        controlApi.listDomains(app.id, controlStore.selectedEnvKind),
      ]);
      overview = ov;
      providers = prov;
      credentials = creds;
      domains = doms;
    } catch (err) {
      addToast(
        err instanceof Error ? err.message : "Failed to load environment state",
        "error",
      );
    } finally {
      loadingEnv = false;
    }
  }

  $effect(() => {
    const _appId = controlStore.activeApplication?.application.id;
    const _env = controlStore.selectedEnvKind;
    void _appId;
    void _env;
    loadActiveEnvironmentData();
  });

  async function refreshWorkspaceApplications() {
    const wsId = controlStore.activeWorkspace?.id;
    if (!wsId) return;
    const apps = await controlApi.listApplications(wsId);
    setApplications(apps);
    await loadActiveEnvironmentData();
  }

  async function handleWorkspaceChange(wsId: string) {
    selectWorkspace(wsId);
    setApplications([]);
    try {
      const apps = await controlApi.listApplications(wsId);
      if (controlStore.activeWorkspace?.id === wsId) setApplications(apps);
    } catch (err) {
      addToast(
        err instanceof Error ? err.message : "Failed to load applications",
        "error",
      );
    }
  }

  async function handleCreateApplication() {
    createAppError = "";
    const wsId = controlStore.activeWorkspace?.id;
    if (!wsId) return;
    const slugErr = validateSlug(newAppSlug);
    if (slugErr) {
      createAppError = slugErr;
      return;
    }
    creatingApp = true;
    try {
      const created = await controlApi.createApplication({
        workspace_id: wsId,
        name: newAppName.trim(),
        slug: newAppSlug.trim().toLowerCase(),
        initial_dev_redirect_uri: newAppRedirect.trim() || undefined,
      });
      createAppModalOpen = false;
      newAppName = "";
      newAppSlug = "";
      showOneTimeSecret({
        title: `Application '${created.application.name}' Provisioned`,
        publishableKey: created.development_publishable_key,
        secretKey: created.initial_development_secret_key,
      });
      await refreshWorkspaceApplications();
      selectApplication(created.application.id);
      addToast(
        "Application created with development and production environments",
        "success",
      );
    } catch (err) {
      createAppError =
        err instanceof Error ? err.message : "Failed to create application";
    } finally {
      creatingApp = false;
    }
  }

  async function handleSignOut() {
    try {
      await controlApi.signOut();
    } finally {
      setDeveloper(null);
      addToast("Signed out of Control Plane", "info");
      goto("/control/login");
    }
  }

  async function openSidebar() {
    mobileSidebarOpen = true;
    await tick();
    mobileCloseButton?.focus();
  }

  function closeSidebar() {
    mobileSidebarOpen = false;
    mobileMenuButton?.focus();
  }

  function selectNav(section: SectionId) {
    activeSection = section;
    if (mobileSidebarOpen) closeSidebar();
  }
</script>

<svelte:head>
  <title>{sectionTitles[activeSection]} — Signet Control Plane</title>
</svelte:head>

<svelte:window onkeydown={(event) => {
  if (event.key === "Escape" && mobileSidebarOpen) closeSidebar();
}} />

<div class="workspace-theme grid min-h-dvh grid-cols-[minmax(0,1fr)] bg-brand-surface text-brand-text-primary min-[961px]:grid-cols-[15rem_minmax(0,1fr)]">
  {#if mobileSidebarOpen}
    <button type="button" class="fixed inset-0 z-40 bg-black/60 min-[961px]:hidden" aria-label="Dismiss navigation" onclick={closeSidebar}></button>
  {/if}
  <aside
    class="fixed inset-y-0 left-0 z-50 w-66 flex-col gap-5 overflow-y-auto overscroll-contain border-r border-brand-border bg-brand-surface-card p-4 min-[961px]:sticky min-[961px]:top-0 min-[961px]:z-30 min-[961px]:flex min-[961px]:h-dvh min-[961px]:w-auto {mobileSidebarOpen ? 'flex' : 'hidden'}"
    aria-label="Control Plane navigation"
  >
    <div class="sidebar-brand">
      <div class="brand-mark">
        <SignetMark size={20} class="brand-icon" />
        <span>Signet</span>
      </div>
      <button
        type="button"
        bind:this={mobileCloseButton}
        class="rounded-md border border-brand-border-hover px-2.5 py-1.5 text-xs text-brand-text-primary focus-visible:outline-2 focus-visible:outline-brand-lime min-[961px]:hidden"
        onclick={closeSidebar}
        aria-label="Close sidebar"
      >
        Close
      </button>
    </div>

    <div class="flex flex-col gap-3 border-b border-brand-border pb-4">
      <div class="grid min-w-0 gap-1.5">
        <span class="text-xs font-medium text-brand-text-secondary">Workspace</span>
        <Dropdown
          label="Workspace selector"
          value={controlStore.activeWorkspace?.id ?? ""}
          options={controlStore.workspaces.map((ws) => ({ value: ws.id, label: ws.name }))}
          placeholder="No workspaces"
          onchange={handleWorkspaceChange}
          class="w-full"
        />
      </div>

      <div class="flex min-w-0 items-end gap-2">
        <div class="grid min-w-0 flex-1 gap-1.5">
          <span class="text-xs font-medium text-brand-text-secondary">Application</span>
          <Dropdown
            label="Application selector"
            value={controlStore.activeApplication?.application.id ?? ""}
            options={controlStore.applications.map(({ application }) => ({ value: application.id, label: application.name }))}
            placeholder="No applications"
            onchange={selectApplication}
            class="w-full"
          />
        </div>
        {#if allowMutate}
          <button
            type="button"
            class="grid size-9 shrink-0 place-items-center rounded-lg border border-brand-border-hover bg-brand-surface-input text-lg text-brand-text-primary hover:bg-brand-surface-raised focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-lime"
            title="Create application"
            aria-label="Create application"
            onclick={() => {
              createAppError = "";
              createAppModalOpen = true;
            }}
          >
            <span aria-hidden="true">+</span>
          </button>
        {/if}
      </div>

      <div class="env-switch" role="group" aria-label="Environment switcher">
        <button
          type="button"
          class:active-dev={controlStore.selectedEnvKind === "development"}
          aria-pressed={controlStore.selectedEnvKind === "development"}
          onclick={() => selectEnvironmentKind("development")}
        >
          Development
        </button>
        <button
          type="button"
          class:active-prod={controlStore.selectedEnvKind === "production"}
          aria-pressed={controlStore.selectedEnvKind === "production"}
          onclick={() => selectEnvironmentKind("production")}
        >
          Production
        </button>
      </div>
    </div>

    <nav class="nav-groups" aria-label="Dashboard sections">
      <div class="nav-group">
        <span class="nav-label">Workspace</span>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "overview"}
          onclick={() => selectNav("overview")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <rect x="3" y="3" width="7" height="7" /><rect
              x="14"
              y="3"
              width="7"
              height="7"
            /><rect x="3" y="14" width="7" height="7" /><rect
              x="14"
              y="14"
              width="7"
              height="7"
            />
          </svg>
          Overview
        </button>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "analytics"}
          onclick={() => selectNav("analytics")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <line x1="18" y1="20" x2="18" y2="10" /><line
              x1="12"
              y1="20"
              x2="12"
              y2="4"
            /><line x1="6" y1="20" x2="6" y2="14" />
          </svg>
          Analytics
        </button>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "auth"}
          onclick={() => selectNav("auth")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <rect x="3" y="11" width="18" height="11" rx="2" /><path
              d="M7 11V7a5 5 0 0 1 10 0v4"
            />
          </svg>
          Auth &amp; SSO
        </button>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "keys"}
          onclick={() => selectNav("keys")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <circle cx="8" cy="15" r="4" /><path d="M10.85 12.15 19 4" /><path
              d="m18 5 2 2"
            /><path d="m15 8 2 2" />
          </svg>
          Keys &amp; Domains
        </button>
      </div>

      <div class="nav-group">
        <span class="nav-label">Directory</span>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "users"}
          onclick={() => selectNav("users")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" /><circle
              cx="9"
              cy="7"
              r="4"
            /><path d="M23 21v-2a4 4 0 0 0-3-3.87" /><path
              d="M16 3.13a4 4 0 0 1 0 7.75"
            />
          </svg>
          Users
          {#if overview && overview.total_users > 0}
            <span class="nav-count">{overview.total_users}</span>
          {/if}
        </button>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "organizations"}
          onclick={() => selectNav("organizations")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <path d="M3 21h18" /><path d="M5 21V7l8-4v18" /><path
              d="M19 21V11l-6-4"
            /><path d="M9 9v.01" /><path d="M9 13v.01" /><path d="M9 17v.01" />
          </svg>
          Organizations
        </button>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "webhooks"}
          onclick={() => selectNav("webhooks")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
          </svg>
          Webhooks &amp; Sync
        </button>
      </div>

      <div class="nav-group">
        <span class="nav-label">Security</span>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "roles"}
          onclick={() => selectNav("roles")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <rect x="3" y="4" width="18" height="16" rx="2" /><path
              d="M7 9h10"
            /><path d="M7 13h6" /><path d="M7 17h4" />
          </svg>
          Roles &amp; Permissions
        </button>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "audit"}
          onclick={() => selectNav("audit")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <path
              d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
            /><polyline points="14 2 14 8 20 8" /><line
              x1="16"
              y1="13"
              x2="8"
              y2="13"
            /><line x1="16" y1="17" x2="8" y2="17" />
          </svg>
          Audit Logs
        </button>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "team"}
          onclick={() => selectNav("team")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
          </svg>
          Team &amp; Roles
        </button>
        <button
          type="button"
          class="nav-item"
          class:active={activeSection === "security"}
          onclick={() => selectNav("security")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="3" /><path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
            />
          </svg>
          Account Security
        </button>
      </div>
    </nav>

    <div class="sidebar-footer">
      {#if controlStore.developer}
        <div class="dev-card">
          <span class="dev-avatar">
            {getInitials(
              controlStore.developer.display_name,
              controlStore.developer.email,
            )}
          </span>
          <div class="dev-meta">
            <strong>{controlStore.developer.display_name || "Developer"}</strong
            >
            <span>{controlStore.developer.email}</span>
          </div>
        </div>
        <button type="button" class="signout-btn" onclick={handleSignOut}
          >Sign out</button
        >
      {/if}
    </div>
  </aside>

  <div class="min-w-0">
    <header class="sticky top-0 z-30 border-b border-brand-border bg-brand-surface/95 backdrop-blur">
      <div class="mx-auto flex min-h-14 w-full max-w-7xl items-center justify-between gap-3 px-4 sm:px-6 lg:px-8">
        <div class="flex min-w-0 items-center gap-3">
          <button
            bind:this={mobileMenuButton}
            type="button"
            class="shrink-0 rounded-md border border-brand-border-hover px-2.5 py-1.5 text-xs text-brand-text-primary focus-visible:outline-2 focus-visible:outline-brand-lime min-[961px]:hidden"
            onclick={openSidebar}
            aria-label="Open navigation menu"
            aria-expanded={mobileSidebarOpen}
          >Menu</button>
          <nav class="flex min-w-0 items-center gap-2 text-sm" aria-label="Current section">
            <span class="hidden truncate text-brand-text-secondary sm:inline">{controlStore.activeWorkspace?.name ?? "Workspace"}</span>
            <span class="hidden text-brand-text-muted sm:inline" aria-hidden="true">/</span>
            {#if controlStore.activeApplication}
              <span class="hidden truncate text-brand-text-secondary md:inline">{controlStore.activeApplication.application.name}</span>
              <span class="hidden text-brand-text-muted md:inline" aria-hidden="true">/</span>
            {/if}
            <span class="min-w-0 truncate font-medium text-brand-text-primary" aria-current="page">{sectionTitles[activeSection]}</span>
          </nav>
        </div>

        {#if controlStore.activeApplication}
          <div class="flex shrink-0 items-center gap-2 text-xs text-brand-text-secondary">
            <span class="size-1.5 rounded-full {controlStore.selectedEnvKind === 'production' ? 'bg-brand-success' : 'bg-brand-warning'}" aria-hidden="true"></span>
            {controlStore.selectedEnvKind === "production" ? "Production" : "Development"}
          </div>
        {/if}
      </div>
    </header>

    <main class="mx-auto grid w-full max-w-7xl min-w-0 grid-cols-1 gap-5 px-4 py-5 sm:gap-6 sm:px-6 sm:py-7 lg:px-8">
      {#if !allowMutate}
        <div class="readonly-banner" role="status">
          Read-only access: your workspace role is <strong>{activeRole}</strong
          >. Contact a workspace admin to perform mutations.
        </div>
      {/if}

      {#if activeSection === "team" && controlStore.activeWorkspace && controlStore.developer}
        <TeamSecurityPanel
          mode="team"
          workspaceId={controlStore.activeWorkspace.id}
          workspaceName={controlStore.activeWorkspace.name}
          actorRole={activeRole}
          currentDeveloperId={controlStore.developer.id}
        />
      {:else if activeSection === "security" && controlStore.activeWorkspace && controlStore.developer}
        <TeamSecurityPanel
          mode="security"
          workspaceId={controlStore.activeWorkspace.id}
          workspaceName={controlStore.activeWorkspace.name}
          actorRole={activeRole}
          currentDeveloperId={controlStore.developer.id}
        />
      {:else if !controlStore.activeApplication || !controlStore.activeEnvironment}
        <SectionHeader title={sectionTitles[activeSection]} description="Select an application to manage its environments." />
        <div class="grid justify-items-start gap-3 rounded-xl border border-brand-border bg-brand-surface-card p-6 sm:p-8">
          <h2 class="m-0 text-xl font-semibold text-brand-text-primary">Create your first application</h2>
          <p class="max-w-lg text-sm leading-relaxed text-brand-text-secondary">Each application has separate development and production environments, API keys, and users.</p>
          {#if allowMutate}
            <Button
              variant="primary"
              onclick={() => (createAppModalOpen = true)}
              >Create Application</Button
            >
          {/if}
        </div>
      {:else if activeSection === "overview"}
        <OverviewPanel
          {overview}
          loading={loadingEnv}
          appName={controlStore.activeApplication.application.name}
          appSlug={controlStore.activeApplication.application.slug}
          environmentKind={controlStore.selectedEnvKind}
          onRefresh={loadActiveEnvironmentData}
        />
      {:else if activeSection === "analytics"}
        <AnalyticsPanel
          appId={controlStore.activeApplication.application.id}
          envKind={controlStore.selectedEnvKind}
          canMutate={allowMutate}
          onNavigate={(target) => selectNav(target)}
        />
      {:else if activeSection === "auth"}
        <AuthConfigPanel
          appId={controlStore.activeApplication.application.id}
          appName={controlStore.activeApplication.application.name}
          envKind={controlStore.selectedEnvKind}
          environment={controlStore.activeEnvironment}
          {providers}
          canEditPolicies={allowPolicyEdit}
          canEditProviders={allowMutate}
          onSaved={refreshWorkspaceApplications}
        />
      {:else if activeSection === "users"}
        <UsersDirectoryPanel
          appId={controlStore.activeApplication.application.id}
          envKind={controlStore.selectedEnvKind}
          canMutate={allowMutate}
        />
      {:else if activeSection === "organizations"}
        <OrganizationsPanel
          appId={controlStore.activeApplication.application.id}
          envKind={controlStore.selectedEnvKind}
          canMutate={allowMutate}
        />
      {:else if activeSection === "roles"}
        <RolesPermissionsPanel
          appId={controlStore.activeApplication.application.id}
          envKind={controlStore.selectedEnvKind}
          canMutate={allowMutate}
        />
      {:else if activeSection === "webhooks"}
        <WebhooksPanel
          appId={controlStore.activeApplication.application.id}
          envKind={controlStore.selectedEnvKind}
          canMutate={allowMutate}
        />
      {:else if activeSection === "keys"}
        <KeysDomainsPanel
          appId={controlStore.activeApplication.application.id}
          appName={controlStore.activeApplication.application.name}
          envKind={controlStore.selectedEnvKind}
          {credentials}
          {domains}
          canMutate={allowMutate}
          canDeleteApp={allowDeleteApp}
          onReload={loadActiveEnvironmentData}
          onAppDeleted={refreshWorkspaceApplications}
        />
      {:else if activeSection === "audit"}
        <AuditLogsPanel
          appId={controlStore.activeApplication.application.id}
          envKind={controlStore.selectedEnvKind}
        />
      {/if}
    </main>
  </div>
</div>

<div class="workspace-theme">
<Modal
  open={createAppModalOpen}
  title="Create Application"
  description="Provision a new tenant application with isolated Development and Production environments."
  confirmText="Create Application"
  loading={creatingApp}
  onconfirm={handleCreateApplication}
  oncancel={() => (createAppModalOpen = false)}
>
  <div class="modal-fields">
    {#if createAppError}
      <p class="error-text" role="alert">{createAppError}</p>
    {/if}
    <label class="modal-field">
      <span>Application Name</span>
      <input
        type="text"
        bind:value={newAppName}
        placeholder="Ferrum"
        required
      />
    </label>
    <label class="modal-field">
      <span>Application Slug</span>
      <input
        type="text"
        bind:value={newAppSlug}
        placeholder="ferrum"
        required
      />
    </label>
    <label class="modal-field">
      <span>Initial Development Redirect URI (Optional)</span>
      <input
        type="url"
        bind:value={newAppRedirect}
        placeholder="http://localhost:5174/auth/callback"
      />
    </label>
  </div>
</Modal>
</div>

<style>
  .sidebar-brand {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.35rem;
  }

  .brand-mark {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    font-weight: 600;
    font-size: 0.95rem;
    color: #ffffff;
  }

  .env-switch {
    display: grid;
    grid-template-columns: 1fr 1fr;
    background-color: var(--color-brand-surface);
    border: 1px solid var(--color-brand-border);
    border-radius: 6px;
    padding: 2px;
  }

  .env-switch button {
    background: transparent;
    border: none;
    color: var(--color-brand-text-secondary);
    font-size: 0.74rem;
    font-weight: 500;
    padding: 0.34rem 0.4rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .env-switch button.active-dev {
    background-color: #202020;
    color: #c9c9c9;
  }

  .env-switch button.active-prod {
    background-color: #1f1f1f;
    color: #b8b8b8;
  }

  .nav-groups {
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
    flex: 1;
  }

  .nav-group {
    display: flex;
    flex-direction: column;
    gap: 0.18rem;
  }

  .nav-label {
    font-size: 0.66rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-brand-text-secondary);
    padding: 0 0.55rem 0.25rem;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    padding: 0.5rem 0.6rem;
    color: var(--color-brand-text-secondary);
    font-size: 0.84rem;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }

  .nav-item:hover {
    background-color: var(--color-brand-surface-raised);
    color: var(--color-brand-text-primary);
  }

  .nav-item.active {
    background-color: var(--color-brand-surface-alt);
    border-color: var(--color-brand-border-hover);
    color: #ffffff;
  }

  .nav-count {
    margin-left: auto;
    font-size: 0.7rem;
    font-family: var(--font-mono);
    color: var(--color-brand-text-secondary);
    background-color: var(--color-brand-surface-card);
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
  }

  .sidebar-footer {
    border-top: 1px solid var(--color-brand-border);
    padding-top: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .dev-card {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .dev-avatar {
    width: 30px;
    height: 30px;
    border-radius: 9999px;
    background-color: #ffffff;
    color: var(--color-brand-surface);
    font-size: 0.72rem;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .dev-meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .dev-meta strong {
    font-size: 0.8rem;
    color: var(--color-brand-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dev-meta span {
    font-size: 0.7rem;
    color: var(--color-brand-text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .signout-btn {
    background-color: var(--color-brand-surface-card);
    border: 1px solid var(--color-brand-border);
    color: var(--color-brand-text-secondary);
    border-radius: 6px;
    padding: 0.4rem 0.6rem;
    font-size: 0.76rem;
    cursor: pointer;
  }

  .signout-btn:hover {
    color: #ffffff;
    border-color: var(--color-brand-border-strong);
  }

  .readonly-banner {
    padding: 0.7rem 1rem;
    background-color: #202020;
    border: 1px solid #3d3d3d;
    color: #c9c9c9;
    border-radius: 8px;
    font-size: 0.82rem;
  }

  .modal-fields {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    margin-top: 0.75rem;
  }

  .modal-field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .modal-field span {
    font-size: 0.74rem;
    color: var(--color-brand-text-secondary);
  }

  .modal-field input {
    background-color: var(--color-brand-surface);
    border: 1px solid var(--color-brand-border);
    border-radius: 6px;
    padding: 0.55rem 0.75rem;
    color: var(--color-brand-text-primary);
    font-size: 0.84rem;
    outline: none;
  }
</style>
