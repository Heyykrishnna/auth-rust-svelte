<script lang="ts">
	import { goto } from '$app/navigation';
	import type {
		ApplicationWithEnvironmentsDto,
		WorkspaceSummary
	} from '$lib/api/types';
	import Button from '$lib/components/Button.svelte';
	import Dropdown from '$lib/components/Dropdown.svelte';
	import SignetMark from '$lib/components/SignetMark.svelte';
	import { workspaceHref, type WorkspaceView } from '$lib/control/navigation';
	import { getInitials } from '$lib/control/permissions';
	import { controlStore } from '$lib/stores/control.svelte';
	import AccountSecurityPanel from './AccountSecurityPanel.svelte';
	import WorkspaceTeam from './WorkspaceTeam.svelte';
	import WorkspaceApplications from './WorkspaceApplications.svelte';
	import WorkspaceAvatar from './WorkspaceAvatar.svelte';
	import WorkspaceSettings from './WorkspaceSettings.svelte';

	let {
		workspace,
		view,
		apps,
		loadingApplications,
		applicationsError,
		missingWorkspace,
		unavailableApp,
		onRetryApplications,
		onCreateWorkspace,
		onCreateApplication,
		onWorkspaceUpdated,
		onSignOut,
		signingOut
	}: {
		workspace?: WorkspaceSummary;
		view: WorkspaceView;
		apps: ApplicationWithEnvironmentsDto[];
		loadingApplications: boolean;
		applicationsError: string;
		missingWorkspace: boolean;
		unavailableApp: boolean;
		onRetryApplications: () => void;
		onCreateWorkspace: () => void;
		onCreateApplication: () => void;
		onWorkspaceUpdated: (workspace: WorkspaceSummary) => void;
		onSignOut: () => void;
		signingOut: boolean;
	} = $props();

	const links = [
		{
			view: 'applications',
			label: 'Applications',
			icon: 'M104 48H56a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16V64a16 16 0 0 0-16-16zm96 0h-48a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16V64a16 16 0 0 0-16-16zm-96 96H56a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16v-48a16 16 0 0 0-16-16zm96 0h-48a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16v-48a16 16 0 0 0-16-16z'
		},
		{
			view: 'team',
			label: 'Team',
			icon: 'M168 112a40 40 0 1 0-40-40 40 40 0 0 0 40 40zm48 16h-11.8a55.8 55.8 0 0 1-72.4 0H120a48 48 0 0 0-48 48v8a8 8 0 0 0 8 8h176a8 8 0 0 0 8-8v-8a48 48 0 0 0-48-48zm-128-16a40 40 0 1 0-40-40 40 40 0 0 0 40 40zm24 64v-8a48 48 0 0 0-18.47-37.74 55.8 55.8 0 0 1-21.53 4.24H60a48 48 0 0 0-48 48v8a8 8 0 0 0 8 8h84a8 8 0 0 0 8-8z'
		},
		{
			view: 'settings',
			label: 'Settings',
			icon: 'M224 80h-40V64a8 8 0 0 0-16 0v48a8 8 0 0 0 16 0V96h40a8 8 0 0 0 0-16zm-80 8H32a8 8 0 0 0 0 16h112a8 8 0 0 0 0-16zm80 80h-88a8 8 0 0 0 0 16h88a8 8 0 0 0 0-16zm-112-24a8 8 0 0 0-8 8v16H32a8 8 0 0 0 0 16h72v16a8 8 0 0 0 16 0v-48a8 8 0 0 0-8-8z'
		},
		{
			view: 'security',
			label: 'Account Security',
			icon: 'M208 40H48a16 16 0 0 0-16 16v56c0 68.66 43.15 111.41 92.42 126.78a12 12 0 0 0 7.16 0C180.85 223.41 224 180.66 224 112V56a16 16 0 0 0-16-16z'
		}
	] as const;
	const titles: Record<WorkspaceView, string> = {
		applications: 'Applications',
		team: 'Team',
		settings: 'Settings',
		security: 'Account security'
	};
</script>

<svelte:head>
	<title>{titles[view]} · {workspace?.name ?? 'Workspace'} · Signet</title>
</svelte:head>

<div
	class="workspace-theme grid min-h-dvh grid-cols-[minmax(0,1fr)] bg-brand-surface text-brand-text-primary lg:grid-cols-[14rem_minmax(0,1fr)] xl:grid-cols-[16rem_minmax(0,1fr)]"
