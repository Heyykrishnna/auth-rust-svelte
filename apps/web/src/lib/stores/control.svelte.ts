import type {
	ApplicationWithEnvironmentsDto,
	DeveloperProfile,
	EnvironmentKind,
	WorkspaceSummary
} from '$lib/api/types';
import {
	applyApplicationSelection,
	applyApplications,
	applyDeveloper,
	applyEnvironmentSelection,
	applySecretReveal,
	applyWorkspaceSelection,
	applyWorkspaces,
	createInitialControlState,
	getActiveApplication,
	getActiveEnvironment,
	getActiveWorkspace,
	type ControlPlaneState,
	type OneTimeSecretReveal
} from '$lib/control/state';

export type { ControlPlaneState, OneTimeSecretReveal };

let _state = $state<ControlPlaneState>(createInitialControlState());

export const controlStore = {
	get developer() {
		return _state.developer;
	},
	get isAuthenticated() {
		return _state.developer !== null;
	},
	get workspaces() {
		return _state.workspaces;
	},
	get applications() {
		return _state.applications;
	},
	get selectedWorkspaceId() {
		return _state.selectedWorkspaceId;
	},
	get selectedAppId() {
		return _state.selectedAppId;
	},
	get selectedEnvKind() {
		return _state.selectedEnvKind;
	},
	get activeWorkspace() {
		return getActiveWorkspace(_state);
	},
	get activeApplication() {
		return getActiveApplication(_state);
	},
	get activeEnvironment() {
		return getActiveEnvironment(_state);
	},
	get revealedSecret() {
		return _state.revealedSecret;
	}
};

export function setDeveloper(developer: DeveloperProfile | null) {
	_state = applyDeveloper(_state, developer);
}

export function setWorkspaces(workspaces: WorkspaceSummary[]) {
	_state = applyWorkspaces(_state, workspaces);
}

export function setApplications(apps: ApplicationWithEnvironmentsDto[]) {
	_state = applyApplications(_state, apps);
}

export function selectWorkspace(workspaceId: string) {
	_state = applyWorkspaceSelection(_state, workspaceId);
}

export function selectApplication(appId: string) {
	_state = applyApplicationSelection(_state, appId);
}

export function selectEnvironmentKind(kind: EnvironmentKind) {
	_state = applyEnvironmentSelection(_state, kind);
}

export function showOneTimeSecret(reveal: OneTimeSecretReveal) {
	_state = applySecretReveal(_state, reveal);
}

export function dismissOneTimeSecret() {
	_state = applySecretReveal(_state, null);
}
