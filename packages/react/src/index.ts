import {
	AuthSdkError,
	createAuthClient,
	hasPermission as evaluateScopedPermission,
	type AuthSessionSnapshot,
	type ClientUser,
	type OrganizationInvitationView,
	type OrganizationMemberView,
	type PermissionCheckInput,
	type UserOrganizationMembership
} from '@secure-auth/core';
import type {
	AuthGateProps,
	AuthProviderProps,
	OrganizationProfileProps,
	OrganizationSwitcherProps,
	ProtectProps,
	ReactAuthContextValue,
	ReactAuthState,
	ReactContextLike,
	ReactElementLike,
	ReactNodeLike,
	ReactRuntime,
	SignInProps,
	SignUpProps,
	UserButtonProps
} from './types.js';

export type {
	AuthGateProps,
	AuthProviderProps,
	OrganizationProfileProps,
	OrganizationSwitcherProps,
	ProtectProps,
	ReactAuthContextValue,
	ReactAuthState,
	ReactContextLike,
	ReactElementLike,
	ReactNodeLike,
	ReactRuntime,
	SignInProps,
	SignUpProps,
	UserButtonProps
} from './types.js';

export interface ReactAuthSdk {
	AuthContext: ReactContextLike<ReactAuthContextValue | null>;
	AuthProvider(props: AuthProviderProps): ReactElementLike;
	useAuth(): ReactAuthContextValue;
	useUser(): { isLoaded: boolean; isSignedIn: boolean; user: ClientUser | null };
	useSession(): { isLoaded: boolean; isSignedIn: boolean; sessionId: string | null };
	usePermissions(): {
		roles: string[];
		permissions: string[];
		orgRole: string | null;
		orgPermissions: string[];
		hasPermission(permission: string): boolean;
		hasRole(role: string): boolean;
		has(check: PermissionCheckInput): boolean;
	};
	useOrganization(): {
		isLoaded: boolean;
		organization: UserOrganizationMembership['organization'] | null;
		membership: UserOrganizationMembership | null;
		orgId: string | null;
		orgSlug: string | null;
		orgRole: string | null;
		orgPermissions: string[];
		setActive(organizationId: string | null): Promise<void>;
	};
	useOrganizationList(): {
		isLoaded: boolean;
		organizations: UserOrganizationMembership[];
		activeOrgId: string | null;
		refresh(): Promise<UserOrganizationMembership[]>;
		setActive(organizationId: string | null): Promise<void>;
	};
	SignIn(props?: SignInProps): ReactElementLike;
	SignUp(props?: SignUpProps): ReactElementLike;
	UserButton(props?: UserButtonProps): ReactElementLike | null;
	OrganizationSwitcher(props?: OrganizationSwitcherProps): ReactElementLike | null;
	OrganizationProfile(props?: OrganizationProfileProps): ReactElementLike | null;
	SignedIn(props: AuthGateProps): ReactElementLike | null;
	SignedOut(props: AuthGateProps): ReactElementLike | null;
	Protect(props: ProtectProps): ReactElementLike | null;
}

function initialAuthState(initial?: Partial<ReactAuthState>): ReactAuthState {
	return {
		isLoaded: Boolean(initial?.isLoaded),
		isSignedIn: Boolean(initial?.isSignedIn),
		user: initial?.user ?? null,
		sessionId: initial?.sessionId ?? null,
		activeOrgId: initial?.activeOrgId ?? null,
		orgSlug: initial?.orgSlug ?? null,
		orgRole: initial?.orgRole ?? null,
		orgPermissions: initial?.orgPermissions ? [...initial.orgPermissions] : [],
		organizations: initial?.organizations ? [...initial.organizations] : [],
		roles: initial?.roles ? [...initial.roles] : [],
		permissions: initial?.permissions ? [...initial.permissions] : []
	};
}

