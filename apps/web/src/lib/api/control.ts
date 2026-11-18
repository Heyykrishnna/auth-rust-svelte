import { apiClient, type RequestOptions } from './client';
import type {
	ApplicationDomainDto,
	ApplicationDto,
	ApplicationEnvironmentDto,
	ApplicationWithEnvironmentsDto,
	CreatedApplicationResponse,
	CreatedCredentialResponse,
	CredentialSummaryDto,
	CredentialType,
	DeveloperAuthResponse,
	DeveloperProfile,
	EnrollmentPolicy,
	EnvironmentKind,
	WorkspaceSummary
} from './types';

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

	listWorkspaces: (options?: RequestOptions) =>
		apiClient.get<WorkspaceSummary[]>('/v1/control/workspaces', options),

	createWorkspace: (payload: { name: string; slug: string }, options?: RequestOptions) =>
		apiClient.post<WorkspaceSummary>('/v1/control/workspaces', payload, options),

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
		},
		options?: RequestOptions
	) =>
		apiClient.patch<ApplicationEnvironmentDto>(
			`/v1/control/applications/${appId}/environments/${envKind}`,
			payload,
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
		)
};

