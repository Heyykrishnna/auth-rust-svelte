import { apiClient, type RequestOptions } from './client';
import type {
	AnalyticsWindow,
	ApplicationDomainDto,
	ApplicationDto,
	ApplicationEnvironmentDto,
	ApplicationWithEnvironmentsDto,
	CreatedApplicationResponse,
	CreatedCredentialResponse,
	CreatedInvitationDto,
	CreatedWebhookEndpointDto,
	CredentialSummaryDto,
	CredentialType,
	DeveloperAuthResponse,
	DeveloperProfile,
	EnrollmentPolicy,
	EnvironmentAnalyticsDto,
	EnvironmentKind,
	EnvironmentOverviewDto,
	EnvironmentPermissionDto,
	EnvironmentRoleDto,
	EnvironmentUserDetailDto,
	OAuthProviderConfigDto,
	OrganizationDto,
	OrganizationInvitationDto,
	OrganizationMemberDto,
	PaginatedAuditLogsDto,
	PaginatedEnvironmentUsersDto,
	PaginatedOrganizationsDto,
	RotatedWebhookSecretDto,
	WebhookDeliveryDto,
	WebhookEndpointViewDto,
	WorkspaceMemberDto,
	WorkspaceRole,
	WorkspaceSummary
} from './types';

function toQueryString(params: Record<string, string | number | boolean | undefined | null>): string {
	const search = new URLSearchParams();
	for (const [key, value] of Object.entries(params)) {
		if (value !== undefined && value !== null && value !== '') {
			search.set(key, String(value));
		}
	}
	const str = search.toString();
	return str ? `?${str}` : '';
}

