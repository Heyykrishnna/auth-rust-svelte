import { apiClient, type RequestOptions } from './client';
import type {
	AccountRecoveryStatus,
	BackupCodesResponse,
	DeviceInfo,
	LoginHistoryEntry,
	PasskeyInfo,
	PasskeyRegistrationOptions,
	SecurityAlertInfo,
	SecuritySummaryResponse,
	StepUpChallengeRequest,
	StepUpTokenResponse,
	ThreatIntelligenceReport,
	TotpEnrollResponse,
	UpdateSecuritySettingsRequest,
	UserSecuritySettings,
	VerifyPasskeyRegistrationRequest,
	VerifyTotpRequest
} from './types';

export const securityApi = {
	getSummary: (options?: RequestOptions) =>
		options
			? apiClient.get<SecuritySummaryResponse>('/api/users/security/summary', options)
			: apiClient.get<SecuritySummaryResponse>('/api/users/security/summary'),

	getSettings: (options?: RequestOptions) =>
		options
			? apiClient.get<UserSecuritySettings>('/api/users/security/settings', options)
			: apiClient.get<UserSecuritySettings>('/api/users/security/settings'),

	updateSettings: (data: UpdateSecuritySettingsRequest, options?: RequestOptions) =>
		options
			? apiClient.put<UserSecuritySettings>('/api/users/security/settings', data, options)
			: apiClient.put<UserSecuritySettings>('/api/users/security/settings', data),

	enrollTotp: (options?: RequestOptions) =>
		options
			? apiClient.post<TotpEnrollResponse>('/api/users/security/totp/enroll', {}, options)
			: apiClient.post<TotpEnrollResponse>('/api/users/security/totp/enroll', {}),

	confirmTotp: (data: VerifyTotpRequest, options?: RequestOptions) =>
		options
			? apiClient.post<{ success: boolean; message: string }>('/api/users/security/totp/confirm', data, options)
			: apiClient.post<{ success: boolean; message: string }>('/api/users/security/totp/confirm', data),

	disableTotp: (stepUpToken?: string, options?: RequestOptions) => {
		const headers = { ...options?.headers };
		if (stepUpToken) {
			headers['x-step-up-token'] = stepUpToken;
		}
		return apiClient.delete<void>('/api/users/security/totp', { ...options, headers });
	},

	getPasskeyOptions: (options?: RequestOptions) =>
		options
			? apiClient.get<PasskeyRegistrationOptions>('/api/users/security/passkeys/options', options)
			: apiClient.get<PasskeyRegistrationOptions>('/api/users/security/passkeys/options'),

	registerPasskey: (data: VerifyPasskeyRegistrationRequest, options?: RequestOptions) =>
		options
			? apiClient.post<PasskeyInfo>('/api/users/security/passkeys', data, options)
			: apiClient.post<PasskeyInfo>('/api/users/security/passkeys', data),

	listPasskeys: (options?: RequestOptions) =>
		options
			? apiClient.get<PasskeyInfo[]>('/api/users/security/passkeys', options)
			: apiClient.get<PasskeyInfo[]>('/api/users/security/passkeys'),

	deletePasskey: (passkeyId: string, options?: RequestOptions) =>
		options
			? apiClient.delete<void>(`/api/users/security/passkeys/${passkeyId}`, options)
			: apiClient.delete<void>(`/api/users/security/passkeys/${passkeyId}`),

	getBackupCodesStatus: (options?: RequestOptions) =>
		options
			? apiClient.get<{ remaining_count: number }>('/api/users/security/backup-codes/status', options)
			: apiClient.get<{ remaining_count: number }>('/api/users/security/backup-codes/status'),

	generateBackupCodes: (stepUpToken?: string, options?: RequestOptions) => {
		const headers = { ...options?.headers };
		if (stepUpToken) {
			headers['x-step-up-token'] = stepUpToken;
		}
		return apiClient.post<BackupCodesResponse>(
			'/api/users/security/backup-codes/generate',
			{},
			{ ...options, headers }
		);
	},

	verifyBackupCode: (code: string, options?: RequestOptions) =>
		options
			? apiClient.post<{ valid: boolean }>('/api/users/security/backup-codes/verify', { code }, options)
			: apiClient.post<{ valid: boolean }>('/api/users/security/backup-codes/verify', { code }),

	stepUp: (data: StepUpChallengeRequest, options?: RequestOptions) =>
		options
			? apiClient.post<StepUpTokenResponse>('/api/users/security/step-up', data, options)
			: apiClient.post<StepUpTokenResponse>('/api/users/security/step-up', data),

	listDevices: (options?: RequestOptions) =>
		options
			? apiClient.get<DeviceInfo[]>('/api/users/security/devices', options)
			: apiClient.get<DeviceInfo[]>('/api/users/security/devices'),

	trustDevice: (deviceId: string, options?: RequestOptions) =>
		options
			? apiClient.post<void>(`/api/users/security/devices/${deviceId}/trust`, {}, options)
			: apiClient.post<void>(`/api/users/security/devices/${deviceId}/trust`, {}),

	untrustDevice: (deviceId: string, options?: RequestOptions) =>
		options
			? apiClient.delete<void>(`/api/users/security/devices/${deviceId}/trust`, options)
			: apiClient.delete<void>(`/api/users/security/devices/${deviceId}/trust`),

	removeDevice: (deviceId: string, options?: RequestOptions) =>
		options
			? apiClient.delete<void>(`/api/users/security/devices/${deviceId}`, options)
			: apiClient.delete<void>(`/api/users/security/devices/${deviceId}`),

	listAlerts: (options?: RequestOptions) =>
		options
			? apiClient.get<SecurityAlertInfo[]>('/api/users/security/alerts', options)
			: apiClient.get<SecurityAlertInfo[]>('/api/users/security/alerts'),

	resolveAlert: (alertId: string, options?: RequestOptions) =>
		options
			? apiClient.post<void>(`/api/users/security/alerts/${alertId}/resolve`, {}, options)
			: apiClient.post<void>(`/api/users/security/alerts/${alertId}/resolve`, {}),

	getThreatIntelligence: (options?: RequestOptions) =>
		options
			? apiClient.get<ThreatIntelligenceReport>('/api/users/security/threat-intel', options)
			: apiClient.get<ThreatIntelligenceReport>('/api/users/security/threat-intel'),

	getLoginHistory: (limit?: number, options?: RequestOptions) => {
		const path = limit ? `/api/users/security/login-history?limit=${limit}` : '/api/users/security/login-history';
		return options
			? apiClient.get<LoginHistoryEntry[]>(path, options)
			: apiClient.get<LoginHistoryEntry[]>(path);
	},

	getRecoveryStatus: (options?: RequestOptions) =>
		options
			? apiClient.get<AccountRecoveryStatus>('/api/users/security/recovery', options)
			: apiClient.get<AccountRecoveryStatus>('/api/users/security/recovery'),

	revokeOtherSessions: (stepUpToken?: string, options?: RequestOptions) => {
		const headers = { ...options?.headers };
		if (stepUpToken) {
			headers['x-step-up-token'] = stepUpToken;
		}
		return apiClient.post<void>('/api/users/security/sessions/revoke-others', {}, { ...options, headers });
	}
};

