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
			icon: 'M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z'
		},
		{
			view: 'team',
			label: 'Team',
			icon: 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2m20 0v-2a4 4 0 0 0-3-3.87M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm7-7.87a4 4 0 0 1 0 7.75'
		},
		{ view: 'settings', label: 'Settings', icon: 'M4 7h16M4 17h16M9 4v6m6 4v6' }
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
	class="workspace-theme workspace-accent grid min-h-dvh grid-cols-[minmax(0,1fr)] bg-brand-surface text-brand-text-primary lg:grid-cols-[14rem_minmax(0,1fr)] xl:grid-cols-[16rem_minmax(0,1fr)]"
>
	<aside
		class="flex min-w-0 flex-col gap-4 border-b border-brand-border bg-brand-surface-card px-4 pt-4 lg:sticky lg:top-0 lg:h-dvh lg:gap-6 lg:border-r lg:border-b-0 lg:p-5"
	>
		<div class="flex min-w-0 flex-wrap items-center justify-between gap-3">
			<a
				href={workspaceHref(workspace?.id)}
				class="inline-flex items-center gap-2.5 rounded-md text-brand-text-primary no-underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-lime"
				aria-label="Signet workspace applications"
			>
				<span
					class="grid size-8 place-items-center rounded-md bg-brand-lime text-brand-lime-ink"
				>
					<SignetMark size={18} />
				</span>
				<span class="font-heading text-lg font-extrabold tracking-tight">
					Signet
				</span>
			</a>
			{#if controlStore.developer}
				<div class="lg:hidden">
					<Button
						size="sm"
						variant="outline"
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
					class="grid size-9 shrink-0 place-items-center rounded-lg border border-brand-border-hover text-brand-text-secondary transition-colors hover:border-brand-border-strong hover:bg-brand-surface-raised hover:text-brand-lime focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-lime"
					aria-label="Create workspace"
					title="Create workspace"
					onclick={onCreateWorkspace}
				>
					<span class="text-lg leading-none" aria-hidden="true">+</span>
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
						class="inline-flex min-w-0 items-center gap-2 rounded-lg border border-transparent px-2.5 py-2 text-sm font-medium text-brand-text-secondary no-underline transition-colors hover:bg-brand-surface-raised hover:text-brand-text-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-lime aria-[current=page]:border-brand-success-border aria-[current=page]:bg-brand-success-soft aria-[current=page]:text-brand-lime sm:gap-2.5 sm:px-3"
					>
						<svg
							class="size-4 shrink-0"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="1.6"
							stroke-linecap="round"
							stroke-linejoin="round"
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
					size="sm"
					variant="outline"
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
				class="mx-auto flex min-h-14 w-full max-w-7xl items-center justify-between gap-3 px-4 text-sm sm:px-6 lg:px-9"
			>
				<span class="min-w-0 truncate font-medium">
					{workspace?.name ?? 'Developer Console'}
				</span>
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
			class="mx-auto grid w-full max-w-7xl gap-5 px-4 py-6 sm:gap-6 sm:px-6 sm:py-9 lg:px-9"
		>
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
					class="grid justify-items-start gap-3 rounded-xl border border-brand-border bg-brand-surface-card p-6 sm:p-8"
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
						<Button href="/control" variant="outline">
							Go to my workspace
						</Button>
					{:else}
						<Button onclick={onCreateWorkspace}>Create workspace</Button>
					{/if}
				</div>
			{/if}
		</main>
	</div>
</div>
