'use client';

export {
	AuthProvider,
	OrganizationProfile,
	OrganizationSwitcher,
	Protect,
	SignIn,
	SignUp,
	SignedIn,
	SignedOut,
	UserButton,
	canAccess,
	configureReactRuntime,
	createReactAuth,
	useAuth,
	useOrganization,
	useOrganizationList,
	usePermissions,
	useSession,
	useUser
} from '@secure-auth/react';

export type {
	AuthGateProps,
	AuthProviderProps,
	OrganizationProfileProps,
	OrganizationSwitcherProps,
	ProtectProps,
	ReactAuthContextValue,
	ReactAuthSdk,
	ReactAuthState,
	ReactContextLike,
	ReactElementLike,
	ReactNodeLike,
	ReactRuntime,
	SignInProps,
	SignUpProps,
	UserButtonProps
} from '@secure-auth/react';

