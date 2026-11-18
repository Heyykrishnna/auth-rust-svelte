import type {
	AuthClient,
	AuthClientConfig,
	ClientUser,
	RedirectSignInOptions,
	SignOutOptions,
	UserOrganizationMembership
} from '@secure-auth/core/client';
import type { PermissionCheckInput } from '@secure-auth/core/server';

export type ReactNodeLike =
	| string
	| number
	| boolean
	| null
	| undefined
	| ReactElementLike
	| ReactNodeLike[];

export interface ReactElementLike {
	type: unknown;
	props: Record<string, unknown>;
	key?: string | number | null;
}

export interface ReactContextLike<T> {
	Provider: unknown;
	Consumer?: unknown;
	_currentValue?: T;
}

export interface ReactRuntime {
	createContext<T>(defaultValue: T): ReactContextLike<T>;
	useContext<T>(context: ReactContextLike<T>): T;
	useState<S>(initialState: S | (() => S)): [S, (next: S | ((prev: S) => S)) => void];
	useEffect(effect: () => void | (() => void), deps?: readonly unknown[]): void;
	useMemo<T>(factory: () => T, deps: readonly unknown[]): T;
	createElement(
		type: unknown,
		props: Record<string, unknown> | null,
		...children: ReactNodeLike[]
	): ReactElementLike;
	Fragment: unknown;
}

export interface ReactAuthState {
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

export interface ReactAuthContextValue extends ReactAuthState {
	client: AuthClient;
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

export interface AuthProviderProps {
	config: AuthClientConfig;
	initialState?: Partial<ReactAuthState>;
	children?: ReactNodeLike;
}

export interface SignInProps {
	mode?: 'hosted' | 'direct';
	redirectUri?: string;
	onSuccess?: () => void;
	onError?: (error: unknown) => void;
}

export interface SignUpProps {
	onSuccess?: () => void;
	onError?: (error: unknown) => void;
}

export interface UserButtonProps {
	globalSignOut?: boolean;
	postLogoutRedirectUri?: string;
	onSignedOut?: () => void;
}

export interface AuthGateProps {
	children?: ReactNodeLike;
}

export interface ProtectProps {
	permission?: string;
	role?: string;
	orgId?: string;
	fallback?: ReactNodeLike;
	children?: ReactNodeLike;
}

export interface OrganizationSwitcherProps {
	allowPersonal?: boolean;
	onSwitch?: (orgId: string | null) => void;
}

export interface OrganizationProfileProps {
	organizationId?: string;
}
