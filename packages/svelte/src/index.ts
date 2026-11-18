import {
	createAuthClient,
	type AuthClient,
	type AuthClientConfig,
	type AuthSessionSnapshot,
	type ClientUser,
	type RedirectSignInOptions,
	type SignOutOptions,
	type UserOrganizationMembership
} from '@secure-auth/core/client';
import { hasPermission as evaluateScopedPermission, type PermissionCheckInput } from '@secure-auth/core/server';

export const AUTH_CONTEXT_KEY = Symbol.for('secure-auth:svelte-context');

export interface PublicAuthState {
	isLoaded: boolean;
	isSignedIn: boolean;
	user: ClientUser | null;
	sessionId: string | null;
	activeOrgId: string | null;
	orgSlug: string | null;
	orgRole: string | null;
	orgPermissions: string[];
	organizations: UserOrganizationMembership[];
	roles: string[];
	permissions: string[];
}

export interface SvelteAuthContext {
	client: AuthClient;
	subscribe(listener: (state: PublicAuthState) => void): () => void;
	getState(): PublicAuthState;
	hydrate(initial: Partial<PublicAuthState>): void;
	hasPermission(permission: string): boolean;
	hasRole(role: string): boolean;
	has(check: PermissionCheckInput): boolean;
	refreshOrganizations(): Promise<UserOrganizationMembership[]>;
	setActiveOrganization(organizationId: string | null): Promise<void>;
	signInWithRedirect(options?: RedirectSignInOptions): Promise<string>;
	signInWithPassword(input: { email: string; password: string }): Promise<void>;
	signUp(input: { email: string; password: string; displayName: string }): Promise<void>;
	verifyOtp(input: { email: string; code: string }): Promise<void>;
	signOut(options?: SignOutOptions): Promise<{ logoutUrl?: string }>;
}

function evaluateStateCheck(state: PublicAuthState, check: PermissionCheckInput): boolean {
	if (!state.isSignedIn) {
		return false;
	}
	return evaluateScopedPermission(
		{
			roles: state.roles,
			permissions: state.permissions,
			org_id: state.activeOrgId ?? undefined,
			org_role: state.orgRole ?? undefined,
			org_permissions: state.orgPermissions
		},
		check
	);
}

function snapshotToPublicState(
	snap: AuthSessionSnapshot,
	prev: PublicAuthState
): PublicAuthState {
	const user = snap.user ?? prev.user;
	const isSignedIn = Boolean(snap.accessToken || snap.sessionId || user);
	return {
		isLoaded: true,
		isSignedIn,
		user: isSignedIn ? user : null,
		sessionId: isSignedIn ? (snap.sessionId ?? prev.sessionId) : null,
		activeOrgId: isSignedIn ? (snap.activeOrganizationId ?? prev.activeOrgId) : null,
		orgSlug: isSignedIn ? (snap.orgSlug ?? prev.orgSlug) : null,
		orgRole: isSignedIn ? (snap.orgRole ?? prev.orgRole) : null,
		orgPermissions: isSignedIn
			? (snap.orgPermissions.length > 0 ? [...snap.orgPermissions] : [...prev.orgPermissions])
			: [],
		organizations: isSignedIn ? [...prev.organizations] : [],
		roles: isSignedIn ? (user?.roles ?? prev.roles) : [],
		permissions: isSignedIn ? (user?.permissions ?? prev.permissions) : []
	};
}

