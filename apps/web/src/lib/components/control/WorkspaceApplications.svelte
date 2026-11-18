<script lang="ts">
	import type {
		ApplicationWithEnvironmentsDto,
		WorkspaceSummary
	} from '$lib/api/types';
	import Button from '$lib/components/Button.svelte';
	import Dropdown from '$lib/components/Dropdown.svelte';
	import { workspaceHref } from '$lib/control/navigation';
	import { canMutateEnvironment } from '$lib/control/permissions';
	import SectionHeader from './SectionHeader.svelte';
	import WorkspaceSearch from './WorkspaceSearch.svelte';

	let {
		workspace,
		apps,
		loading,
		error,
		unavailableApp,
		onRetry,
		onCreate
	}: {
		workspace: WorkspaceSummary;
		apps: ApplicationWithEnvironmentsDto[];
		loading: boolean;
		error: string;
		unavailableApp: boolean;
		onRetry: () => void;
		onCreate: () => void;
	} = $props();

	const sortOptions = [
		{ value: 'updated', label: 'Recently updated' },
		{ value: 'name', label: 'Name A–Z' }
	] as const;
	let search = $state('');
	let sort = $state<'updated' | 'name'>('updated');
	const canCreate = $derived(canMutateEnvironment(workspace.role));
	const visibleApps = $derived.by(() => {
		const query = search.trim().toLowerCase();
		return apps
			.filter(
				({ application: { name, slug } }) =>
					name.toLowerCase().includes(query) || slug.includes(query)
			)
			.sort(({ application: first }, { application: second }) =>
				sort === 'name'
					? first.name.localeCompare(second.name)
					: Date.parse(second.updated_at) - Date.parse(first.updated_at)
			);
	});
	const dateFormat = new Intl.DateTimeFormat(undefined, {
		dateStyle: 'medium'
	});
</script>

{#snippet appIcon()}
	<svg
		class="size-5"
		viewBox="0 0 24 24"
		fill="none"
		stroke="currentColor"
		stroke-width="1.5"
		stroke-linecap="round"
		stroke-linejoin="round"
		aria-hidden="true"
	>
		<path
			d="M3.75 8.5 12 4l8.25 4.5L12 13 3.75 8.5ZM3.75 12.75 12 17.25l8.25-4.5M3.75 17 12 21.5l8.25-4.5"
		/>
	</svg>
{/snippet}

<SectionHeader
	title="Applications"
	description="Open an application to manage its authentication and environments."
>
	{#if canCreate}
		<Button onclick={onCreate}>
			<span class="text-lg leading-none" aria-hidden="true">+</span> Create application
		</Button>
	{/if}
</SectionHeader>

{#if unavailableApp}
	<p
		role="status"
		class="rounded-lg border border-brand-border bg-brand-surface-card px-4 py-3 text-sm text-brand-text-secondary"
	>
		The requested application is unavailable in this workspace.
	</p>
{/if}

{#if error}
	<div
		class="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-brand-danger-border bg-brand-danger-soft p-4"
	>
		<p role="alert" class="text-sm text-brand-danger-text">{error}</p>
		<Button size="sm" variant="outline" onclick={onRetry}>Retry</Button>
	</div>
{/if}

{#if loading && apps.length === 0}
	<div class="grid gap-3 md:grid-cols-2 2xl:grid-cols-3" role="status">
		<span class="sr-only">Loading applications…</span>
		{#each [1, 2, 3] as placeholder (placeholder)}
			<div
				class="h-48 rounded-xl border border-brand-border bg-brand-surface-card motion-safe:animate-pulse"
				aria-hidden="true"
			></div>
		{/each}
	</div>
{:else if apps.length > 0}
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="min-w-0 flex-1 sm:max-w-sm">
			<WorkspaceSearch label="Search applications" bind:value={search} />
		</div>
		<Dropdown
			label="Sort applications"
			bind:value={sort}
			options={sortOptions}
			class="min-w-40"
		/>
	</div>

	{#if visibleApps.length > 0}
		<ul class="m-0 grid list-none gap-3 p-0 md:grid-cols-2 2xl:grid-cols-3">
			{#each visibleApps as { application, environments } (application.id)}
				<li class="min-w-0">
					<a
						href={workspaceHref(workspace.id, 'applications', application.id)}
						class="group flex h-full min-w-0 flex-col rounded-xl border border-brand-border bg-brand-surface-card p-5 text-brand-text-primary no-underline transition-colors hover:border-brand-border-strong hover:bg-brand-surface-raised focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-lime"
					>
						<div class="flex items-start justify-between gap-3">
							<span
								class="grid size-10 shrink-0 place-items-center rounded-lg border border-brand-success-border bg-brand-success-soft text-brand-lime"
							>
								{@render appIcon()}
							</span>
							<svg
								class="size-4 shrink-0 text-brand-text-muted transition-colors group-hover:text-brand-lime"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="1.6"
								aria-hidden="true"
							>
								<path
									d="M6 18 18 6M7 6h11v11"
									stroke-linecap="round"
									stroke-linejoin="round"
								/>
							</svg>
						</div>
						<h2
							class="mt-4 truncate font-heading text-lg font-bold"
							title={application.name}
						>
							{application.name}
						</h2>
						<p
							class="mt-0.5 truncate font-mono text-xs text-brand-text-secondary"
						>
							{application.slug}
						</p>
						<div
							class="mt-4 flex flex-wrap items-center gap-x-2 gap-y-1 border-t border-brand-border pt-3 text-xs text-brand-text-secondary"
						>
							{#if environments.length > 0}
								{#each environments as environment (environment.id)}
									<span class="capitalize">{environment.kind}</span>
								{/each}
							{:else}
								<span>No environments</span>
							{/if}
							{#if application.status !== 'active'}
								<span class="text-brand-warning-text capitalize">
									{application.status}
								</span>
							{/if}
							<time class="ml-auto" datetime={application.updated_at}>
								{dateFormat.format(new Date(application.updated_at))}
							</time>
						</div>
					</a>
				</li>
			{/each}
		</ul>
	{:else}
		<div
			class="grid justify-items-center gap-3 rounded-xl border border-dashed border-brand-border-hover p-9 text-center"
			role="status"
		>
			<p class="text-sm text-brand-text-secondary">
				No applications match your search.
			</p>
			<Button size="sm" variant="outline" onclick={() => (search = '')}>
				Clear search
			</Button>
		</div>
	{/if}
{:else if !error}
	<div
		class="grid justify-items-center gap-3 rounded-xl border border-dashed border-brand-border-hover bg-brand-surface-card px-5 py-16 text-center"
	>
		<span
			class="grid size-11 place-items-center rounded-lg border border-brand-success-border bg-brand-success-soft text-brand-lime"
		>
			{@render appIcon()}
		</span>
		<h2 class="font-heading text-xl font-bold text-brand-text-primary">
			No applications yet
		</h2>
		<p class="max-w-sm text-sm leading-relaxed text-brand-text-secondary">
			{canCreate
				? 'Create an application to get separate development and production environments.'
				: 'Applications will appear here when a workspace developer creates one.'}
		</p>
		{#if canCreate}
			<Button variant="outline" onclick={onCreate}>
				Create your first application
			</Button>
		{/if}
	</div>
{/if}
