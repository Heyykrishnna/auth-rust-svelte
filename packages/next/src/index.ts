'use client';

export {
	AuthProvider,
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
	usePermissions,
	useSession,
	useUser
} from '@secure-auth/react';

export type {
	AuthGateProps,
	AuthProviderProps,
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

