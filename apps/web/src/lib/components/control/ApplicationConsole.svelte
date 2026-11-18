<script lang="ts">
	import Dropdown from '$lib/components/Dropdown.svelte';
	import { tick } from 'svelte';
	import { page } from '$app/state';
	import { workspaceHref } from '$lib/control/navigation';
	import { goto } from '$app/navigation';
	import SignetMark from '$lib/components/SignetMark.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import SectionHeader from '$lib/components/control/SectionHeader.svelte';
	import OverviewPanel from '$lib/components/control/OverviewPanel.svelte';
	import AnalyticsPanel from '$lib/components/control/AnalyticsPanel.svelte';
	import AuthConfigPanel from '$lib/components/control/AuthConfigPanel.svelte';
	import UsersDirectoryPanel from '$lib/components/control/UsersDirectoryPanel.svelte';
	import OrganizationsPanel from '$lib/components/control/OrganizationsPanel.svelte';
	import RolesPermissionsPanel from '$lib/components/control/RolesPermissionsPanel.svelte';
	import WebhooksPanel from '$lib/components/control/WebhooksPanel.svelte';
	import KeysDomainsPanel from '$lib/components/control/KeysDomainsPanel.svelte';
	import AuditLogsPanel from '$lib/components/control/AuditLogsPanel.svelte';
	import AccountSecurityPanel from '$lib/components/control/AccountSecurityPanel.svelte';
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
	import {
		controlStore,
		selectEnvironmentKind
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
		| 'security';

	let {
		onCreateApplication,
		onSignOut,
		onRefreshApplications
	}: {
		onCreateApplication: () => void;
		onSignOut: () => void;
		onRefreshApplications: () => Promise<void>;
	} = $props();

	const appId = $derived(controlStore.activeApplication?.application.id);
	const environmentKind = $derived(controlStore.selectedEnvKind);
	let mobileSidebarOpen = $state(false);
	let mobileMenuButton: HTMLButtonElement;
	let mobileCloseButton: HTMLButtonElement;

	let overview = $state<EnvironmentOverviewDto | null>(null);
	let providers = $state<OAuthProviderConfigDto[]>([]);
	let credentials = $state<CredentialSummaryDto[]>([]);
	let domains = $state<ApplicationDomainDto[]>([]);
	let loadingEnv = $state(false);

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
		security: 'Account Security'
	};

	const requestedSection = $derived(page.url.searchParams.get('section'));
	const activeSection = $derived(
		(Object.keys(sectionTitles) as SectionId[]).find(
			(section) => section === requestedSection
		) ?? 'overview'
	);

	let environmentRequest = 0;

	async function loadActiveEnvironmentData() {
		const request = ++environmentRequest;
		const id = appId;
		const kind = environmentKind;
		if (!id) return;
		loadingEnv = true;
		try {
			const [ov, prov, creds, doms] = await Promise.all([
				controlApi.getOverview(id, kind),
				controlApi.listOAuthProviders(id, kind),
				controlApi.listCredentials(id, kind),
				controlApi.listDomains(id, kind)
			]);
			if (request !== environmentRequest) return;
			overview = ov;
			providers = prov;
			credentials = creds;
			domains = doms;
		} catch (err) {
			if (request !== environmentRequest) return;
			addToast(
				err instanceof Error ? err.message : 'Failed to load environment state',
				'error'
			);
		} finally {
			if (request === environmentRequest) loadingEnv = false;
		}
	}

	$effect(() => {
		overview = null;
		providers = [];
		credentials = [];
		domains = [];
		void loadActiveEnvironmentData();
		return () => {
			environmentRequest += 1;
		};
	});

	async function refreshWorkspaceApplications() {
		await onRefreshApplications();
		await loadActiveEnvironmentData();
	}

	async function handleAppDeleted() {
		await onRefreshApplications();
		await goto(workspaceHref(controlStore.activeWorkspace?.id));
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
		void goto(
			workspaceHref(
				controlStore.activeWorkspace?.id,
				'applications',
				appId,
				section
			)
		);
		if (mobileSidebarOpen) closeSidebar();
	}
</script>

<svelte:head>
	<title>{sectionTitles[activeSection]} — Signet Control Plane</title>
