import assert from 'node:assert/strict';
import test from 'node:test';
import { AuthSdkError } from '@secure-auth/core';
import {
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
} from '../dist/index.js';

function createTestReactRuntime() {
	const contextStack = new Map();
	let hookIndex = 0;
	let currentHooks = [];

	const runtime = {
		Fragment: Symbol('Fragment'),
		createContext(defaultValue) {
			const ctx = {
				_currentValue: defaultValue,
				Provider: null
			};
			ctx.Provider = ({ value, children }) => {
				contextStack.set(ctx, value);
				return { type: 'Provider', props: { value, children } };
			};
			return ctx;
		},
		useContext(ctx) {
			return contextStack.has(ctx) ? contextStack.get(ctx) : ctx._currentValue;
		},
		useState(initial) {
			const idx = hookIndex++;
			if (!(idx in currentHooks)) {
				currentHooks[idx] = typeof initial === 'function' ? initial() : initial;
			}
			const setState = (next) => {
				currentHooks[idx] = typeof next === 'function' ? next(currentHooks[idx]) : next;
			};
			return [currentHooks[idx], setState];
		},
		useEffect(effect) {
			effect();
		},
		useMemo(factory) {
			return factory();
		},
		createElement(type, props, ...children) {
			return {
				type,
				props: { ...(props ?? {}), children: children.length === 1 ? children[0] : children }
			};
		}
	};

	function renderComponent(fn, props) {
		hookIndex = 0;
		currentHooks = [];
		return fn(props);
	}

	return { runtime, renderComponent };
}

test('createReactAuth provides AuthProvider, hooks, and gated components', async () => {
	const { runtime, renderComponent } = createTestReactRuntime();
	const sdk = createReactAuth(runtime);

	assert.throws(() => sdk.useAuth(), (err) => {
		assert.ok(err instanceof AuthSdkError);
		assert.equal(err.code, 'MISSING_PROVIDER');
		return true;
	});

	const providerElement = renderComponent(sdk.AuthProvider, {
		config: {
			issuerUrl: 'https://auth.example.test',
			publishableKey: 'pub-elysiar',
			clientId: 'client_elysiar',
			redirectUri: 'https://elysiar.example.test/auth/callback'
		},
		initialState: {
			isLoaded: true,
			isSignedIn: true,
			sessionId: 'sid_react_1',
			user: {
				id: 'u_react_1',
				app_user_id: 'au_react_1',
				email: 'react@example.test',
				email_verified: true,
				display_name: 'Ada Lovelace',
				roles: ['admin'],
				permissions: ['billing:write', 'users:read']
			},
			roles: ['admin'],
			permissions: ['billing:write', 'users:read']
		},
		children: 'app-shell'
	});
	providerElement.type(providerElement.props);

	const auth = sdk.useAuth();
	assert.equal(auth.isSignedIn, true);
	assert.equal(sdk.useUser().user?.email, 'react@example.test');
	assert.equal(sdk.useSession().sessionId, 'sid_react_1');
	assert.equal(sdk.usePermissions().hasPermission('billing:write'), true);
	assert.equal(sdk.usePermissions().hasRole('admin'), true);

	const signedInTree = renderComponent(sdk.SignedIn, { children: 'secret-panel' });
	assert.equal(signedInTree?.props.children, 'secret-panel');
	const signedOutTree = renderComponent(sdk.SignedOut, { children: 'login-cta' });
	assert.equal(signedOutTree, null);

	const allowedProtect = renderComponent(sdk.Protect, {
		permission: 'billing:write',
		fallback: 'denied',
		children: 'billing-console'
	});
	assert.equal(allowedProtect?.props.children, 'billing-console');

	const deniedProtect = renderComponent(sdk.Protect, {
		permission: 'org:delete',
		fallback: 'denied',
		children: 'danger-zone'
	});
	assert.equal(deniedProtect?.props.children, 'denied');

	const signInCard = renderComponent(sdk.SignIn, { mode: 'hosted' });
	assert.equal(signInCard.type, 'section');
	const signUpCard = renderComponent(sdk.SignUp, {});
	assert.equal(signUpCard.type, 'section');
	const userBtn = renderComponent(sdk.UserButton, {});
	assert.equal(userBtn?.type, 'div');

	assert.equal(
		canAccess(
			{ isSignedIn: true, roles: ['member'], permissions: ['read'] },
			{ permission: 'read' }
		),
		true
	);
	assert.equal(
		canAccess(
			{ isSignedIn: false, roles: ['admin'], permissions: ['read'] },
			{ permission: 'read' }
		),
		false
	);
});

test('configureReactRuntime wires top-level React exports cleanly', () => {
	const { runtime, renderComponent } = createTestReactRuntime();
	configureReactRuntime(runtime);

	const tree = renderComponent(AuthProvider, {
		config: {
			issuerUrl: 'https://auth.example.test',
			publishableKey: 'pub-elysiar'
		},
		initialState: {
			isLoaded: true,
			isSignedIn: false,
			roles: [],
			permissions: []
		},
		children: 'guest'
	});
	tree.type(tree.props);

	assert.equal(useAuth().isSignedIn, false);
	assert.equal(useUser().user, null);
	assert.equal(useSession().sessionId, null);
	assert.deepEqual(usePermissions().permissions, []);
	assert.equal(renderComponent(SignedIn, { children: 'in' }), null);
	assert.equal(renderComponent(SignedOut, { children: 'out' })?.props.children, 'out');
	assert.equal(renderComponent(UserButton, {}), null);
	assert.equal(renderComponent(Protect, { fallback: 'blocked', children: 'ok' })?.props.children, 'blocked');
	assert.equal(renderComponent(SignIn, { mode: 'direct' }).type, 'section');
	assert.equal(renderComponent(SignUp, {}).type, 'section');
});

