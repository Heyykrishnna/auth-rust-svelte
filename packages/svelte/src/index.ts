import {
	createAuthClient,
	type AuthClient,
	type AuthClientConfig,
	type AuthSessionSnapshot,
	type ClientUser,
	type RedirectSignInOptions,
	type SignOutOptions
} from '@secure-auth/core/client';

export const AUTH_CONTEXT_KEY = Symbol.for('secure-auth:svelte-context');

export interface PublicAuthState {
	isLoaded: boolean;
	isSignedIn: boolean;
	user: ClientUser | null;
	sessionId: string | null;
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
	signInWithRedirect(options?: RedirectSignInOptions): Promise<string>;
	signInWithPassword(input: { email: string; password: string }): Promise<void>;
	signUp(input: { email: string; password: string; displayName: string }): Promise<void>;
	verifyOtp(input: { email: string; code: string }): Promise<void>;
	signOut(options?: SignOutOptions): Promise<{ logoutUrl?: string }>;
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
		roles: initialState?.roles ? [...initialState.roles] : [],
		permissions: initialState?.permissions ? [...initialState.permissions] : []
	};

	const listeners = new Set<(state: PublicAuthState) => void>();

	function emit(): void {
		const copy: PublicAuthState = {
			...current,
			roles: [...current.roles],
			permissions: [...current.permissions]
		};
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
			listener({ ...current, roles: [...current.roles], permissions: [...current.permissions] });
			return () => {
				listeners.delete(listener);
			};
		},
		getState: () => ({
			...current,
			roles: [...current.roles],
			permissions: [...current.permissions]
		}),
		hydrate(initial) {
			current = {
				isLoaded: initial.isLoaded ?? true,
				isSignedIn: initial.isSignedIn ?? Boolean(initial.user || initial.sessionId),
				user: initial.user ?? null,
				sessionId: initial.sessionId ?? null,
				roles: initial.roles ? [...initial.roles] : [],
				permissions: initial.permissions ? [...initial.permissions] : []
			};
			emit();
		},
		hasPermission: (permission) => current.permissions.includes(permission),
		hasRole: (role) => current.roles.includes(role),
		signInWithRedirect: (options) => client.signInWithRedirect(options),
		signInWithPassword: async (input) => {
			const res = await client.signInWithPassword(input);
			current = {
				isLoaded: true,
				isSignedIn: true,
				user: res.user,
				sessionId: res.session_id,
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
	hasPermission(permission: string): boolean;
	hasRole(role: string): boolean;
} {
	const st = ctx.getState();
	return {
		roles: st.roles,
		permissions: st.permissions,
		hasPermission: ctx.hasPermission,
		hasRole: ctx.hasRole
	};
}

export function canAccessRoute(
	state: PublicAuthState,
	requirements: { permission?: string; role?: string } = {}
): boolean {
	if (!state.isSignedIn) {
		return false;
	}
	if (requirements.permission && !state.permissions.includes(requirements.permission)) {
		return false;
	}
	if (requirements.role && !state.roles.includes(requirements.role)) {
		return false;
	}
	return true;
}