</svelte:head>

<svelte:window
	onkeydown={(event) => {
		if (event.key === 'Escape' && mobileSidebarOpen) closeSidebar();
	}}
/>

<div
	class="workspace-theme workspace-accent grid min-h-dvh grid-cols-[minmax(0,1fr)] bg-brand-surface text-brand-text-primary min-[961px]:grid-cols-[15rem_minmax(0,1fr)]"
>
	<button
		type="button"
		class="fixed inset-0 z-40 bg-black/60 transition-opacity duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] min-[961px]:hidden {mobileSidebarOpen
			? 'opacity-100 pointer-events-auto'
			: 'opacity-0 pointer-events-none'}"
		aria-label="Dismiss navigation"
		onclick={closeSidebar}
	></button>
	<aside
		class="fixed inset-y-0 left-0 z-50 flex w-66 flex-col gap-5 overflow-y-auto overscroll-contain border-r border-brand-border bg-brand-surface-card p-4 transition-transform duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] min-[961px]:sticky min-[961px]:top-0 min-[961px]:z-30 min-[961px]:h-dvh min-[961px]:w-auto min-[961px]:translate-x-0 {mobileSidebarOpen
			? 'translate-x-0'
			: '-translate-x-full'}"
		aria-label="Control Plane navigation"
	>
		<div class="sidebar-brand">
			<a
				class="brand-mark rounded focus-visible:outline-2 focus-visible:outline-brand-lime"
				href={workspaceHref(controlStore.activeWorkspace?.id)}
				aria-label="Signet workspace applications"
			>
				<SignetMark size={20} class="brand-icon" />
				<span>Signet</span>
			</a>
			<button
				type="button"
				bind:this={mobileCloseButton}
				class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold min-[961px]:hidden"
				onclick={closeSidebar}
				aria-label="Close sidebar"
			>
				Close
			</button>
		</div>

		<div class="flex flex-col gap-3 border-b border-brand-border pb-4">
			<div class="grid min-w-0 gap-1.5">
				<span class="text-xs font-medium text-brand-text-secondary">
					Workspace
				</span>
				<Dropdown
					label="Workspace selector"
					value={controlStore.activeWorkspace?.id ?? ''}
					options={controlStore.workspaces.map((ws) => ({
						value: ws.id,
						label: ws.name
					}))}
					placeholder="No workspaces"
					onchange={(workspaceId) => {
						void goto(workspaceHref(workspaceId));
					}}
					class="w-full"
				/>
			</div>

			<div class="flex min-w-0 items-end gap-2">
				<div class="grid min-w-0 flex-1 gap-1.5">
					<span class="text-xs font-medium text-brand-text-secondary">
						Application
					</span>
					<Dropdown
						label="Application selector"
						value={controlStore.activeApplication?.application.id ?? ''}
						options={controlStore.applications.map(({ application }) => ({
							value: application.id,
							label: application.name
						}))}
						placeholder="No applications"
						onchange={(nextAppId) => {
							void goto(
								workspaceHref(
									controlStore.activeWorkspace?.id,
									'applications',
									nextAppId,
									activeSection
								)
							);
							if (mobileSidebarOpen) closeSidebar();
						}}
						class="w-full"
					/>
				</div>
				{#if allowMutate}
					<button
						type="button"
						class="inline-grid size-9 shrink-0 place-items-center rounded-lg text-brand-text-secondary transition-colors hover:text-white hover:bg-brand-surface-raised focus-visible:outline-2 focus-visible:outline-[#296FF0] cursor-pointer bg-transparent border-0 p-0 text-xl font-light leading-none"
						title="Create application"
						aria-label="Create application"
						onclick={onCreateApplication}
					>
						<span aria-hidden="true">+</span>
					</button>
				{/if}
			</div>

			<div class="env-switch" role="group" aria-label="Environment switcher">
				<button
					type="button"
					class:active-dev={controlStore.selectedEnvKind === 'development'}
					aria-pressed={controlStore.selectedEnvKind === 'development'}
					onclick={() => selectEnvironmentKind('development')}
				>
					Development
				</button>
				<button
					type="button"
					class:active-prod={controlStore.selectedEnvKind === 'production'}
					aria-pressed={controlStore.selectedEnvKind === 'production'}
					onclick={() => selectEnvironmentKind('production')}
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
					class:active={activeSection === 'overview'}
					onclick={() => selectNav('overview')}
				>
					<svg
						class="size-4 shrink-0"
						viewBox="0 0 256 256"
						fill="currentColor"
						aria-hidden="true"
					>
						<path d="M104 48H56a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16V64a16 16 0 0 0-16-16zm96 0h-48a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16V64a16 16 0 0 0-16-16zm-96 96H56a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16v-48a16 16 0 0 0-16-16zm96 0h-48a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16v-48a16 16 0 0 0-16-16z" />
					</svg>
					Overview
				</button>
				<button
					type="button"
					class="nav-item"
					class:active={activeSection === 'analytics'}
					onclick={() => selectNav('analytics')}
				>
					<svg
						class="size-4 shrink-0"
						viewBox="0 0 256 256"
						fill="currentColor"
						aria-hidden="true"
					>
						<path d="M224 200h-8V40a16 16 0 0 0-16-16h-32a16 16 0 0 0-16 16v160h-16V88a16 16 0 0 0-16-16h-32a16 16 0 0 0-16 16v112H56V128a16 16 0 0 0-16-16H24a8 8 0 0 0 0 16h8v72H24a8 8 0 0 0 0 16h200a8 8 0 0 0 0-16z" />
					</svg>
					Analytics
				</button>
				<button
					type="button"
					class="nav-item"
					class:active={activeSection === 'auth'}
					onclick={() => selectNav('auth')}
				>
					<svg
						class="size-4 shrink-0"
						viewBox="0 0 256 256"
						fill="currentColor"
						aria-hidden="true"
					>
						<path d="M208 80h-32V56a48 48 0 0 0-96 0v24H48a16 16 0 0 0-16 16v112a16 16 0 0 0 16 16h160a16 16 0 0 0 16-16V96a16 16 0 0 0-16-16zm-80 84a12 12 0 1 1 12-12 12 12 0 0 1-12 12zm32-84H96V56a32 32 0 0 1 64 0z" />
					</svg>
					Auth &amp; SSO
				</button>
				<button
					type="button"
					class="nav-item"
					class:active={activeSection === 'keys'}
					onclick={() => selectNav('keys')}
				>
					<svg
						class="size-4 shrink-0"
						viewBox="0 0 256 256"
						fill="currentColor"
						aria-hidden="true"
					>
						<path d="M208 80a64 64 0 0 0-104.5 49.38l-65.15 65.15A16 16 0 0 0 33.72 206l16 16A16 16 0 0 0 61 226.34l19.31-19.31a4 4 0 0 0 0-5.66l-7.31-7.31a4 4 0 0 1 0-5.66l18.34-18.34a4 4 0 0 1 5.66 0l7.31 7.31a4 4 0 0 0 5.66 0l16.41-16.41A64 64 0 1 0 208 80zm-40 40a16 16 0 1 1 16-16 16 16 0 0 1-16 16z" />
					</svg>
					Keys &amp; Domains
				</button>
			</div>

			<div class="nav-group">
				<span class="nav-label">Directory</span>
				<button
					type="button"
					class="nav-item"
					class:active={activeSection === 'users'}
					onclick={() => selectNav('users')}
				>
					<svg
						class="size-4 shrink-0"
						viewBox="0 0 256 256"
						fill="currentColor"
						aria-hidden="true"
					>
						<path d="M168 112a40 40 0 1 0-40-40 40 40 0 0 0 40 40zm48 16h-11.8a55.8 55.8 0 0 1-72.4 0H120a48 48 0 0 0-48 48v8a8 8 0 0 0 8 8h176a8 8 0 0 0 8-8v-8a48 48 0 0 0-48-48zm-128-16a40 40 0 1 0-40-40 40 40 0 0 0 40 40zm24 64v-8a48 48 0 0 0-18.47-37.74 55.8 55.8 0 0 1-21.53 4.24H60a48 48 0 0 0-48 48v8a8 8 0 0 0 8 8h84a8 8 0 0 0 8-8z" />
					</svg>
					Users
					{#if overview && overview.total_users > 0}
						<span class="nav-count">{overview.total_users}</span>
					{/if}
				</button>
				<button
					type="button"
					class="nav-item"
					class:active={activeSection === 'organizations'}
					onclick={() => selectNav('organizations')}
				>
					<svg
						class="size-4 shrink-0"
						viewBox="0 0 256 256"
						fill="currentColor"
						aria-hidden="true"
					>
						<path d="M200 152h-24a16 16 0 0 0-16 16v8h-24v-40h16a16 16 0 0 0 16-16V64a16 16 0 0 0-16-16h-48a16 16 0 0 0-16 16v56a16 16 0 0 0 16 16h16v40H96v-8a16 16 0 0 0-16-16H56a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16v-8h88v8a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16v-48a16 16 0 0 0-16-16zm-96-88h48v56h-48zm-48 144H56v-48h48zm144 0h-48v-48h48z" />
					</svg>
					Organizations
				</button>
				<button
					type="button"
					class="nav-item"
					class:active={activeSection === 'webhooks'}
					onclick={() => selectNav('webhooks')}
				>
					<svg
						class="size-4 shrink-0"
						viewBox="0 0 256 256"
						fill="currentColor"
						aria-hidden="true"
					>
						<path d="M215.79 108.92A8 8 0 0 0 208 104h-64V24a8 8 0 0 0-13.66-5.66l-96 96A8 8 0 0 0 40 128h64v80a8 8 0 0 0 13.66 5.66l96-96a8 8 0 0 0 2.13-8.74z" />
					</svg>
					Webhooks &amp; Sync
				</button>
			</div>

			<div class="nav-group">
				<span class="nav-label">Security</span>
				<button
					type="button"
					class="nav-item"
					class:active={activeSection === 'roles'}
					onclick={() => selectNav('roles')}
				>
					<svg
						class="size-4 shrink-0"
						viewBox="0 0 256 256"
						fill="currentColor"
						aria-hidden="true"
					>
						<path d="M208 40H48a16 16 0 0 0-16 16v56c0 68.66 43.15 111.41 92.42 126.78a12 12 0 0 0 7.16 0C180.85 223.41 224 180.66 224 112V56a16 16 0 0 0-16-16zm-36.69 77.66l-56 56a8 8 0 0 1-11.32 0l-24-24a8 8 0 0 1 11.32-11.32L110 156.69l50.34-50.35a8 8 0 0 1 11.32 11.32z" />
					</svg>
					Roles &amp; Permissions
				</button>
				<button
					type="button"
					class="nav-item"
					class:active={activeSection === 'audit'}
					onclick={() => selectNav('audit')}
				>
					<svg
						class="size-4 shrink-0"
						viewBox="0 0 256 256"
						fill="currentColor"
						aria-hidden="true"
					>
						<path d="M200 32h-36.26a47.92 47.92 0 0 0-71.48 0H56a16 16 0 0 0-16 16v160a16 16 0 0 0 16 16h144a16 16 0 0 0 16-16V48a16 16 0 0 0-16-16zm-72 0a32 32 0 0 1 32 32H96a32 32 0 0 1 32-32zm32 128H96a8 8 0 0 1 0-16h64a8 8 0 0 1 0 16zm0-32H96a8 8 0 0 1 0-16h64a8 8 0 0 1 0 16z" />
					</svg>
					Audit Logs
				</button>
				<button
					type="button"
					class="nav-item"
					class:active={activeSection === 'security'}
					onclick={() => selectNav('security')}
				>
					<svg
						class="size-4 shrink-0"
						viewBox="0 0 256 256"
						fill="currentColor"
						aria-hidden="true"
					>
						<path d="M208 40H48a16 16 0 0 0-16 16v56c0 68.66 43.15 111.41 92.42 126.78a12 12 0 0 0 7.16 0C180.85 223.41 224 180.66 224 112V56a16 16 0 0 0-16-16z" />
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
							controlStore.developer.email
						)}
					</span>
					<div class="dev-meta">
						<strong>
							{controlStore.developer.display_name || 'Developer'}
						</strong>
						<span>{controlStore.developer.email}</span>
					</div>
				</div>
				<button type="button" class="danger-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold" onclick={onSignOut}>
					Sign out
				</button>
			{/if}
		</div>
	</aside>

	<div class="min-w-0">
		<header
			class="sticky top-0 z-30 border-b border-brand-border bg-brand-surface/95 backdrop-blur"
		>
			<div
				class="flex min-h-14 w-full items-center justify-between gap-3 px-4 sm:px-6 lg:px-8"
			>
				<div class="flex min-w-0 items-center gap-3">
					<button
						bind:this={mobileMenuButton}
						type="button"
						class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold min-[961px]:hidden"
						onclick={openSidebar}
						aria-label="Open navigation menu"
						aria-expanded={mobileSidebarOpen}
					>
						Menu
					</button>
					<nav
						class="flex min-w-0 items-center gap-2 text-sm"
						aria-label="Current section"
					>
						<a
							class="truncate rounded text-brand-text-secondary hover:text-brand-text-primary focus-visible:outline-2 focus-visible:outline-brand-lime"
							href={workspaceHref(controlStore.activeWorkspace?.id)}
						>
							{controlStore.activeWorkspace?.name ?? 'Workspace'}
						</a>
						<span class="text-brand-text-muted" aria-hidden="true">/</span>
						{#if controlStore.activeApplication}
							<span class="hidden truncate text-brand-text-secondary md:inline">
								{controlStore.activeApplication.application.name}
							</span>
							<span
								class="hidden text-brand-text-muted md:inline"
								aria-hidden="true"
							>
								/
							</span>
						{/if}
						<span
							class="min-w-0 truncate font-medium text-brand-text-primary"
							aria-current="page"
						>
							{sectionTitles[activeSection]}
						</span>
					</nav>
				</div>

				{#if controlStore.activeApplication}
					<div
						class="flex shrink-0 items-center gap-2 text-xs text-brand-text-secondary"
					>
						<span
							class="size-1.5 rounded-full {controlStore.selectedEnvKind ===
							'production'
								? 'bg-brand-success'
								: 'bg-brand-warning'}"
							aria-hidden="true"
						></span>
						{controlStore.selectedEnvKind === 'production'
							? 'Production'
							: 'Development'}
					</div>
				{/if}
			</div>
		</header>

		<main
			class="grid w-full min-w-0 grid-cols-1 gap-5 px-4 py-5 sm:gap-6 sm:px-6 sm:py-7 lg:px-8"
		>
			{#if !allowMutate}
				<div class="readonly-banner" role="status">
					Read-only access: your workspace role is <strong>
						{activeRole}
					</strong>. Contact a workspace admin to perform mutations.
				</div>
			{/if}

			{#key activeSection}
				<div class="panel-transition grid w-full min-w-0 grid-cols-1 gap-5 sm:gap-6">
					{#if !controlStore.activeApplication || !controlStore.activeEnvironment}
						<SectionHeader
							title={sectionTitles[activeSection]}
							description="This application has no available environments."
						/>
					{:else if activeSection === 'overview'}
						<OverviewPanel
							{overview}
							loading={loadingEnv}
							appName={controlStore.activeApplication.application.name}
							appSlug={controlStore.activeApplication.application.slug}
							environmentKind={controlStore.selectedEnvKind}
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
							onAppDeleted={handleAppDeleted}
						/>
					{:else if activeSection === 'audit'}
						<AuditLogsPanel
							appId={controlStore.activeApplication.application.id}
							envKind={controlStore.selectedEnvKind}
						/>
					{:else if activeSection === 'security'}
						<AccountSecurityPanel />
					{/if}
				</div>
			{/key}
		</main>
	</div>
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
		transition: all 200ms cubic-bezier(0.16, 1, 0.3, 1);
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
		transition: all 200ms cubic-bezier(0.16, 1, 0.3, 1);
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



	.readonly-banner {
		padding: 0.7rem 1rem;
		background-color: #202020;
		border: 1px solid #3d3d3d;
		color: #c9c9c9;
		border-radius: 8px;
		font-size: 0.82rem;
	}
</style>
