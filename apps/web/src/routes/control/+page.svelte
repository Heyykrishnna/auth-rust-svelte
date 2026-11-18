<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import OverviewPanel from '$lib/components/control/OverviewPanel.svelte';
	import AnalyticsPanel from '$lib/components/control/AnalyticsPanel.svelte';
	import AuthConfigPanel from '$lib/components/control/AuthConfigPanel.svelte';
	import UsersDirectoryPanel from '$lib/components/control/UsersDirectoryPanel.svelte';
	import OrganizationsPanel from '$lib/components/control/OrganizationsPanel.svelte';
	import RolesPermissionsPanel from '$lib/components/control/RolesPermissionsPanel.svelte';
	import WebhooksPanel from '$lib/components/control/WebhooksPanel.svelte';
	import KeysDomainsPanel from '$lib/components/control/KeysDomainsPanel.svelte';
	import AuditLogsPanel from '$lib/components/control/AuditLogsPanel.svelte';
	import TeamSecurityPanel from '$lib/components/control/TeamSecurityPanel.svelte';
	import { controlApi } from '$lib/api/control';
	import type {
		ApplicationDomainDto,
		CredentialSummaryDto,
		EnvironmentOverviewDto,
		OAuthProviderConfigDto
	} from '$lib/api/types';
	import {
		canDeleteApplication,
		canEditEnvironmentSettings,
		canMutateEnvironment,
		getInitials
	} from '$lib/control/permissions';
	import { validateSlug } from '$lib/control/validation';
	import {
		controlStore,
		selectApplication,
		selectEnvironmentKind,
		selectWorkspace,
		setApplications,
		setDeveloper,
		showOneTimeSecret
	} from '$lib/stores/control.svelte';

	type SectionId =
		| 'overview'
		| 'analytics'
		| 'auth'
		| 'keys'
		| 'users'
		| 'organizations'
		| 'roles'
		| 'webhooks'
		| 'audit'
		| 'team'
		| 'security';

	let activeSection = $state<SectionId>('overview');
	let mobileSidebarOpen = $state(false);

	let overview = $state<EnvironmentOverviewDto | null>(null);
	let providers = $state<OAuthProviderConfigDto[]>([]);
	let credentials = $state<CredentialSummaryDto[]>([]);
	let domains = $state<ApplicationDomainDto[]>([]);
	let loadingEnv = $state(false);

	let createAppModalOpen = $state(false);
	let newAppName = $state('');
	let newAppSlug = $state('');
	let newAppRedirect = $state('http://localhost:5174/auth/callback');
	let createAppError = $state('');
	let creatingApp = $state(false);

	const activeRole = $derived(controlStore.activeWorkspace?.role ?? 'viewer');
	const allowMutate = $derived(canMutateEnvironment(activeRole));
	const allowPolicyEdit = $derived(canEditEnvironmentSettings(activeRole));
	const allowDeleteApp = $derived(canDeleteApplication(activeRole));

	const sectionTitles: Record<SectionId, string> = {
		overview: 'Overview',
		analytics: 'Analytics',
		auth: 'Authentication & SSO',
		keys: 'API Keys & Domains',
		users: 'Users & Sessions',
		organizations: 'Organizations',
		roles: 'Roles & Permissions',
		webhooks: 'Webhooks & Sync',
		audit: 'Audit Logs',
		team: 'Team & Roles',
		security: 'Account Security'
	};

	onMount(() => {
		const requested = new URLSearchParams(window.location.search).get('section');
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
				controlApi.listDomains(app.id, controlStore.selectedEnvKind)
			]);
			overview = ov;
			providers = prov;
			credentials = creds;
			domains = doms;
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load environment state', 'error');
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

	async function handleWorkspaceChange(e: Event) {
		const wsId = (e.currentTarget as HTMLSelectElement).value;
		selectWorkspace(wsId);
		try {
			const apps = await controlApi.listApplications(wsId);
			setApplications(apps);
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load applications', 'error');
		}
	}

	async function handleCreateApplication() {
		createAppError = '';
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
				initial_dev_redirect_uri: newAppRedirect.trim() || undefined
			});
			createAppModalOpen = false;
			newAppName = '';
			newAppSlug = '';
			showOneTimeSecret({
				title: `Application '${created.application.name}' Provisioned`,
				publishableKey: created.development_publishable_key,
				secretKey: created.initial_development_secret_key
			});
			await refreshWorkspaceApplications();
			selectApplication(created.application.id);
			addToast('Application created with development and production environments', 'success');
		} catch (err) {
			createAppError = err instanceof Error ? err.message : 'Failed to create application';
		} finally {
			creatingApp = false;
		}
	}

	async function handleSignOut() {
		try {
			await controlApi.signOut();
		} finally {
			setDeveloper(null);
			addToast('Signed out of Control Plane', 'info');
			goto('/control/login');
		}
	}

	function selectNav(section: SectionId) {
		activeSection = section;
		mobileSidebarOpen = false;
	}