export function createSvelteAuthContext(
	config: AuthClientConfig,
	initialState?: Partial<PublicAuthState>
): SvelteAuthContext {
	const client = createAuthClient(config);
	let current: PublicAuthState = {
		isLoaded: Boolean(initialState?.isLoaded),
		isSignedIn: Boolean(initialState?.isSignedIn),
		user: initialState?.user ?? null,
		sessionId: initialState?.sessionId ?? null,
		activeOrgId: initialState?.activeOrgId ?? null,
		orgSlug: initialState?.orgSlug ?? null,
		orgRole: initialState?.orgRole ?? null,
		orgPermissions: initialState?.orgPermissions ? [...initialState.orgPermissions] : [],
		organizations: initialState?.organizations ? [...initialState.organizations] : [],
		roles: initialState?.roles ? [...initialState.roles] : [],
		permissions: initialState?.permissions ? [...initialState.permissions] : []
	};

	const listeners = new Set<(state: PublicAuthState) => void>();

	function cloneState(): PublicAuthState {
		return {
			...current,
			orgPermissions: [...current.orgPermissions],
			organizations: [...current.organizations],
			roles: [...current.roles],
			permissions: [...current.permissions]
		};
	}

	function emit(): void {
		const copy = cloneState();
		for (const listener of listeners) {
			listener(copy);
		}
	}

	let receivedInitialClientSnapshot = false;
	client.subscribe((snap) => {
		const isFirstEmission = !receivedInitialClientSnapshot;
		receivedInitialClientSnapshot = true;
		if (isFirstEmission && !snap.accessToken && !snap.sessionId && !snap.user) {
			return;
		}
		current = snapshotToPublicState(snap, current);
		emit();
	});

	return {
		client,
		subscribe(listener) {
			listeners.add(listener);
			listener(cloneState());
			return () => {
				listeners.delete(listener);
			};
		},
		getState: () => cloneState(),
		hydrate(initial) {
			current = {
				isLoaded: initial.isLoaded ?? true,
				isSignedIn: initial.isSignedIn ?? Boolean(initial.user || initial.sessionId),
				user: initial.user ?? null,
				sessionId: initial.sessionId ?? null,
				activeOrgId: initial.activeOrgId ?? null,
				orgSlug: initial.orgSlug ?? null,
				orgRole: initial.orgRole ?? null,
				orgPermissions: initial.orgPermissions ? [...initial.orgPermissions] : [],
				organizations: initial.organizations ? [...initial.organizations] : [],
				roles: initial.roles ? [...initial.roles] : [],
				permissions: initial.permissions ? [...initial.permissions] : []
			};
			emit();
		},
		hasPermission: (permission) =>
			current.permissions.includes(permission) || current.orgPermissions.includes(permission),
		hasRole: (role) => current.roles.includes(role) || current.orgRole === role,
		has: (check) => evaluateStateCheck(current, check),
		refreshOrganizations: async () => {
			const memberships = await client.listOrganizations();
			current = { ...current, organizations: memberships };
			emit();
			return memberships;
		},
		setActiveOrganization: async (organizationId) => {
			const switched = await client.switchActiveOrganization(organizationId);
			current = {
				...current,
				activeOrgId: switched.active_organization_id,
				orgSlug: switched.org_slug,
				orgRole: switched.org_role,
				orgPermissions: [...switched.org_permissions]
			};
			emit();
		},
		signInWithRedirect: (options) => client.signInWithRedirect(options),
		signInWithPassword: async (input) => {
			const res = await client.signInWithPassword(input);
			current = {
				isLoaded: true,
				isSignedIn: true,
				user: res.user,
				sessionId: res.session_id,
				activeOrgId: null,
				orgSlug: null,
				orgRole: null,
				orgPermissions: [],
				organizations: [],
				roles: [...res.user.roles],
				permissions: [...res.user.permissions]
			};
			emit();
		},
		signUp: async (input) => {
			await client.signUp(input);
		},
		verifyOtp: async (input) => {
			const res = await client.verifyOtp(input);
			current = {
				isLoaded: true,
				isSignedIn: true,
				user: res.user,
				sessionId: res.session_id,
				activeOrgId: null,
				orgSlug: null,
				orgRole: null,
				orgPermissions: [],
				organizations: [],
				roles: [...res.user.roles],
				permissions: [...res.user.permissions]
			};
			emit();
		},
		signOut: async (options) => {
			const res = await client.signOut(options);
			current = {
				isLoaded: true,
				isSignedIn: false,
				user: null,
				sessionId: null,
				activeOrgId: null,
				orgSlug: null,
				orgRole: null,
				orgPermissions: [],
				organizations: [],
				roles: [],
				permissions: []
			};
			emit();
			return res;
		}
	};
}

export function useAuth(ctx: SvelteAuthContext): SvelteAuthContext {
	return ctx;
}

export function useUser(ctx: SvelteAuthContext): ClientUser | null {
	return ctx.getState().user;
}

export function useSession(ctx: SvelteAuthContext): {
	isSignedIn: boolean;
	sessionId: string | null;
} {
	const st = ctx.getState();
	return { isSignedIn: st.isSignedIn, sessionId: st.sessionId };
}

export function usePermissions(ctx: SvelteAuthContext): {
	roles: string[];
	permissions: string[];
	orgRole: string | null;
	orgPermissions: string[];
	hasPermission(permission: string): boolean;
	hasRole(role: string): boolean;
	has(check: PermissionCheckInput): boolean;
} {
	const st = ctx.getState();
	return {
		roles: st.roles,
		permissions: st.permissions,
		orgRole: st.orgRole,
		orgPermissions: st.orgPermissions,
		hasPermission: ctx.hasPermission,
		hasRole: ctx.hasRole,
		has: ctx.has
	};
}

export function useOrganization(ctx: SvelteAuthContext): {
	organization: UserOrganizationMembership['organization'] | null;
	membership: UserOrganizationMembership | null;
	orgId: string | null;
	orgSlug: string | null;
	orgRole: string | null;
	orgPermissions: string[];
	setActive(organizationId: string | null): Promise<void>;
} {
	const st = ctx.getState();
	const membership = st.activeOrgId
		? (st.organizations.find((item) => item.organization.id === st.activeOrgId) ?? null)
		: null;
	return {
		organization: membership?.organization ?? null,
		membership,
		orgId: st.activeOrgId,
		orgSlug: st.orgSlug,
		orgRole: st.orgRole,
		orgPermissions: st.orgPermissions,
		setActive: ctx.setActiveOrganization
	};
}

export function useOrganizationList(ctx: SvelteAuthContext): {
	organizations: UserOrganizationMembership[];
	activeOrgId: string | null;
	refresh(): Promise<UserOrganizationMembership[]>;
	setActive(organizationId: string | null): Promise<void>;
} {
	const st = ctx.getState();
	return {
		organizations: st.organizations,
		activeOrgId: st.activeOrgId,
		refresh: ctx.refreshOrganizations,
		setActive: ctx.setActiveOrganization
	};
}

export function canAccessRoute(
	state: PublicAuthState,
	requirements: PermissionCheckInput = {}
): boolean {
	if (!state.isSignedIn) {
		return false;
	}
	if (!requirements.permission && !requirements.role) {
		return !requirements.orgId || state.activeOrgId === requirements.orgId;
	}
	return evaluateStateCheck(state, requirements);
}