>
	<aside
		class="flex min-w-0 flex-col gap-5 border-b border-brand-border bg-brand-surface-card p-4 lg:sticky lg:top-0 lg:h-dvh lg:border-r lg:border-b-0"
	>
		<div class="flex min-w-0 items-center justify-between gap-3 px-[0.35rem]">
			<a
				href={workspaceHref(workspace?.id)}
				class="inline-flex items-center gap-[0.55rem] rounded text-white font-semibold text-[0.95rem] no-underline focus-visible:outline-2 focus-visible:outline-brand-lime"
				aria-label="Signet workspace applications"
			>
				<SignetMark size={20} class="text-white shrink-0" />
				<span>Signet</span>
			</a>
			{#if controlStore.developer}
				<div class="lg:hidden">
					<Button
						variant="danger"
						size="sm"
						disabled={signingOut}
						onclick={onSignOut}
					>
						Sign out
					</Button>
				</div>
			{/if}
		</div>

		<div class="grid min-w-0 gap-1.5">
			<span class="text-xs font-medium text-brand-text-secondary">
				Workspace
			</span>
			<div class="flex min-w-0 items-center gap-2">
				<Dropdown
					label="Workspace selector"
					options={controlStore.workspaces.map(({ id, name }) => ({
						value: id,
						label: name
					}))}
					value={workspace?.id ?? ''}
					placeholder="Select workspace"
					class="min-w-0 flex-1"
					onchange={(id) => {
						void goto(workspaceHref(id, view));
					}}
				/>
				<button
					type="button"
					class="inline-grid size-9 shrink-0 place-items-center rounded-lg text-brand-text-secondary transition-colors hover:text-white hover:bg-brand-surface-raised focus-visible:outline-2 focus-visible:outline-action-focus cursor-pointer bg-transparent border-0 p-0 text-xl font-light leading-none"
					aria-label="Create workspace"
					title="Create workspace"
					onclick={onCreateWorkspace}
				>
					<span class="text-xl font-light leading-none" aria-hidden="true">+</span>
				</button>
			</div>
		</div>

		{#if workspace}
			<nav
				class="flex min-w-0 flex-wrap gap-1 border-t border-brand-border py-2 lg:grid lg:py-4"
				aria-label="Workspace navigation"
			>
				{#each links as link (link.view)}
					<a
						href={workspaceHref(workspace.id, link.view)}
						aria-current={view === link.view ? 'page' : undefined}
						class="inline-flex min-w-0 items-center gap-2 rounded-lg border border-transparent px-2.5 py-2 text-sm font-medium text-brand-text-secondary no-underline transition-all duration-200 ease-[cubic-bezier(0.16,1,0.3,1)] hover:bg-brand-surface-raised hover:text-brand-text-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-lime aria-[current=page]:border-brand-border-hover aria-[current=page]:bg-brand-surface-alt aria-[current=page]:text-white sm:gap-2.5 sm:px-3"
					>
						<svg
							class="size-4 shrink-0"
							viewBox="0 0 256 256"
							fill="currentColor"
							aria-hidden="true"
						>
							<path d={link.icon} />
						</svg>
						{link.label}
					</a>
				{/each}
			</nav>
		{/if}

		{#if controlStore.developer}
			<div
				class="mt-auto hidden min-w-0 gap-3 border-t border-brand-border pt-4 lg:grid"
			>
				<a
					href={workspaceHref(workspace?.id, 'security')}
					aria-current={view === 'security' ? 'page' : undefined}
					class="flex min-w-0 items-center gap-2.5 rounded-md text-brand-text-primary no-underline hover:text-brand-lime focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-lime"
				>
					<span
						class="grid size-8 shrink-0 place-items-center rounded-full bg-brand-surface-alt text-xs font-semibold"
						aria-hidden="true"
					>
						{getInitials(
							controlStore.developer.display_name,
							controlStore.developer.email
						)}
					</span>
					<span class="grid min-w-0">
						<span class="truncate text-sm font-medium">
							{controlStore.developer.display_name || 'Developer account'}
						</span>
						<span class="text-xs text-brand-text-secondary">
							Account security
						</span>
					</span>
				</a>
				<Button
					variant="danger"
					size="sm"
					disabled={signingOut}
					onclick={onSignOut}
					full
				>
					Sign out
				</Button>
			</div>
		{/if}
	</aside>

	<div class="min-w-0">
		<header class="border-b border-brand-border">
			<div
				class="flex min-h-14 w-full items-center justify-between gap-3 px-4 text-sm sm:px-6 lg:px-8"
			>
				<div class="flex min-w-0 items-center gap-2.5">
					{#if workspace}
						<WorkspaceAvatar
							{workspace}
							class="size-7 rounded-md text-[10px]"
						/>
					{/if}
					<span class="min-w-0 truncate font-medium">
						{workspace?.name ?? 'Developer Console'}
					</span>
				</div>
				{#if workspace}
					<span class="shrink-0 text-xs text-brand-text-secondary capitalize">
						{workspace.role}
					</span>
				{/if}
				{#if controlStore.developer}
					<a
						href={workspaceHref(workspace?.id, 'security')}
						class="shrink-0 rounded text-xs text-brand-text-secondary no-underline hover:text-brand-lime focus-visible:outline-2 focus-visible:outline-brand-lime lg:hidden"
					>
						Account security
					</a>
				{/if}
			</div>
		</header>

		<main
			class="grid w-full gap-5 px-4 py-6 sm:gap-6 sm:px-6 sm:py-9 lg:px-8"
		>
			{#key view}
				<div class="panel-transition grid w-full gap-5 sm:gap-6">
					{#if view === 'security'}
						<AccountSecurityPanel />
					{:else if workspace && controlStore.developer}
						{#key workspace.id}
							{#if view === 'team'}
								<WorkspaceTeam
									{workspace}
									currentDeveloperId={controlStore.developer.id}
									{onWorkspaceUpdated}
								/>
							{:else if view === 'settings'}
								<WorkspaceSettings {workspace} onUpdated={onWorkspaceUpdated} />
							{:else}
								<WorkspaceApplications
									{workspace}
									{apps}
									loading={loadingApplications}
									error={applicationsError}
									{unavailableApp}
									onRetry={onRetryApplications}
									onCreate={onCreateApplication}
								/>
							{/if}
						{/key}
					{:else}
						<div
							class="grid justify-items-start gap-3 task-surface p-6 sm:p-8"
						>
							<h1 class="font-heading text-2xl font-bold">
								{missingWorkspace ? 'Workspace unavailable' : 'Create a workspace'}
							</h1>
							<p class="text-sm text-brand-text-secondary">
								{missingWorkspace
									? 'Select a workspace you have access to.'
									: 'A workspace brings your applications and team together.'}
							</p>
							{#if missingWorkspace && controlStore.workspaces.length > 0}
								<Button href="/control">
									Go to my workspace
								</Button>
							{:else}
								<Button onclick={onCreateWorkspace}>Create workspace</Button>
							{/if}
						</div>
					{/if}
				</div>
			{/key}
		</main>
	</div>
</div>