function snapshotToReactState(
	snap: AuthSessionSnapshot,
	prev: ReactAuthState
): ReactAuthState {
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

export function canAccess(
	state: Pick<
		ReactAuthState,
		'isSignedIn' | 'roles' | 'permissions'
	> &
		Partial<Pick<ReactAuthState, 'activeOrgId' | 'orgRole' | 'orgPermissions'>>,
	requirements: PermissionCheckInput = {}
): boolean {
	if (!state.isSignedIn) {
		return false;
	}
	if (!requirements.permission && !requirements.role) {
		return !requirements.orgId || state.activeOrgId === requirements.orgId;
	}
	return evaluateScopedPermission(
		{
			roles: state.roles,
			permissions: state.permissions,
			org_id: state.activeOrgId ?? undefined,
			org_role: state.orgRole ?? undefined,
			org_permissions: state.orgPermissions ?? []
		},
		requirements
	);
}

export function createReactAuth(react: ReactRuntime): ReactAuthSdk {
	const AuthContext = react.createContext<ReactAuthContextValue | null>(null);

	function useAuth(): ReactAuthContextValue {
		const ctx = react.useContext(AuthContext);
		if (!ctx) {
			throw new AuthSdkError('MISSING_PROVIDER', 'useAuth must be used inside <AuthProvider>');
		}
		return ctx;
	}

	function useUser(): { isLoaded: boolean; isSignedIn: boolean; user: ClientUser | null } {
		const ctx = useAuth();
		return {
			isLoaded: ctx.isLoaded,
			isSignedIn: ctx.isSignedIn,
			user: ctx.user
		};
	}

	function useSession(): { isLoaded: boolean; isSignedIn: boolean; sessionId: string | null } {
		const ctx = useAuth();
		return {
			isLoaded: ctx.isLoaded,
			isSignedIn: ctx.isSignedIn,
			sessionId: ctx.sessionId
		};
	}

	function usePermissions() {
		const ctx = useAuth();
		return {
			roles: ctx.roles,
			permissions: ctx.permissions,
			orgRole: ctx.orgRole,
			orgPermissions: ctx.orgPermissions,
			hasPermission: ctx.hasPermission,
			hasRole: ctx.hasRole,
			has: ctx.has
		};
	}

	function useOrganization() {
		const ctx = useAuth();
		const membership = ctx.activeOrgId
			? (ctx.organizations.find((item) => item.organization.id === ctx.activeOrgId) ?? null)
			: null;
		return {
			isLoaded: ctx.isLoaded,
			organization: membership?.organization ?? null,
			membership,
			orgId: ctx.activeOrgId,
			orgSlug: ctx.orgSlug,
			orgRole: ctx.orgRole,
			orgPermissions: ctx.orgPermissions,
			setActive: ctx.setActiveOrganization
		};
	}

	function useOrganizationList() {
		const ctx = useAuth();
		return {
			isLoaded: ctx.isLoaded,
			organizations: ctx.organizations,
			activeOrgId: ctx.activeOrgId,
			refresh: ctx.refreshOrganizations,
			setActive: ctx.setActiveOrganization
		};
	}

	function AuthProvider(props: AuthProviderProps): ReactElementLike {
		const client = react.useMemo(
			() => createAuthClient(props.config),
			[
				props.config.issuerUrl,
				props.config.publishableKey,
				props.config.clientId,
				props.config.redirectUri
			]
		);

		const [state, setState] = react.useState<ReactAuthState>(() =>
			initialAuthState(props.initialState)
		);

		react.useEffect(() => {
			let receivedInitial = false;
			return client.subscribe((snap) => {
				const isFirstEmission = !receivedInitial;
				receivedInitial = true;
				if (isFirstEmission && !snap.accessToken && !snap.sessionId && !snap.user) {
					return;
				}
				setState((prev) => snapshotToReactState(snap, prev));
			});
		}, [client]);

		const value = react.useMemo<ReactAuthContextValue>(
			() => ({
				...state,
				client,
				hasPermission: (permission: string) =>
					state.permissions.includes(permission) || state.orgPermissions.includes(permission),
				hasRole: (role: string) => state.roles.includes(role) || state.orgRole === role,
				has: (check: PermissionCheckInput) => canAccess(state, check),
				refreshOrganizations: async () => {
					const list = await client.listOrganizations();
					setState((prev) => ({ ...prev, organizations: list }));
					return list;
				},
				setActiveOrganization: async (organizationId: string | null) => {
					const switched = await client.switchActiveOrganization(organizationId);
					setState((prev) => ({
						...prev,
						activeOrgId: switched.active_organization_id,
						orgSlug: switched.org_slug,
						orgRole: switched.org_role,
						orgPermissions: [...switched.org_permissions]
					}));
				},
				signInWithRedirect: (options) => client.signInWithRedirect(options),
				signInWithPassword: async (input) => {
					const res = await client.signInWithPassword(input);
					setState({
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
					});
				},
				signUp: async (input) => {
					await client.signUp(input);
				},
				verifyOtp: async (input) => {
					const res = await client.verifyOtp(input);
					setState({
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
					});
				},
				signOut: async (options) => {
					const res = await client.signOut(options);
					setState({
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
					});
					return res;
				}
			}),
			[state, client]
		);

		return react.createElement(AuthContext.Provider, { value }, props.children ?? null);
	}

	function SignIn(props: SignInProps = {}): ReactElementLike {
		const auth = useAuth();
		const mode = props.mode ?? 'hosted';
		const [email, setEmail] = react.useState('');
		const [password, setPassword] = react.useState('');
		const [submitting, setSubmitting] = react.useState(false);
		const [errorMessage, setErrorMessage] = react.useState<string | null>(null);

		const onHostedClick = async () => {
			setSubmitting(true);
			setErrorMessage(null);
			try {
				const url = await auth.signInWithRedirect({ redirectUri: props.redirectUri });
				if (typeof globalThis.location !== 'undefined' && typeof globalThis.location.assign === 'function') {
					globalThis.location.assign(url);
				}
				props.onSuccess?.();
			} catch (err) {
				setErrorMessage(err instanceof Error ? err.message : 'Unable to start sign-in');
				props.onError?.(err);
			} finally {
				setSubmitting(false);
			}
		};

		const onDirectSubmit = async (event: { preventDefault(): void }) => {
			event.preventDefault();
			setSubmitting(true);
			setErrorMessage(null);
			try {
				await auth.signInWithPassword({ email: email.trim(), password });
				props.onSuccess?.();
			} catch (err) {
				setErrorMessage(err instanceof Error ? err.message : 'Sign-in failed');
				props.onError?.(err);
			} finally {
				setSubmitting(false);
			}
		};

		const body =
			mode === 'hosted'
				? react.createElement(
						'button',
						{
							type: 'button',
							className: 'sa-button sa-button-primary',
							disabled: submitting,
							onClick: onHostedClick
						},
						submitting ? 'Redirecting…' : 'Continue with SSO'
					)
				: react.createElement(
						'form',
						{ className: 'sa-form', onSubmit: onDirectSubmit },
						react.createElement(
							'label',
							{ htmlFor: 'sa-react-signin-email' },
							'Email'
						),
						react.createElement('input', {
							id: 'sa-react-signin-email',
							type: 'email',
							value: email,
							required: true,
							onChange: (e: { target: { value: string } }) => setEmail(e.target.value)
						}),
						react.createElement(
							'label',
							{ htmlFor: 'sa-react-signin-password' },
							'Password'
						),
						react.createElement('input', {
							id: 'sa-react-signin-password',
							type: 'password',
							value: password,
							required: true,
							onChange: (e: { target: { value: string } }) => setPassword(e.target.value)
						}),
						react.createElement(
							'button',
							{
								type: 'submit',
								className: 'sa-button sa-button-primary',
								disabled: submitting
							},
							submitting ? 'Signing in…' : 'Sign in'
						)
					);

		const errorEl = errorMessage
			? react.createElement(
					'p',
					{ className: 'sa-error', role: 'alert', 'aria-live': 'polite' },
					errorMessage
				)
			: null;

		return react.createElement(
			'section',
			{ className: 'sa-card', 'aria-label': 'Sign in' },
			body,
			errorEl
		);
	}

	function SignUp(props: SignUpProps = {}): ReactElementLike {
		const auth = useAuth();
		const [displayName, setDisplayName] = react.useState('');
		const [email, setEmail] = react.useState('');
		const [password, setPassword] = react.useState('');
		const [otpCode, setOtpCode] = react.useState('');
		const [awaitingVerification, setAwaitingVerification] = react.useState(false);
		const [submitting, setSubmitting] = react.useState(false);
		const [errorMessage, setErrorMessage] = react.useState<string | null>(null);

		const onSignUpSubmit = async (event: { preventDefault(): void }) => {
			event.preventDefault();
			setSubmitting(true);
			setErrorMessage(null);
			try {
				await auth.signUp({
					email: email.trim(),
					password,
					displayName: displayName.trim()
				});
				setAwaitingVerification(true);
			} catch (err) {
				setErrorMessage(err instanceof Error ? err.message : 'Sign-up failed');
				props.onError?.(err);
			} finally {
				setSubmitting(false);
			}
		};

		const onVerifySubmit = async (event: { preventDefault(): void }) => {
			event.preventDefault();
			setSubmitting(true);
			setErrorMessage(null);
			try {
				await auth.verifyOtp({ email: email.trim(), code: otpCode.trim() });
				props.onSuccess?.();
			} catch (err) {
				setErrorMessage(err instanceof Error ? err.message : 'Verification failed');
				props.onError?.(err);
			} finally {
				setSubmitting(false);
			}
		};

		const form = !awaitingVerification
			? react.createElement(
					'form',
					{ className: 'sa-form', onSubmit: onSignUpSubmit },
					react.createElement('label', { htmlFor: 'sa-react-signup-name' }, 'Display name'),
					react.createElement('input', {
						id: 'sa-react-signup-name',
						type: 'text',
						value: displayName,
						required: true,
						onChange: (e: { target: { value: string } }) => setDisplayName(e.target.value)
					}),
					react.createElement('label', { htmlFor: 'sa-react-signup-email' }, 'Email'),
					react.createElement('input', {
						id: 'sa-react-signup-email',
						type: 'email',
						value: email,
						required: true,
						onChange: (e: { target: { value: string } }) => setEmail(e.target.value)
					}),
					react.createElement('label', { htmlFor: 'sa-react-signup-password' }, 'Password'),
					react.createElement('input', {
						id: 'sa-react-signup-password',
						type: 'password',
						value: password,
						minLength: 12,
						required: true,
						onChange: (e: { target: { value: string } }) => setPassword(e.target.value)
					}),
					react.createElement(
						'button',
						{
							type: 'submit',
							className: 'sa-button sa-button-primary',
							disabled: submitting
						},
						submitting ? 'Creating account…' : 'Create account'
					)
				)
			: react.createElement(
					'form',
					{ className: 'sa-form', onSubmit: onVerifySubmit },
					react.createElement('label', { htmlFor: 'sa-react-signup-otp' }, 'Verification code'),
					react.createElement('input', {
						id: 'sa-react-signup-otp',
						type: 'text',
						inputMode: 'numeric',
						value: otpCode,
						required: true,
						onChange: (e: { target: { value: string } }) => setOtpCode(e.target.value)
					}),
					react.createElement(
						'button',
						{
							type: 'submit',
							className: 'sa-button sa-button-primary',
							disabled: submitting
						},
						submitting ? 'Verifying…' : 'Verify email'
					)
				);

		const errorEl = errorMessage
			? react.createElement(
					'p',
					{ className: 'sa-error', role: 'alert', 'aria-live': 'polite' },
					errorMessage
				)
			: null;

		return react.createElement(
			'section',
			{ className: 'sa-card', 'aria-label': 'Create account' },
			form,
			errorEl
		);
	}

	function UserButton(props: UserButtonProps = {}): ReactElementLike | null {
		const auth = useAuth();
		const [menuOpen, setMenuOpen] = react.useState(false);
		const [busy, setBusy] = react.useState(false);

		if (!auth.isSignedIn) {
			return null;
		}

		const labelSource = auth.user?.display_name ?? auth.user?.email ?? 'U';
		const initials = labelSource.slice(0, 2).toUpperCase();

		const onSignOutClick = async () => {
			setBusy(true);
			try {
				const result = await auth.signOut({
					global: props.globalSignOut ?? false,
					postLogoutRedirectUri: props.postLogoutRedirectUri
				});
				setMenuOpen(false);
				props.onSignedOut?.();
				if (
					result.logoutUrl &&
					typeof globalThis.location !== 'undefined' &&
					typeof globalThis.location.assign === 'function'
				) {
					globalThis.location.assign(result.logoutUrl);
				}
			} finally {
				setBusy(false);
			}
		};

		const menu = menuOpen
			? react.createElement(
					'div',
					{ className: 'sa-menu', role: 'menu' },
					react.createElement(
						'div',
						{ className: 'sa-menu-header' },
						react.createElement('strong', null, auth.user?.display_name ?? 'Account'),
						auth.user?.email ? react.createElement('span', null, auth.user.email) : null
					),
					react.createElement(
						'button',
						{
							type: 'button',
							role: 'menuitem',
							className: 'sa-menu-item',
							disabled: busy,
							onClick: onSignOutClick
						},
						busy ? 'Signing out…' : 'Sign out'
					)
				)
			: null;

		return react.createElement(
			'div',
			{ className: 'sa-user-button' },
			react.createElement(
				'button',
				{
					type: 'button',
					className: 'sa-avatar-trigger',
					'aria-haspopup': 'menu',
					'aria-expanded': menuOpen,
					onClick: () => setMenuOpen((prev) => !prev)
				},
				react.createElement('span', { className: 'sa-avatar' }, initials)
			),
			menu
		);
	}

	function OrganizationSwitcher(props: OrganizationSwitcherProps = {}): ReactElementLike | null {
		const auth = useAuth();
		const [open, setOpen] = react.useState(false);
		const [switching, setSwitching] = react.useState(false);

		if (!auth.isSignedIn) {
			return null;
		}

		const allowPersonal = props.allowPersonal ?? true;
		const activeOrg = auth.organizations.find((o) => o.organization.id === auth.activeOrgId);
		const label = activeOrg?.organization.name ?? auth.orgSlug ?? 'Personal Account';

		const selectOrg = async (orgId: string | null) => {
			setSwitching(true);
			try {
				await auth.setActiveOrganization(orgId);
				setOpen(false);
				props.onSwitch?.(orgId);
			} finally {
				setSwitching(false);
			}
		};

		const options: ReactElementLike[] = [];
		if (allowPersonal) {
			options.push(
				react.createElement(
					'button',
					{
						key: '__personal',
						type: 'button',
						role: 'option',
						'aria-selected': auth.activeOrgId === null,
						disabled: switching,
						onClick: () => selectOrg(null)
					},
					'Personal Account'
				)
			);
		}
		for (const item of auth.organizations) {
			options.push(
				react.createElement(
					'button',
					{
						key: item.organization.id,
						type: 'button',
						role: 'option',
						'aria-selected': auth.activeOrgId === item.organization.id,
						disabled: switching,
						onClick: () => selectOrg(item.organization.id)
					},
					`${item.organization.name} (${item.roles[0] ?? 'org:member'})`
				)
			);
		}

		const menu = open
			? react.createElement('div', { className: 'sa-org-menu', role: 'listbox' }, options)
			: null;

		return react.createElement(
			'div',
			{ className: 'sa-org-switcher' },
			react.createElement(
				'button',
				{
					type: 'button',
					className: 'sa-org-trigger',
					'aria-haspopup': 'listbox',
					'aria-expanded': open,
					disabled: switching,
					onClick: () => setOpen((prev) => !prev)
				},
				label
			),
			menu
		);
	}

	function OrganizationProfile(props: OrganizationProfileProps = {}): ReactElementLike | null {
		const auth = useAuth();
		const targetOrgId = props.organizationId ?? auth.activeOrgId;
		const [members, setMembers] = react.useState<OrganizationMemberView[]>([]);
		const [invitations, setInvitations] = react.useState<OrganizationInvitationView[]>([]);

		react.useEffect(() => {
			if (!targetOrgId || !auth.isSignedIn) {
				setMembers([]);
				setInvitations([]);
				return;
			}
			void auth.client
				.listOrganizationMembers(targetOrgId)
				.then(setMembers)
				.catch(() => setMembers([]));
			void auth.client
				.listOrganizationInvitations(targetOrgId)
				.then(setInvitations)
				.catch(() => setInvitations([]));
		}, [targetOrgId, auth.isSignedIn, auth.client]);

		if (!targetOrgId) {
			return null;
		}

		return react.createElement(
			'section',
			{ className: 'sa-org-profile', 'aria-label': 'Organization profile' },
			react.createElement('h3', null, auth.orgSlug ?? targetOrgId),
			react.createElement(
				'ul',
				{ className: 'sa-org-members' },
				members.map((m) =>
					react.createElement('li', { key: m.id }, `${m.display_name} (${m.roles.join(', ')})`)
				)
			),
			invitations.length > 0
				? react.createElement(
						'ul',
						{ className: 'sa-org-invitations' },
						invitations.map((inv) =>
							react.createElement('li', { key: inv.id }, `${inv.email} - ${inv.status}`)
						)
					)
				: null
		);
	}

	function SignedIn(props: AuthGateProps): ReactElementLike | null {
		const auth = useAuth();
		if (!auth.isLoaded || !auth.isSignedIn) {
			return null;
		}
		return react.createElement(react.Fragment, null, props.children ?? null);
	}

	function SignedOut(props: AuthGateProps): ReactElementLike | null {
		const auth = useAuth();
		if (!auth.isLoaded || auth.isSignedIn) {
			return null;
		}
		return react.createElement(react.Fragment, null, props.children ?? null);
	}

	function Protect(props: ProtectProps): ReactElementLike | null {
		const auth = useAuth();
		if (!auth.isLoaded) {
			return null;
		}
		if (
			canAccess(auth, {
				permission: props.permission,
				role: props.role,
				orgId: props.orgId
			})
		) {
			return react.createElement(react.Fragment, null, props.children ?? null);
		}
		if (props.fallback === undefined || props.fallback === null) {
			return null;
		}
		return react.createElement(react.Fragment, null, props.fallback);
	}

	return {
		AuthContext,
		AuthProvider,
		useAuth,
		useUser,
		useSession,
		usePermissions,
		useOrganization,
		useOrganizationList,
		SignIn,
		SignUp,
		UserButton,
		OrganizationSwitcher,
		OrganizationProfile,
		SignedIn,
		SignedOut,
		Protect
	};
}

let defaultSdk: ReactAuthSdk | null = null;

export function configureReactRuntime(runtime: ReactRuntime): ReactAuthSdk {
	defaultSdk = createReactAuth(runtime);
	return defaultSdk;
}

async function tryAutoConfigureReact(): Promise<void> {
	const moduleName: string = 'react';
	try {
		const mod = (await import(moduleName)) as Partial<ReactRuntime> & {
			default?: Partial<ReactRuntime>;
		};
		const candidate = mod.createContext ? mod : mod.default;
		if (
			candidate &&
			typeof candidate.createContext === 'function' &&
			typeof candidate.useContext === 'function' &&
			typeof candidate.useState === 'function' &&
			typeof candidate.useEffect === 'function' &&
			typeof candidate.useMemo === 'function' &&
			typeof candidate.createElement === 'function'
		) {
			defaultSdk = createReactAuth(candidate as ReactRuntime);
		}
	} catch {
		return;
	}
}

await tryAutoConfigureReact();

function getDefaultSdk(): ReactAuthSdk {
	if (!defaultSdk) {
		throw new AuthSdkError(
			'REACT_RUNTIME_UNAVAILABLE',
			'React peer dependency is not installed. Install react or call configureReactRuntime().'
		);
	}
	return defaultSdk;
}

export function AuthProvider(props: AuthProviderProps): ReactElementLike {
	return getDefaultSdk().AuthProvider(props);
}

export function useAuth(): ReactAuthContextValue {
	return getDefaultSdk().useAuth();
}

export function useUser(): { isLoaded: boolean; isSignedIn: boolean; user: ClientUser | null } {
	return getDefaultSdk().useUser();
}

export function useSession(): {
	isLoaded: boolean;
	isSignedIn: boolean;
	sessionId: string | null;
} {
	return getDefaultSdk().useSession();
}

export function usePermissions() {
	return getDefaultSdk().usePermissions();
}

export function useOrganization() {
	return getDefaultSdk().useOrganization();
}

export function useOrganizationList() {
	return getDefaultSdk().useOrganizationList();
}

export function SignIn(props?: SignInProps): ReactElementLike {
	return getDefaultSdk().SignIn(props);
}

export function SignUp(props?: SignUpProps): ReactElementLike {
	return getDefaultSdk().SignUp(props);
}

export function UserButton(props?: UserButtonProps): ReactElementLike | null {
	return getDefaultSdk().UserButton(props);
}

export function OrganizationSwitcher(
	props?: OrganizationSwitcherProps
): ReactElementLike | null {
	return getDefaultSdk().OrganizationSwitcher(props);
}

export function OrganizationProfile(
	props?: OrganizationProfileProps
): ReactElementLike | null {
	return getDefaultSdk().OrganizationProfile(props);
}

export function SignedIn(props: AuthGateProps): ReactElementLike | null {
	return getDefaultSdk().SignedIn(props);
}

export function SignedOut(props: AuthGateProps): ReactElementLike | null {
	return getDefaultSdk().SignedOut(props);
}

export function Protect(props: ProtectProps): ReactElementLike | null {
	return getDefaultSdk().Protect(props);
}