</script>

<svelte:head>
	<title>{sectionTitles[activeSection]} — Secure Auth Control Plane</title>
</svelte:head>

<div class="console-shell">
	<aside class="sidebar" class:sidebar-open={mobileSidebarOpen} aria-label="Control Plane navigation">
		<div class="sidebar-brand">
			<div class="brand-mark">
				<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
					<line x1="12" y1="2" x2="12" y2="22" />
					<line x1="2" y1="12" x2="22" y2="12" />
					<line x1="5" y1="5" x2="19" y2="19" />
					<line x1="19" y1="5" x2="5" y2="19" />
				</svg>
				<span>Secure Auth</span>
			</div>
			<button
				type="button"
				class="mobile-close"
				onclick={() => (mobileSidebarOpen = false)}
				aria-label="Close sidebar"
			>
				Close
			</button>
		</div>

		<div class="context-selectors">
			<label class="selector-field">
				<span>Workspace</span>
				<select
					aria-label="Workspace selector"
					value={controlStore.activeWorkspace?.id ?? ''}
					onchange={handleWorkspaceChange}
				>
					{#each controlStore.workspaces as ws}
						<option value={ws.id}>{ws.name}</option>
					{/each}
				</select>
			</label>

			<div class="app-selector-row">
				<label class="selector-field flex-1">
					<span>Application</span>
					<select
						aria-label="Application selector"
						value={controlStore.activeApplication?.application.id ?? ''}
						onchange={(e) => selectApplication((e.currentTarget as HTMLSelectElement).value)}
					>
						{#if controlStore.applications.length === 0}
							<option value="">No applications</option>
						{:else}
							{#each controlStore.applications as item}
								<option value={item.application.id}>{item.application.name}</option>
							{/each}
						{/if}
					</select>
				</label>
				{#if allowMutate}
					<button
						type="button"
						class="new-app-btn"
						title="Create application"
						aria-label="Create application"
						onclick={() => {
							createAppError = '';
							createAppModalOpen = true;
						}}
					>
						+
					</button>
				{/if}
			</div>

			<div class="env-switch" role="group" aria-label="Environment switcher">
				<button
					type="button"
					class:active-dev={controlStore.selectedEnvKind === 'development'}
					onclick={() => selectEnvironmentKind('development')}
				>
					Development
				</button>
				<button
					type="button"
					class:active-prod={controlStore.selectedEnvKind === 'production'}
					onclick={() => selectEnvironmentKind('production')}
				>
					Production
				</button>
			</div>
		</div>

		<nav class="nav-groups" aria-label="Dashboard sections">
			<div class="nav-group">
				<span class="nav-label">Workspace</span>
				<button type="button" class="nav-item" class:active={activeSection === 'overview'} onclick={() => selectNav('overview')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<rect x="3" y="3" width="7" height="7" /><rect x="14" y="3" width="7" height="7" /><rect x="3" y="14" width="7" height="7" /><rect x="14" y="14" width="7" height="7" />
					</svg>
					Overview
				</button>
				<button type="button" class="nav-item" class:active={activeSection === 'analytics'} onclick={() => selectNav('analytics')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<line x1="18" y1="20" x2="18" y2="10" /><line x1="12" y1="20" x2="12" y2="4" /><line x1="6" y1="20" x2="6" y2="14" />
					</svg>
					Analytics
				</button>
				<button type="button" class="nav-item" class:active={activeSection === 'auth'} onclick={() => selectNav('auth')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<rect x="3" y="11" width="18" height="11" rx="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" />
					</svg>
					Auth &amp; SSO
				</button>
				<button type="button" class="nav-item" class:active={activeSection === 'keys'} onclick={() => selectNav('keys')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<circle cx="8" cy="15" r="4" /><path d="M10.85 12.15 19 4" /><path d="m18 5 2 2" /><path d="m15 8 2 2" />
					</svg>
					Keys &amp; Domains
				</button>
			</div>

			<div class="nav-group">
				<span class="nav-label">Directory</span>
				<button type="button" class="nav-item" class:active={activeSection === 'users'} onclick={() => selectNav('users')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" /><circle cx="9" cy="7" r="4" /><path d="M23 21v-2a4 4 0 0 0-3-3.87" /><path d="M16 3.13a4 4 0 0 1 0 7.75" />
					</svg>
					Users
					{#if overview}
						<span class="nav-count">{overview.total_users}</span>
					{/if}
				</button>
				<button type="button" class="nav-item" class:active={activeSection === 'organizations'} onclick={() => selectNav('organizations')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<path d="M3 21h18" /><path d="M5 21V7l8-4v18" /><path d="M19 21V11l-6-4" /><path d="M9 9v.01" /><path d="M9 13v.01" /><path d="M9 17v.01" />
					</svg>
					Organizations
				</button>
				<button type="button" class="nav-item" class:active={activeSection === 'webhooks'} onclick={() => selectNav('webhooks')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
					</svg>
					Webhooks &amp; Sync
				</button>
			</div>

			<div class="nav-group">
				<span class="nav-label">Security</span>
				<button type="button" class="nav-item" class:active={activeSection === 'roles'} onclick={() => selectNav('roles')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<rect x="3" y="4" width="18" height="16" rx="2" /><path d="M7 9h10" /><path d="M7 13h6" /><path d="M7 17h4" />
					</svg>
					Roles &amp; Permissions
				</button>
				<button type="button" class="nav-item" class:active={activeSection === 'audit'} onclick={() => selectNav('audit')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><polyline points="14 2 14 8 20 8" /><line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" />
					</svg>
					Audit Logs
				</button>
				<button type="button" class="nav-item" class:active={activeSection === 'team'} onclick={() => selectNav('team')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
					</svg>
					Team &amp; Roles
				</button>
				<button type="button" class="nav-item" class:active={activeSection === 'security'} onclick={() => selectNav('security')}>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
						<circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
					</svg>
					Account Security
				</button>
			</div>
		</nav>

		<div class="sidebar-footer">
			{#if controlStore.developer}
				<div class="dev-card">
					<span class="dev-avatar">
						{getInitials(controlStore.developer.display_name, controlStore.developer.email)}
					</span>
					<div class="dev-meta">
						<strong>{controlStore.developer.display_name || 'Developer'}</strong>
						<span>{controlStore.developer.email}</span>
					</div>
				</div>
				<button type="button" class="signout-btn" onclick={handleSignOut}>Sign out</button>
			{/if}
		</div>
	</aside>

	<div class="workspace-stage">
		<header class="utility-bar">
			<div class="utility-left">
				<button
					type="button"
					class="mobile-menu-btn"
					onclick={() => (mobileSidebarOpen = true)}
					aria-label="Open navigation menu"
				>
					Menu
				</button>
				<div class="breadcrumbs">
					<span>{controlStore.activeWorkspace?.name ?? 'Workspace'}</span>
					<span class="sep">/</span>
					{#if controlStore.activeApplication}
						<span>{controlStore.activeApplication.application.name}</span>
						<span class="sep">/</span>
					{/if}
					<strong>{sectionTitles[activeSection]}</strong>
				</div>
			</div>

			<div class="utility-right">
				<Badge variant={controlStore.selectedEnvKind === 'production' ? 'success' : 'warning'} size="sm" dot>
					{controlStore.selectedEnvKind === 'production' ? 'Production' : 'Development'}
				</Badge>
				<Badge variant="neutral" size="sm">{activeRole}</Badge>
			</div>
		</header>

		<main class="main-canvas">
			{#if !allowMutate}
				<div class="readonly-banner" role="status">
					Read-only access: your workspace role is <strong>{activeRole}</strong>. Contact a workspace admin to perform mutations.
				</div>
			{/if}

			{#if activeSection === 'team' && controlStore.activeWorkspace && controlStore.developer}
				<TeamSecurityPanel
					mode="team"
					workspaceId={controlStore.activeWorkspace.id}
					workspaceName={controlStore.activeWorkspace.name}
					actorRole={activeRole}
					currentDeveloperId={controlStore.developer.id}
				/>
			{:else if activeSection === 'security' && controlStore.activeWorkspace && controlStore.developer}
				<TeamSecurityPanel
					mode="security"
					workspaceId={controlStore.activeWorkspace.id}
					workspaceName={controlStore.activeWorkspace.name}
					actorRole={activeRole}
					currentDeveloperId={controlStore.developer.id}
				/>
			{:else if !controlStore.activeApplication || !controlStore.activeEnvironment}
				<div class="empty-app-state">
					<h1>Create your first application</h1>
					<p>Every application provisions isolated Development and Production environments with separate API keys, users, domains, and webhooks.</p>
					{#if allowMutate}
						<Button variant="primary" onclick={() => (createAppModalOpen = true)}>Create Application</Button>
					{/if}
				</div>
			{:else if activeSection === 'overview'}
				<OverviewPanel
					{overview}
					loading={loadingEnv}
					appName={controlStore.activeApplication.application.name}
					appSlug={controlStore.activeApplication.application.slug}
					onRefresh={loadActiveEnvironmentData}
				/>
			{:else if activeSection === 'analytics'}
				<AnalyticsPanel
					appId={controlStore.activeApplication.application.id}
					envKind={controlStore.selectedEnvKind}
					canMutate={allowMutate}
					onNavigate={(target) => selectNav(target)}
				/>
			{:else if activeSection === 'auth'}
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
			{:else if activeSection === 'users'}
				<UsersDirectoryPanel
					appId={controlStore.activeApplication.application.id}
					envKind={controlStore.selectedEnvKind}
					canMutate={allowMutate}
				/>
			{:else if activeSection === 'organizations'}
				<OrganizationsPanel
					appId={controlStore.activeApplication.application.id}
					envKind={controlStore.selectedEnvKind}
					canMutate={allowMutate}
				/>
			{:else if activeSection === 'roles'}
				<RolesPermissionsPanel
					appId={controlStore.activeApplication.application.id}
					envKind={controlStore.selectedEnvKind}
					canMutate={allowMutate}
				/>
			{:else if activeSection === 'webhooks'}
				<WebhooksPanel
					appId={controlStore.activeApplication.application.id}
					envKind={controlStore.selectedEnvKind}
					canMutate={allowMutate}
				/>
			{:else if activeSection === 'keys'}
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
			{:else if activeSection === 'audit'}
				<AuditLogsPanel
					appId={controlStore.activeApplication.application.id}
					envKind={controlStore.selectedEnvKind}
				/>
			{/if}
		</main>
	</div>
</div>

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
			<input type="text" bind:value={newAppName} placeholder="Ferrum" required />
		</label>
		<label class="modal-field">
			<span>Application Slug</span>
			<input type="text" bind:value={newAppSlug} placeholder="ferrum" required />
		</label>
		<label class="modal-field">
			<span>Initial Development Redirect URI (Optional)</span>
			<input type="url" bind:value={newAppRedirect} placeholder="http://localhost:5174/auth/callback" />
		</label>
	</div>
</Modal>

<style>
	.console-shell {
		display: grid;
		grid-template-columns: 256px 1fr;
		min-height: 100vh;
		background-color: #0b0c0e;
		color: #f4f4f5;
	}

	.sidebar {
		background-color: #111215;
		border-right: 1px solid #1f2228;
		padding: 1.15rem 0.95rem;
		display: flex;
		flex-direction: column;
		gap: 1.15rem;
		position: sticky;
		top: 0;
		height: 100vh;
		overflow-y: auto;
	}

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

	.mobile-close,
	.mobile-menu-btn {
		display: none;
		background: transparent;
		border: 1px solid #2e333d;
		color: #d4d4d8;
		border-radius: 6px;
		padding: 0.28rem 0.6rem;
		font-size: 0.76rem;
		cursor: pointer;
	}

	.context-selectors {
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid #1f2228;
	}

	.selector-field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.selector-field span {
		font-size: 0.66rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: #71717a;
	}

	.selector-field select {
		width: 100%;
		background-color: #15171b;
		border: 1px solid #23262e;
		border-radius: 6px;
		padding: 0.42rem 0.55rem;
		color: #f4f4f5;
		font-size: 0.8rem;
		outline: none;
	}

	.app-selector-row {
		display: flex;
		align-items: flex-end;
		gap: 0.4rem;
	}

	.flex-1 {
		flex: 1;
	}

	.new-app-btn {
		width: 32px;
		height: 32px;
		background-color: #15171b;
		border: 1px solid #2e333d;
		border-radius: 6px;
		color: #ffffff;
		font-size: 1rem;
		cursor: pointer;
		flex-shrink: 0;
	}

	.new-app-btn:hover {
		border-color: #ffffff;
	}

	.env-switch {
		display: grid;
		grid-template-columns: 1fr 1fr;
		background-color: #0b0c0e;
		border: 1px solid #23262e;
		border-radius: 6px;
		padding: 2px;
	}

	.env-switch button {
		background: transparent;
		border: none;
		color: #71717a;
		font-size: 0.74rem;
		font-weight: 500;
		padding: 0.34rem 0.4rem;
		border-radius: 4px;
		cursor: pointer;
	}

	.env-switch button.active-dev {
		background-color: #26200f;
		color: #facc15;
	}

	.env-switch button.active-prod {
		background-color: #11241a;
		color: #4ade80;
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
		color: #52525b;
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
		color: #a1a1aa;
		font-size: 0.84rem;
		font-weight: 500;
		text-align: left;
		cursor: pointer;
	}

	.nav-item:hover {
		background-color: #181a20;
		color: #f4f4f5;
	}

	.nav-item.active {
		background-color: #1c1f26;
		border-color: #2c303a;
		color: #ffffff;
	}

	.nav-count {
		margin-left: auto;
		font-size: 0.7rem;
		font-family: var(--font-mono);
		color: #71717a;
		background-color: #15171b;
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
	}

	.sidebar-footer {
		border-top: 1px solid #1f2228;
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
		color: #0b0c0e;
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
		color: #f4f4f5;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.dev-meta span {
		font-size: 0.7rem;
		color: #71717a;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.signout-btn {
		background-color: #15171b;
		border: 1px solid #23262e;
		color: #a1a1aa;
		border-radius: 6px;
		padding: 0.4rem 0.6rem;
		font-size: 0.76rem;
		cursor: pointer;
	}

	.signout-btn:hover {
		color: #ffffff;
		border-color: #343944;
	}

	.workspace-stage {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.utility-bar {
		height: 54px;
		padding: 0 1.75rem;
		border-bottom: 1px solid #1f2228;
		background-color: #0b0c0e;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		position: sticky;
		top: 0;
		z-index: 30;
	}

	.utility-left,
	.utility-right {
		display: flex;
		align-items: center;
		gap: 0.65rem;
	}

	.breadcrumbs {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-size: 0.82rem;
		color: #71717a;
	}

	.breadcrumbs strong {
		color: #f4f4f5;
		font-weight: 500;
	}

	.sep {
		color: #3f3f46;
	}

	.main-canvas {
		padding: 1.75rem 2rem 3rem;
		max-width: 1320px;
		width: 100%;
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.readonly-banner {
		padding: 0.7rem 1rem;
		background-color: #26200f;
		border: 1px solid #4b3d18;
		color: #facc15;
		border-radius: 8px;
		font-size: 0.82rem;
	}

	.empty-app-state {
		padding: 4rem 2rem;
		text-align: center;
		background-color: #15171b;
		border: 1px solid #23262e;
		border-radius: 12px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.85rem;
	}

	.empty-app-state h1 {
		font-size: 1.45rem;
		margin: 0;
	}

	.empty-app-state p {
		max-width: 480px;
		font-size: 0.88rem;
		color: #a1a1aa;
		margin: 0;
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
		color: #a1a1aa;
	}

	.modal-field input {
		background-color: #0b0c0e;
		border: 1px solid #23262e;
		border-radius: 6px;
		padding: 0.55rem 0.75rem;
		color: #f4f4f5;
		font-size: 0.84rem;
		outline: none;
	}

	@media (max-width: 960px) {
		.console-shell {
			grid-template-columns: 1fr;
		}

		.sidebar {
			position: fixed;
			left: -280px;
			width: 268px;
			z-index: 800;
			transition: left 180ms ease;
		}

		.sidebar.sidebar-open {
			left: 0;
		}

		.mobile-close,
		.mobile-menu-btn {
			display: inline-flex;
		}

		.utility-bar {
			padding: 0 1rem;
		}

		.main-canvas {
			padding: 1.25rem 1rem 2.5rem;
		}
	}
</style>
