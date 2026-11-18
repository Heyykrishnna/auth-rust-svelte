import { describe, expect, it, vi } from 'vitest';
import { controlApi } from './control';
import { apiClient } from './client';

describe('controlApi Phase 07 endpoints', () => {
	it('queries environment users with encoded filter parameters', async () => {
		const getSpy = vi.spyOn(apiClient, 'get').mockResolvedValueOnce({
			users: [],
			total: 0,
			limit: 20,
			offset: 0
		});

		await controlApi.listUsers('app-1', 'production', {
			q: 'alice@example.test',
			status: 'suspended',
			email_verified: false,
			role: 'admin',
			limit: 20,
			offset: 40
		});

		expect(getSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/production/users?q=alice%40example.test&status=suspended&email_verified=false&role=admin&limit=20&offset=40',
			undefined
		);
		getSpy.mockRestore();
	});

	it('calls OAuth provider, webhook replay, audit log, and workspace member endpoints', async () => {
		const putSpy = vi.spyOn(apiClient, 'put').mockResolvedValueOnce({});
		const postSpy = vi.spyOn(apiClient, 'post').mockResolvedValue({ replayed_count: 3 });
		const getSpy = vi.spyOn(apiClient, 'get').mockResolvedValue({});

		await controlApi.upsertOAuthProvider('app-1', 'development', 'google', {
			enabled: true,
			use_platform_credentials: true
		});
		expect(putSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/development/oauth-providers/google',
			{ enabled: true, use_platform_credentials: true },
			undefined
		);

		await controlApi.replayWebhookRange('app-1', 'development', 'ep-1', {
			from: '2026-11-18T00:00:00Z',
			to: '2026-11-18T12:00:00Z'
		});
		expect(postSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/development/webhooks/ep-1/replay',
			{ from: '2026-11-18T00:00:00Z', to: '2026-11-18T12:00:00Z' },
			undefined
		);

		await controlApi.listAuditLogs('app-1', 'development', {
			actor_type: 'developer',
			limit: 30,
			offset: 0
		});
		expect(getSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/development/audit-logs?actor_type=developer&limit=30&offset=0',
			undefined
		);

		await controlApi.addWorkspaceMember('ws-1', {
			email: 'teammate@example.test',
			role: 'viewer'
		});
		expect(postSpy).toHaveBeenCalledWith(
			'/v1/control/workspaces/ws-1/members',
			{ email: 'teammate@example.test', role: 'viewer' },
			undefined
		);

		putSpy.mockRestore();
		postSpy.mockRestore();
		getSpy.mockRestore();
	});

	it('calls Phase 08 organization, invitation, permission, and custom role endpoints', async () => {
		const getSpy = vi.spyOn(apiClient, 'get').mockResolvedValue({ data: [], has_more: false });
		const postSpy = vi.spyOn(apiClient, 'post').mockResolvedValue({});
		const patchSpy = vi.spyOn(apiClient, 'patch').mockResolvedValue({});
		const deleteSpy = vi.spyOn(apiClient, 'delete').mockResolvedValue(undefined);

		await controlApi.listOrganizations('app-1', 'production', { q: 'acme', limit: 25 });
		expect(getSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/production/organizations?q=acme&limit=25',
			undefined
		);

		await controlApi.createOrganization('app-1', 'production', {
			name: 'Acme Corp',
			slug: 'acme-corp'
		});
		expect(postSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/production/organizations',
			{ name: 'Acme Corp', slug: 'acme-corp' },
			undefined
		);

		await controlApi.createOrganizationInvitation('app-1', 'production', 'org-1', {
			email: 'member@acme.test',
			role_slug: 'org:admin',
			expires_in_days: 7
		});
		expect(postSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/production/organizations/org-1/invitations',
			{ email: 'member@acme.test', role_slug: 'org:admin', expires_in_days: 7 },
			undefined
		);

		await controlApi.createEnvironmentPermission('app-1', 'production', {
			slug: 'billing:invoices:write',
			description: 'Write invoices'
		});
		expect(postSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/production/permissions',
			{ slug: 'billing:invoices:write', description: 'Write invoices' },
			undefined
		);

		await controlApi.createEnvironmentRole('app-1', 'production', {
			slug: 'org:billing_admin',
			name: 'Billing Admin',
			scope: 'organization',
			permissions: ['billing:invoices:write']
		});
		expect(postSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/production/roles',
			{
				slug: 'org:billing_admin',
				name: 'Billing Admin',
				scope: 'organization',
				permissions: ['billing:invoices:write']
			},
			undefined
		);

		await controlApi.updateEnvironmentRole('app-1', 'production', 'role-1', {
			name: 'Senior Billing Admin',
			permissions: ['billing:invoices:write']
		});
		expect(patchSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/production/roles/role-1',
			{ name: 'Senior Billing Admin', permissions: ['billing:invoices:write'] },
			undefined
		);

		await controlApi.deleteEnvironmentRole('app-1', 'production', 'role-1');
		expect(deleteSpy).toHaveBeenCalledWith(
			'/v1/control/applications/app-1/environments/production/roles/role-1',
			undefined
		);

		getSpy.mockRestore();
		postSpy.mockRestore();
		patchSpy.mockRestore();
		deleteSpy.mockRestore();
	});
});

