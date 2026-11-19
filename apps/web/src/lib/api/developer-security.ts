import { apiClient } from './client';

export interface DeveloperPasskey {
	id: string;
	label: string;
	created_at: string;
	last_used_at: string | null;
}

export interface DeveloperTrustedDevice {
	id: string;
	label: string;
	created_at: string;
	last_seen_at: string;
	expires_at: string;
	current: boolean;
}

export interface SecurityNotifications {
	notify_new_sign_in: boolean;
	notify_suspicious_sign_in: boolean;
}

export interface DeveloperProtection {
	verified_until: string | null;
	totp_enabled: boolean;
	backup_codes_remaining: number;
	passkeys_configured: boolean;
	passkeys: DeveloperPasskey[];
	trusted_devices: DeveloperTrustedDevice[];
	notifications: SecurityNotifications;
	delivery: { email_configured: boolean; recovery_configured: boolean };
	risk_coverage: {
		ip_reputation_configured: boolean;
		location_data_configured: boolean;
	};
	credential_stuffing_protection: boolean;
}

type CreationOptions = Parameters<
	typeof PublicKeyCredential.parseCreationOptionsFromJSON
>[0];
type RequestOptions = Parameters<
	typeof PublicKeyCredential.parseRequestOptionsFromJSON
>[0];

const root = '/v1/control/auth/security';
const idPath = (id: string) => encodeURIComponent(id);

export function supportsPasskeys() {
	return (
		typeof PublicKeyCredential !== 'undefined' &&
		typeof PublicKeyCredential.parseCreationOptionsFromJSON === 'function' &&
		typeof PublicKeyCredential.parseRequestOptionsFromJSON === 'function' &&
		typeof PublicKeyCredential.prototype.toJSON === 'function'
	);
}

export const developerSecurityApi = {
	verifyPassword: (password: string, secondFactor?: string) =>
		apiClient.post<{ verified_until: string }>(`${root}/step-up`, {
			password,
			second_factor_code: secondFactor
		}),
	async verifyPasskey() {
		const { publicKey } = await apiClient.post<{ publicKey: RequestOptions }>(
			`${root}/step-up/passkey/start`
		);
		const credential = await navigator.credentials.get({
			publicKey: PublicKeyCredential.parseRequestOptionsFromJSON(publicKey)
		});
		if (!(credential instanceof PublicKeyCredential))
			throw new Error('Passkey verification was cancelled.');
		return apiClient.post<{ verified_until: string }>(
			`${root}/step-up/passkey/finish`,
			credential.toJSON()
		);
	},
	async registerPasskey(label: string) {
		const { publicKey } = await apiClient.post<{ publicKey: CreationOptions }>(
			`${root}/passkeys/start`
		);
		const credential = await navigator.credentials.create({
			publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(publicKey)
		});
		if (!(credential instanceof PublicKeyCredential))
			throw new Error('Passkey registration was cancelled.');
		await apiClient.post<void>(`${root}/passkeys/finish`, {
			label,
			credential: credential.toJSON()
		});
	},
	removePasskey: (id: string) =>
		apiClient.delete<void>(`${root}/passkeys/${idPath(id)}`),
	beginTotp: () =>
		apiClient.post<{ secret: string; uri: string }>(`${root}/totp/start`),
	confirmTotp: (code: string) =>
		apiClient.post<{ codes: string[] }>(`${root}/totp/finish`, { code }),
	removeAuthenticator: () => apiClient.delete<void>(`${root}/totp`),
	rotateBackupCodes: () =>
		apiClient.post<{ codes: string[] }>(`${root}/backup-codes`),
	trustBrowser: (label: string) =>
		apiClient.post<void>(`${root}/devices`, { label }),
	forgetBrowser: (id: string) =>
		apiClient.delete<void>(`${root}/devices/${idPath(id)}`),
	setNotifications: (preferences: SecurityNotifications) =>
		apiClient.patch<void>(`${root}/notifications`, preferences),
	requestRecovery: (email: string) =>
		apiClient.post<void>('/v1/control/auth/forgot-password', { email }),
	resetPassword: (token: string, newPassword: string) =>
		apiClient.post<void>('/v1/control/auth/reset-password', {
			token,
			new_password: newPassword
		})
};
