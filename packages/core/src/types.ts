export interface EcPublicJwk {
	kty: 'EC';
	crv: 'P-256';
	use?: string;
	alg?: string;
	kid: string;
	x: string;
	y: string;
}

export interface JwksDocument {
	keys: EcPublicJwk[];
}

export interface OidcDiscoveryDocument {
	issuer: string;
	authorization_endpoint: string;
	token_endpoint: string;
	userinfo_endpoint: string;
	jwks_uri: string;
	end_session_endpoint: string;
	revocation_endpoint: string;
	response_types_supported: string[];
	grant_types_supported: string[];
	code_challenge_methods_supported: string[];
	id_token_signing_alg_values_supported: string[];
	scopes_supported: string[];
	claims_supported: string[];
}

export interface TenantAccessTokenClaims {
	iss: string;
	sub: string;
	aud: string;
	azp: string;
	app_user_id: string;
	sid: string;
	org_id?: string;
	org_slug?: string;
	org_role?: string;
	org_permissions?: string[];
	scope?: string;
	roles: string[];
	permissions: string[];
	amr: string[];
	acr: string;
	ver: number;
	iat: number;
	exp: number;
	nbf?: number;
	jti: string;
}

export interface IdTokenClaims {
	iss: string;
	sub: string;
	aud: string;
	nonce?: string;
	auth_time: number;
	amr: string[];
	acr: string;
	email: string;
	email_verified: boolean;
	name: string;
	picture?: string | null;
	app_user_id: string;
	ver: number;
	iat: number;
	exp: number;
	nbf?: number;
}

export interface ClientUser {
	sub: string;
	app_user_id: string;
	email: string;
	display_name: string;
	avatar_url: string | null;
	email_verified: boolean;
	status: string;
	roles: string[];
	permissions: string[];
	version: number;
}

export interface ClientAuthResponse {
	user: ClientUser;
	access_token: string;
	refresh_token: string;
	token_type: string;
	expires_in: number;
	session_id: string;
}

export interface ClientTokenRefreshResponse {
	access_token: string;
	refresh_token: string;
	token_type: string;
	expires_in: number;
	session_id: string;
}

export interface ClientSignUpInitiatedResponse {
	status: string;
	email: string;
	message: string;
}

export interface ClientSessionView {
	id: string;
	is_current: boolean;
	user_agent: string | null;
	ip_address: string | null;
	active_organization_id: string | null;
	created_at: string;
	last_used_at: string;
	expires_at: string;
}

export interface OidcTokenResponse {
	access_token: string;
	token_type: string;
	expires_in: number;
	scope: string;
	id_token?: string;
	refresh_token?: string;
}

export interface BackendUserSyncItem {
	sub: string;
	app_user_id: string;
	email: string;
	email_verified: boolean;
	display_name: string;
	avatar_url: string | null;
	status: string;
	roles: string[];
	public_metadata: Record<string, unknown>;
	private_metadata: Record<string, unknown>;
	external_id: string | null;
	version: number;
	updated_at: string;
}

export interface PaginatedBackendUsers {
	data: BackendUserSyncItem[];
	next_cursor: string | null;
	has_more: boolean;
}

export interface BackendEventItem {
	id: string;
	event_type: string;
	aggregate_type: string;
	aggregate_id: string;
	aggregate_version: number;
	payload: WebhookEventEnvelope;
	created_at: string;
}

export interface PaginatedBackendEvents {
	data: BackendEventItem[];
	next_event_id: string | null;
	has_more: boolean;
}

export interface BackendSessionStatus {
	session_id: string;
	environment_id: string;
	app_user_id: string;
	sub: string;
	status: 'active' | 'revoked' | 'expired';
	active_organization_id: string | null;
	last_used_at: string;
	expires_at: string;
	revoked_at: string | null;
	created_at: string;
}

export type WebhookEventType =
	| 'user.created'
	| 'user.updated'
	| 'user.deleted'
	| 'user.suspended'
	| 'user.email_verified'
	| 'membership.created'
	| 'membership.updated'
	| 'membership.revoked'
	| 'session.created'
	| 'session.revoked'
	| 'role.assigned'
	| 'role.revoked'
	| 'organization.created'
	| 'organization.updated'
	| 'organization.deleted'
	| 'organization.member_added'
	| 'organization.member_updated'
	| 'organization.member_removed'
	| 'webhook.test';

export interface WebhookEventEnvelope<TData = Record<string, unknown>> {
	spec_version: string;
	event_id: string;
	event_type: WebhookEventType | (string & {});
	environment_id?: string;
	occurred_at: string;
	aggregate_type: string;
	aggregate_id: string;
	aggregate_version: number;
	data: TData;
}

export interface JitUserProjection {
	sub: string;
	appUserId: string;
	email?: string;
	emailVerified?: boolean;
	displayName?: string;
	avatarUrl?: string | null;
	roles: string[];
	permissions: string[];
	identityVersion: number;
}

export interface ApiErrorBody {
	error?: {
		code?: string;
		message?: string;
		request_id?: string | null;
		details?: unknown;
	};
	status?: number;
}

export interface OrganizationView {
	id: string;
	environment_id: string;
	slug: string;
	name: string;
	logo_url: string | null;
	status: string;
	metadata: Record<string, unknown>;
	version: number;
	member_count: number;
	created_at: string;
	updated_at: string;
}

export interface UserOrganizationMembership {
	organization: OrganizationView;
	member_id: string;
	roles: string[];
	status: string;
	joined_at: string;
}

export interface OrganizationMemberView {
	id: string;
	organization_id: string;
	application_user_id: string;
	sub: string;
	email: string;
	display_name: string;
	status: string;
	roles: string[];
	version: number;
	created_at: string;
	updated_at: string;
}

export interface OrganizationInvitationView {
	id: string;
	organization_id: string;
	email: string;
	role_slug: string | null;
	status: string;
	expires_at: string;
	accepted_at: string | null;
	created_at: string;
}

export interface CreatedInvitationResponse {
	invitation: OrganizationInvitationView;
	invitation_token: string;
}

export interface ActiveOrganizationSwitchResponse {
	access_token: string;
	token_type: string;
	expires_in: number;
	active_organization_id: string | null;
	org_slug: string | null;
	org_role: string | null;
	org_permissions: string[];
}

export interface PaginatedBackendOrganizations {
	data: OrganizationView[];
	next_cursor: string | null;
	has_more: boolean;
}

export interface EnvironmentPermissionView {
	id: string;
	slug: string;
	description: string;
	created_at: string;
}

export interface EnvironmentRoleView {
	id: string;
	slug: string;
	name: string;
	scope: 'application' | 'organization' | string;
	description: string;
	is_system: boolean;
	permissions: string[];
	created_at: string;
	updated_at: string;
}

export interface UserEmailView {
	id: string;
	email: string;
	verified: boolean;
	verified_at: string | null;
	is_primary: boolean;
	created_at: string;
}

export interface LinkedOAuthAccountView {
	id: string;
	provider: string;
	email: string | null;
	email_verified: boolean;
	created_at: string;
}

export interface IdentityDataExportResponse {
	exported_at: string;
	user: Record<string, unknown>;
	emails: UserEmailView[];
	oauth_accounts: LinkedOAuthAccountView[];
	memberships: Record<string, unknown>[];
	organizations: Record<string, unknown>[];
	recent_audit_logs: Record<string, unknown>[];
}

export interface PermissionCheckInput {
	permission?: string;
	role?: string;
	orgId?: string;
}

