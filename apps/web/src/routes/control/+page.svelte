<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { untrack } from 'svelte';
	import { controlApi } from '$lib/api/control';
	import type {
		CreatedApplicationResponse,
		WorkspaceSummary
	} from '$lib/api/types';
	import { addToast } from '$lib/components/Toast.svelte';
	import ApplicationConsole from '$lib/components/control/ApplicationConsole.svelte';
	import CreateWorkspaceResource from '$lib/components/control/CreateWorkspaceResource.svelte';
	import WorkspaceHome from '$lib/components/control/WorkspaceHome.svelte';
	import { workspaceHref, type WorkspaceView } from '$lib/control/navigation';
	import { canMutateEnvironment } from '$lib/control/permissions';
	import {
		controlStore,
		selectApplication,
		selectEnvironmentKind,
		selectWorkspace,
		setApplications,
		setDeveloper,
		setWorkspaces,
		showOneTimeSecret
	} from '$lib/stores/control.svelte';

	let create = $state<'workspace' | 'application' | null>(null);
	let loadedWorkspace = $state<string>();
	let loadingApplications = $state(false);
	let applicationsError = $state('');
	let signingOut = $state(false);
	let loadRequest = 0;

	const requestedWorkspace = $derived(page.url.searchParams.get('workspace'));
	const requestedApp = $derived(page.url.searchParams.get('app'));
	const requestedSection = $derived(page.url.searchParams.get('section'));
	const requestedView = $derived(page.url.searchParams.get('view'));
	const workspace = $derived(
		requestedWorkspace
			? controlStore.workspaces.find(({ id }) => id === requestedWorkspace)
			: (controlStore.activeWorkspace ?? undefined)
	);
	const workspaceId = $derived(workspace?.id);
	const view: WorkspaceView = $derived(
		requestedView === 'team' || requestedSection === 'team'
			? 'team'
			: requestedView === 'security' || requestedSection === 'security'
				? 'security'
				: requestedView === 'settings'
					? 'settings'
					: 'applications'
	);
	const apps = $derived(
		workspaceId && loadedWorkspace === workspaceId
			? controlStore.applications
			: []
	);
	const app = $derived(
		view !== 'applications'
			? undefined
			: requestedApp
				? apps.find(({ application }) => application.id === requestedApp)
				: requestedSection
					? apps[0]
					: undefined
	);
	const appId = $derived(app?.application.id);
	const unavailableApp = $derived(
		Boolean(
			requestedApp &&
			!app &&
			workspaceId &&
			loadedWorkspace === workspaceId &&
			!applicationsError
		)
	);

	async function loadApplications(id: string) {
		const request = ++loadRequest;
		loadingApplications = true;
		applicationsError = '';
		try {
			const result = await controlApi.listApplications(id);
			if (request !== loadRequest || id !== workspaceId) return;
			setApplications(result);
			loadedWorkspace = id;
		} catch (err) {
			if (request !== loadRequest || id !== workspaceId) return;
			applicationsError =
				err instanceof Error ? err.message : 'Could not load applications.';
		} finally {
			if (request === loadRequest) loadingApplications = false;
		}
	}

	$effect(() => {
		const id = workspaceId;
		if (!id) return;
		untrack(() => {
			selectWorkspace(id);
			setApplications([]);
			loadedWorkspace = undefined;
			void loadApplications(id);
		});
		return () => {
			loadRequest += 1;
		};
	});

	$effect(() => {
		const id = appId;
		if (!id) return;
		untrack(() => {
			selectApplication(id);
			selectEnvironmentKind('development');
		});
	});

	function updateWorkspace(updated: WorkspaceSummary) {
		setWorkspaces(
			controlStore.workspaces.map((ws) => (ws.id === updated.id ? updated : ws))
		);
	}

	function handleWorkspaceCreated(created: WorkspaceSummary) {
		create = null;
		setWorkspaces([...controlStore.workspaces, created]);
		addToast('Workspace created', 'success');
		void goto(workspaceHref(created.id));
	}

	function handleApplicationCreated(created: CreatedApplicationResponse) {
		create = null;
		const id = created.application.workspace_id;
		if (workspaceId === id) {
			loadRequest += 1;
			loadingApplications = false;
			applicationsError = '';
			setApplications([
				...apps,
				{ application: created.application, environments: created.environments }
			]);
			loadedWorkspace = id;
		}
		showOneTimeSecret({
			title: 'Save your development keys',
			publishableKey: created.development_publishable_key,
			secretKey: created.initial_development_secret_key
		});
		addToast('Application created', 'success');
		void goto(workspaceHref(id, 'applications', created.application.id));
		void loadApplications(id);
	}

	async function signOut() {
		if (signingOut) return;
		signingOut = true;
		try {
			await controlApi.signOut();
			setDeveloper(null);
			await goto('/control/login');
		} catch (err) {
			addToast(
				err instanceof Error ? err.message : 'Could not sign out.',
				'error'
			);
		} finally {
			signingOut = false;
		}
	}
</script>

{#if appId && controlStore.selectedAppId === appId && controlStore.activeWorkspace?.id === workspaceId}
	{#key appId}
		<ApplicationConsole
			onCreateApplication={() => (create = 'application')}
			onSignOut={signOut}
			onRefreshApplications={async () => {
				if (workspaceId) await loadApplications(workspaceId);
			}}
		/>
	{/key}
{:else}
	<WorkspaceHome
		{workspace}
		{view}
		{apps}
		{loadingApplications}
		{applicationsError}
		missingWorkspace={Boolean(requestedWorkspace && !workspace)}
		{unavailableApp}
		onRetryApplications={() => {
			if (workspaceId) void loadApplications(workspaceId);
		}}
		onCreateWorkspace={() => (create = 'workspace')}
		onCreateApplication={() => (create = 'application')}
		onWorkspaceUpdated={updateWorkspace}
		onSignOut={signOut}
		{signingOut}
	/>
{/if}

{#if create === 'workspace' || (create === 'application' && workspace && canMutateEnvironment(workspace.role))}
	<div class="workspace-theme workspace-accent">
		<CreateWorkspaceResource
			workspaceId={create === 'application' ? workspaceId : undefined}
			onApplicationCreated={handleApplicationCreated}
			onWorkspaceCreated={handleWorkspaceCreated}
			onClose={() => (create = null)}
		/>
	</div>
{/if}
