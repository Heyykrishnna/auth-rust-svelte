import type {
	ApplicationEnvironmentDto,
	ApplicationWithEnvironmentsDto,
	DeveloperProfile,
	EnvironmentKind,
	WorkspaceSummary
} from '$lib/api/types';

export interface OneTimeSecretReveal {
	title: string;
	secretKey: string;
	publishableKey?: string;
}

export interface ControlPlaneState {
	developer: DeveloperProfile | null;
	workspaces: WorkspaceSummary[];
	applications: ApplicationWithEnvironmentsDto[];
	selectedWorkspaceId: string | null;
	selectedAppId: string | null;
	selectedEnvKind: EnvironmentKind;
	revealedSecret: OneTimeSecretReveal | null;
}

export function createInitialControlState(): ControlPlaneState {
	return {
		developer: null,
		workspaces: [],
		applications: [],
		selectedWorkspaceId: null,
		selectedAppId: null,
		selectedEnvKind: 'development',
		revealedSecret: null
	};
}

export function getActiveWorkspace(state: ControlPlaneState): WorkspaceSummary | null {
	return state.workspaces.find((w) => w.id === state.selectedWorkspaceId) ?? state.workspaces[0] ?? null;
}

export function getActiveApplication(
	state: ControlPlaneState
): ApplicationWithEnvironmentsDto | null {
	return (
		state.applications.find((a) => a.application.id === state.selectedAppId) ??
		state.applications[0] ??
		null
	);
}

export function getActiveEnvironment(state: ControlPlaneState): ApplicationEnvironmentDto | null {
	const app = getActiveApplication(state);
	if (!app) return null;
	return app.environments.find((e) => e.kind === state.selectedEnvKind) ?? app.environments[0] ?? null;
}

export function applyDeveloper(
	state: ControlPlaneState,
	developer: DeveloperProfile | null
): ControlPlaneState {
	if (!developer) {
		return createInitialControlState();
	}
	return { ...state, developer };
}

export function applyWorkspaces(
	state: ControlPlaneState,
	workspaces: WorkspaceSummary[]
): ControlPlaneState {
	const selectedWorkspaceId =
		state.selectedWorkspaceId && workspaces.some((w) => w.id === state.selectedWorkspaceId)
			? state.selectedWorkspaceId
			: (workspaces[0]?.id ?? null);
	return { ...state, workspaces, selectedWorkspaceId };
}

export function applyApplications(
	state: ControlPlaneState,
	applications: ApplicationWithEnvironmentsDto[]
): ControlPlaneState {
	const selectedAppId =
		state.selectedAppId && applications.some((a) => a.application.id === state.selectedAppId)
			? state.selectedAppId
			: (applications[0]?.application.id ?? null);
	return { ...state, applications, selectedAppId };
}

export function applyWorkspaceSelection(
	state: ControlPlaneState,
	workspaceId: string
): ControlPlaneState {
	return { ...state, selectedWorkspaceId: workspaceId, selectedAppId: null };
}

export function applyApplicationSelection(
	state: ControlPlaneState,
	appId: string
): ControlPlaneState {
	return { ...state, selectedAppId: appId };
}

export function applyEnvironmentSelection(
	state: ControlPlaneState,
	kind: EnvironmentKind
): ControlPlaneState {
	return { ...state, selectedEnvKind: kind };
}

export function applySecretReveal(
	state: ControlPlaneState,
	reveal: OneTimeSecretReveal | null
): ControlPlaneState {
	return { ...state, revealedSecret: reveal };
}