export const controlApi = {
	signUp: (
		payload: { email: string; password: string; display_name: string; workspace_name?: string },
		options?: RequestOptions
	) => apiClient.post<DeveloperAuthResponse>('/v1/control/auth/sign-up', payload, options),

	signIn: (payload: { email: string; password: string }, options?: RequestOptions) =>
		apiClient.post<DeveloperAuthResponse>('/v1/control/auth/sign-in', payload, options),

	signOut: (options?: RequestOptions) =>
		apiClient.post<{ message: string }>('/v1/control/auth/sign-out', undefined, options),

	me: (options?: RequestOptions) =>
		apiClient.get<DeveloperProfile>('/v1/control/auth/me', options),

	changePassword: (
		payload: { current_password: string; new_password: string },
		options?: RequestOptions
	) => apiClient.post<{ message: string }>('/v1/control/auth/change-password', payload, options),

	listWorkspaces: (options?: RequestOptions) =>
		apiClient.get<WorkspaceSummary[]>('/v1/control/workspaces', options),

	createWorkspace: (payload: { name: string; slug: string }, options?: RequestOptions) =>
		apiClient.post<WorkspaceSummary>('/v1/control/workspaces', payload, options),

	listWorkspaceMembers: (workspaceId: string, options?: RequestOptions) =>
		apiClient.get<WorkspaceMemberDto[]>(`/v1/control/workspaces/${workspaceId}/members`, options),

	addWorkspaceMember: (
		workspaceId: string,
		payload: { email: string; role: WorkspaceRole },
		options?: RequestOptions
	) =>
		apiClient.post<WorkspaceMemberDto>(
			`/v1/control/workspaces/${workspaceId}/members`,
			payload,
			options
		),

	updateWorkspaceMemberRole: (
		workspaceId: string,
		memberId: string,
		role: WorkspaceRole,
		options?: RequestOptions
	) =>
		apiClient.patch<WorkspaceMemberDto>(
			`/v1/control/workspaces/${workspaceId}/members/${memberId}`,
			{ role },
			options
		),

	removeWorkspaceMember: (workspaceId: string, memberId: string, options?: RequestOptions) =>
		apiClient.delete<void>(`/v1/control/workspaces/${workspaceId}/members/${memberId}`, options),

	listApplications: (workspaceId?: string, options?: RequestOptions) => {
		const path = workspaceId
			? `/v1/control/applications?workspace_id=${encodeURIComponent(workspaceId)}`
			: '/v1/control/applications';
		return apiClient.get<ApplicationWithEnvironmentsDto[]>(path, options);
	},

	createApplication: (
		payload: {
			workspace_id: string;
			name: string;
			slug: string;
			initial_dev_redirect_uri?: string;
		},
		options?: RequestOptions
	) => apiClient.post<CreatedApplicationResponse>('/v1/control/applications', payload, options),

	updateApplication: (
		appId: string,
		payload: { name?: string; status?: 'active' | 'suspended'; metadata?: Record<string, unknown> },
		options?: RequestOptions
	) => apiClient.patch<ApplicationDto>(`/v1/control/applications/${appId}`, payload, options),

	deleteApplication: (appId: string, options?: RequestOptions) =>
		apiClient.delete<void>(`/v1/control/applications/${appId}`, options),

	getOverview: (appId: string, envKind: EnvironmentKind, options?: RequestOptions) =>
		apiClient.get<EnvironmentOverviewDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/overview`,
			options
		),

	updateEnvironment: (
		appId: string,
		envKind: EnvironmentKind,
		payload: {
			enrollment_policy?: EnrollmentPolicy;
			session_ttl_secs?: number;
			access_token_ttl_secs?: number;
			require_email_verification?: boolean;
			require_mfa?: boolean;
			branding?: Record<string, unknown>;
			settings?: Record<string, unknown>;
		},
		options?: RequestOptions
	) =>
		apiClient.patch<ApplicationEnvironmentDto>(
			`/v1/control/applications/${appId}/environments/${envKind}`,
			payload,
			options
		),

	listOAuthProviders: (appId: string, envKind: EnvironmentKind, options?: RequestOptions) =>
		apiClient.get<OAuthProviderConfigDto[]>(
			`/v1/control/applications/${appId}/environments/${envKind}/oauth-providers`,
			options
		),

	upsertOAuthProvider: (
		appId: string,
		envKind: EnvironmentKind,
		provider: 'google' | 'github',
		payload: {
			enabled: boolean;
			use_platform_credentials: boolean;
			client_id?: string | null;
			client_secret?: string | null;
			scopes?: string[];
		},
		options?: RequestOptions
	) =>
		apiClient.put<OAuthProviderConfigDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/oauth-providers/${provider}`,
			payload,
			options
		),

	listUsers: (
		appId: string,
		envKind: EnvironmentKind,
		query: {
			q?: string;
			status?: string;
			email_verified?: boolean;
			role?: string;
			limit?: number;
			offset?: number;
		} = {},
		options?: RequestOptions
	) =>
		apiClient.get<PaginatedEnvironmentUsersDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/users${toQueryString(query)}`,
			options
		),

	getUserDetail: (
		appId: string,
		envKind: EnvironmentKind,
		appUserId: string,
		options?: RequestOptions
	) =>
		apiClient.get<EnvironmentUserDetailDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/users/${appUserId}`,
			options
		),

	updateUserMetadata: (
		appId: string,
		envKind: EnvironmentKind,
		appUserId: string,
		payload: {
			external_id?: string | null;
			public_metadata?: Record<string, unknown>;
			private_metadata?: Record<string, unknown>;
		},
		options?: RequestOptions
	) =>
		apiClient.patch<Record<string, unknown>>(
			`/v1/control/applications/${appId}/environments/${envKind}/users/${appUserId}/metadata`,
			payload,
			options
		),

	updateUserRole: (
		appId: string,
		envKind: EnvironmentKind,
		appUserId: string,
		payload: { role: string; action: 'assign' | 'revoke' },
		options?: RequestOptions
	) =>
		apiClient.post<{ roles: string[] }>(
			`/v1/control/applications/${appId}/environments/${envKind}/users/${appUserId}/roles`,
			payload,
			options
		),

	suspendUser: (
		appId: string,
		envKind: EnvironmentKind,
		appUserId: string,
		reason?: string,
		options?: RequestOptions
	) =>
		apiClient.post<Record<string, unknown>>(
			`/v1/control/applications/${appId}/environments/${envKind}/users/${appUserId}/suspend`,
			reason ? { reason } : {},
			options
		),

	restoreUser: (
		appId: string,
		envKind: EnvironmentKind,
		appUserId: string,
		options?: RequestOptions
	) =>
		apiClient.post<Record<string, unknown>>(
			`/v1/control/applications/${appId}/environments/${envKind}/users/${appUserId}/restore`,
			{},
			options
		),

	triggerPasswordReset: (
		appId: string,
		envKind: EnvironmentKind,
		appUserId: string,
		options?: RequestOptions
	) =>
		apiClient.post<{ status: string; email: string }>(
			`/v1/control/applications/${appId}/environments/${envKind}/users/${appUserId}/password-reset`,
			{},
			options
		),

	revokeAllUserSessions: (
		appId: string,
		envKind: EnvironmentKind,
		appUserId: string,
		options?: RequestOptions
	) =>
		apiClient.post<{ revoked_sessions: number }>(
			`/v1/control/applications/${appId}/environments/${envKind}/users/${appUserId}/sessions/revoke-all`,
			{},
			options
		),

	revokeSession: (
		appId: string,
		envKind: EnvironmentKind,
		sessionId: string,
		options?: RequestOptions
	) =>
		apiClient.delete<Record<string, unknown>>(
			`/v1/control/applications/${appId}/environments/${envKind}/sessions/${sessionId}`,
			options
		),

	listCredentials: (appId: string, envKind: EnvironmentKind, options?: RequestOptions) =>
		apiClient.get<CredentialSummaryDto[]>(
			`/v1/control/applications/${appId}/environments/${envKind}/credentials`,
			options
		),

	createCredential: (
		appId: string,
		envKind: EnvironmentKind,
		payload: { credential_type: CredentialType; name: string; scopes?: string[] },
		options?: RequestOptions
	) =>
		apiClient.post<CreatedCredentialResponse>(
			`/v1/control/applications/${appId}/environments/${envKind}/credentials`,
			payload,
			options
		),

	rotateCredential: (
		appId: string,
		envKind: EnvironmentKind,
		credId: string,
		graceWindowSecs = 0,
		options?: RequestOptions
	) =>
		apiClient.post<CreatedCredentialResponse>(
			`/v1/control/applications/${appId}/environments/${envKind}/credentials/${credId}/rotate`,
			{ grace_window_secs: graceWindowSecs },
			options
		),

	revokeCredential: (
		appId: string,
		envKind: EnvironmentKind,
		credId: string,
		options?: RequestOptions
	) =>
		apiClient.delete<void>(
			`/v1/control/applications/${appId}/environments/${envKind}/credentials/${credId}`,
			options
		),

	listDomains: (appId: string, envKind: EnvironmentKind, options?: RequestOptions) =>
		apiClient.get<ApplicationDomainDto[]>(
			`/v1/control/applications/${appId}/environments/${envKind}/domains`,
			options
		),

	createDomain: (
		appId: string,
		envKind: EnvironmentKind,
		payload: {
			domain: string;
			allowed_origins: string[];
			redirect_uris: string[];
			post_logout_redirect_uris?: string[];
		},
		options?: RequestOptions
	) =>
		apiClient.post<ApplicationDomainDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/domains`,
			payload,
			options
		),

	deleteDomain: (
		appId: string,
		envKind: EnvironmentKind,
		domainId: string,
		options?: RequestOptions
	) =>
		apiClient.delete<void>(
			`/v1/control/applications/${appId}/environments/${envKind}/domains/${domainId}`,
			options
		),

	listWebhooks: (appId: string, envKind: EnvironmentKind, options?: RequestOptions) =>
		apiClient.get<WebhookEndpointViewDto[]>(
			`/v1/control/applications/${appId}/environments/${envKind}/webhooks`,
			options
		),

	createWebhook: (
		appId: string,
		envKind: EnvironmentKind,
		payload: { url: string; description?: string; subscribed_events: string[] },
		options?: RequestOptions
	) =>
		apiClient.post<CreatedWebhookEndpointDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/webhooks`,
			payload,
			options
		),

	updateWebhook: (
		appId: string,
		envKind: EnvironmentKind,
		endpointId: string,
		payload: {
			url?: string;
			description?: string;
			subscribed_events?: string[];
			status?: 'active' | 'paused' | 'disabled';
		},
		options?: RequestOptions
	) =>
		apiClient.patch<WebhookEndpointViewDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/webhooks/${endpointId}`,
			payload,
			options
		),

	deleteWebhook: (
		appId: string,
		envKind: EnvironmentKind,
		endpointId: string,
		options?: RequestOptions
	) =>
		apiClient.delete<void>(
			`/v1/control/applications/${appId}/environments/${envKind}/webhooks/${endpointId}`,
			options
		),

	rotateWebhookSecret: (
		appId: string,
		envKind: EnvironmentKind,
		endpointId: string,
		graceHours = 24,
		options?: RequestOptions
	) =>
		apiClient.post<RotatedWebhookSecretDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/webhooks/${endpointId}/rotate-secret`,
			{ grace_hours: graceHours },
			options
		),

	sendTestWebhook: (
		appId: string,
		envKind: EnvironmentKind,
		endpointId: string,
		options?: RequestOptions
	) =>
		apiClient.post<WebhookDeliveryDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/webhooks/${endpointId}/test`,
			{},
			options
		),

	listWebhookDeliveries: (
		appId: string,
		envKind: EnvironmentKind,
		endpointId: string,
		limit = 50,
		options?: RequestOptions
	) =>
		apiClient.get<WebhookDeliveryDto[]>(
			`/v1/control/applications/${appId}/environments/${envKind}/webhooks/${endpointId}/deliveries${toQueryString({ limit })}`,
			options
		),

	replayWebhookDelivery: (
		appId: string,
		envKind: EnvironmentKind,
		endpointId: string,
		deliveryId: string,
		options?: RequestOptions
	) =>
		apiClient.post<WebhookDeliveryDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/webhooks/${endpointId}/deliveries/${deliveryId}/replay`,
			{},
			options
		),

	replayWebhookRange: (
		appId: string,
		envKind: EnvironmentKind,
		endpointId: string,
		payload: { from: string; to: string; event_types?: string[] },
		options?: RequestOptions
	) =>
		apiClient.post<{ replayed_count: number }>(
			`/v1/control/applications/${appId}/environments/${envKind}/webhooks/${endpointId}/replay`,
			payload,
			options
		),

	listAuditLogs: (
		appId: string,
		envKind: EnvironmentKind,
		query: {
			event?: string;
			actor_type?: string;
			resource_type?: string;
			q?: string;
			from?: string;
			to?: string;
			limit?: number;
			offset?: number;
		} = {},
		options?: RequestOptions
	) =>
		apiClient.get<PaginatedAuditLogsDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/audit-logs${toQueryString(query)}`,
			options
		),

	listOrganizations: (
		appId: string,
		envKind: EnvironmentKind,
		query: { q?: string; cursor?: string; limit?: number } = {},
		options?: RequestOptions
	) =>
		apiClient.get<PaginatedOrganizationsDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations${toQueryString(query)}`,
			options
		),

	createOrganization: (
		appId: string,
		envKind: EnvironmentKind,
		payload: { name: string; slug: string; logo_url?: string; metadata?: Record<string, unknown> },
		options?: RequestOptions
	) =>
		apiClient.post<OrganizationDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations`,
			payload,
			options
		),

	updateOrganization: (
		appId: string,
		envKind: EnvironmentKind,
		orgId: string,
		payload: {
			name?: string;
			slug?: string;
			logo_url?: string | null;
			status?: 'active' | 'suspended';
			metadata?: Record<string, unknown>;
		},
		options?: RequestOptions
	) =>
		apiClient.patch<OrganizationDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations/${orgId}`,
			payload,
			options
		),

	deleteOrganization: (
		appId: string,
		envKind: EnvironmentKind,
		orgId: string,
		options?: RequestOptions
	) =>
		apiClient.delete<void>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations/${orgId}`,
			options
		),

	listOrganizationMembers: (
		appId: string,
		envKind: EnvironmentKind,
		orgId: string,
		options?: RequestOptions
	) =>
		apiClient.get<OrganizationMemberDto[]>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations/${orgId}/members`,
			options
		),

	addOrganizationMember: (
		appId: string,
		envKind: EnvironmentKind,
		orgId: string,
		payload: { application_user_id: string; role_slug?: string },
		options?: RequestOptions
	) =>
		apiClient.post<OrganizationMemberDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations/${orgId}/members`,
			payload,
			options
		),

	updateOrganizationMemberRole: (
		appId: string,
		envKind: EnvironmentKind,
		orgId: string,
		memberId: string,
		roleSlug: string,
		options?: RequestOptions
	) =>
		apiClient.patch<OrganizationMemberDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations/${orgId}/members/${memberId}`,
			{ role_slug: roleSlug },
			options
		),

	removeOrganizationMember: (
		appId: string,
		envKind: EnvironmentKind,
		orgId: string,
		memberId: string,
		options?: RequestOptions
	) =>
		apiClient.delete<void>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations/${orgId}/members/${memberId}`,
			options
		),

	listOrganizationInvitations: (
		appId: string,
		envKind: EnvironmentKind,
		orgId: string,
		options?: RequestOptions
	) =>
		apiClient.get<OrganizationInvitationDto[]>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations/${orgId}/invitations`,
			options
		),

	createOrganizationInvitation: (
		appId: string,
		envKind: EnvironmentKind,
		orgId: string,
		payload: { email: string; role_slug?: string; expires_in_days?: number },
		options?: RequestOptions
	) =>
		apiClient.post<CreatedInvitationDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations/${orgId}/invitations`,
			payload,
			options
		),

	revokeOrganizationInvitation: (
		appId: string,
		envKind: EnvironmentKind,
		orgId: string,
		invitationId: string,
		options?: RequestOptions
	) =>
		apiClient.delete<void>(
			`/v1/control/applications/${appId}/environments/${envKind}/organizations/${orgId}/invitations/${invitationId}`,
			options
		),

	listEnvironmentPermissions: (
		appId: string,
		envKind: EnvironmentKind,
		options?: RequestOptions
	) =>
		apiClient.get<EnvironmentPermissionDto[]>(
			`/v1/control/applications/${appId}/environments/${envKind}/permissions`,
			options
		),

	createEnvironmentPermission: (
		appId: string,
		envKind: EnvironmentKind,
		payload: { slug: string; description?: string },
		options?: RequestOptions
	) =>
		apiClient.post<EnvironmentPermissionDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/permissions`,
			payload,
			options
		),

	deleteEnvironmentPermission: (
		appId: string,
		envKind: EnvironmentKind,
		permissionId: string,
		options?: RequestOptions
	) =>
		apiClient.delete<void>(
			`/v1/control/applications/${appId}/environments/${envKind}/permissions/${permissionId}`,
			options
		),

	listEnvironmentRoles: (
		appId: string,
		envKind: EnvironmentKind,
		options?: RequestOptions
	) =>
		apiClient.get<EnvironmentRoleDto[]>(
			`/v1/control/applications/${appId}/environments/${envKind}/roles`,
			options
		),

	createEnvironmentRole: (
		appId: string,
		envKind: EnvironmentKind,
		payload: {
			slug: string;
			name: string;
			scope: 'application' | 'organization';
			description?: string;
			permissions: string[];
		},
		options?: RequestOptions
	) =>
		apiClient.post<EnvironmentRoleDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/roles`,
			payload,
			options
		),

	updateEnvironmentRole: (
		appId: string,
		envKind: EnvironmentKind,
		roleId: string,
		payload: { name?: string; description?: string; permissions?: string[] },
		options?: RequestOptions
	) =>
		apiClient.patch<EnvironmentRoleDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/roles/${roleId}`,
			payload,
			options
		),

	deleteEnvironmentRole: (
		appId: string,
		envKind: EnvironmentKind,
		roleId: string,
		options?: RequestOptions
	) =>
		apiClient.delete<void>(
			`/v1/control/applications/${appId}/environments/${envKind}/roles/${roleId}`,
			options
		),

	getAnalytics: (
		appId: string,
		envKind: EnvironmentKind,
		window: AnalyticsWindow = '7d',
		options?: RequestOptions
	) =>
		apiClient.get<EnvironmentAnalyticsDto>(
			`/v1/control/applications/${appId}/environments/${envKind}/analytics${toQueryString({ window })}`,
			options
		),

	refreshAnalytics: (
		appId: string,
		envKind: EnvironmentKind,
		window: AnalyticsWindow = '7d',
		options?: RequestOptions
	) =>
		apiClient.post<{ window: string; granularity: string; buckets_updated: number }>(
			`/v1/control/applications/${appId}/environments/${envKind}/analytics/refresh${toQueryString({ window })}`,
			{},
			options
		)
};
