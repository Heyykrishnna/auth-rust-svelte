import { describe, expect, it } from 'vitest';
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
	getActiveWorkspace
} from '../control/state';

const mockDev = {
	id: 'dev-1',
	email: 'founder@example.com',
	display_name: 'Founder',
	email_verified: true,
	status: 'active',
	created_at: '2026-11-18T00:00:00Z'
};

const mockWorkspaces = [
	{
		id: 'ws-1',
		slug: 'core-team',
		name: 'Core Team',
		status: 'active',
		role: 'owner',
		created_at: '2026-11-18T00:00:00Z'
	},
	{
		id: 'ws-2',
		slug: 'labs',
		name: 'Labs',
		status: 'active',
		role: 'admin',
		created_at: '2026-11-18T00:00:00Z'
	}
];

const mockApps = [
	{
		application: {
			id: 'app-ferrum',
			workspace_id: 'ws-1',
			slug: 'ferrum',
			name: 'Ferrum',
			status: 'active',
			metadata: {},
			created_at: '2026-11-18T00:00:00Z',
			updated_at: '2026-11-18T00:00:00Z'
		},
		environments: [
			{
				id: 'env-ferrum-dev',
				application_id: 'app-ferrum',
				kind: 'development' as const,
				slug: 'ferrum-dev',
				enrollment_policy: 'consent_required' as const,
				session_ttl_secs: 604800,
				access_token_ttl_secs: 300,
				require_email_verification: true,
				require_mfa: false,
				branding: {},
				settings: {},
				status: 'active',
				created_at: '2026-11-18T00:00:00Z',
				updated_at: '2026-11-18T00:00:00Z'
			},
			{
				id: 'env-ferrum-prod',
				application_id: 'app-ferrum',
				kind: 'production' as const,
				slug: 'ferrum-prod',
				enrollment_policy: 'consent_required' as const,
				session_ttl_secs: 604800,
				access_token_ttl_secs: 300,
				require_email_verification: true,
				require_mfa: false,
				branding: {},
				settings: {},
				status: 'active',
				created_at: '2026-11-18T00:00:00Z',
				updated_at: '2026-11-18T00:00:00Z'
			}
		]
	},
	{
		application: {
			id: 'app-elysiar',
			workspace_id: 'ws-1',
			slug: 'elysiar',
			name: 'Elysiar',
			status: 'active',
			metadata: {},
			created_at: '2026-11-18T00:00:00Z',
			updated_at: '2026-11-18T00:00:00Z'
		},
		environments: [
			{
				id: 'env-elysiar-dev',
				application_id: 'app-elysiar',
				kind: 'development' as const,
				slug: 'elysiar-dev',
				enrollment_policy: 'auto_join' as const,
				session_ttl_secs: 604800,
				access_token_ttl_secs: 300,
				require_email_verification: true,
				require_mfa: false,
				branding: {},
				settings: {},
				status: 'active',
				created_at: '2026-11-18T00:00:00Z',
				updated_at: '2026-11-18T00:00:00Z'
			}
		]
	}
];

describe('control plane switcher & one-time reveal state', () => {
	it('switches workspaces, applications, and development/production environments', () => {
		let state = createInitialControlState();
		state = applyDeveloper(state, mockDev);
		state = applyWorkspaces(state, mockWorkspaces);
		state = applyApplications(state, mockApps);

		expect(getActiveWorkspace(state)?.id).toBe('ws-1');
		expect(getActiveApplication(state)?.application.id).toBe('app-ferrum');
		expect(getActiveEnvironment(state)?.id).toBe('env-ferrum-dev');

		state = applyEnvironmentSelection(state, 'production');
		expect(getActiveEnvironment(state)?.id).toBe('env-ferrum-prod');

		state = applyApplicationSelection(state, 'app-elysiar');
		expect(getActiveApplication(state)?.application.slug).toBe('elysiar');
		expect(getActiveEnvironment(state)?.id).toBe('env-elysiar-dev');

		state = applyWorkspaceSelection(state, 'ws-2');
		expect(getActiveWorkspace(state)?.id).toBe('ws-2');
	});

	it('reveals and permanently clears one-time secret keys', () => {
		let state = applyDeveloper(createInitialControlState(), mockDev);
		state = applySecretReveal(state, {
			title: 'Initial Secret Key',
			secretKey: 'sk_test_0123456789',
			publishableKey: 'pk_test_abcdef'
		});
		expect(state.revealedSecret?.secretKey).toBe('sk_test_0123456789');

		state = applySecretReveal(state, null);
		expect(state.revealedSecret).toBeNull();
	});

	it('resets all workspaces, apps, and secrets on developer sign-out', () => {
		let state = createInitialControlState();
		state = applyDeveloper(state, mockDev);
		state = applyWorkspaces(state, mockWorkspaces);
		state = applyApplications(state, mockApps);
		state = applySecretReveal(state, { title: 'Secret', secretKey: 'sk_live_secret' });

		state = applyDeveloper(state, null);
		expect(state.developer).toBeNull();
		expect(state.workspaces).toHaveLength(0);
		expect(state.applications).toHaveLength(0);
		expect(state.revealedSecret).toBeNull();
	});
});
