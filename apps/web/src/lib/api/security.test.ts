import { describe, it, expect, vi, beforeEach } from 'vitest';
import { securityApi } from './security';
import { apiClient } from './client';

vi.mock('./client', () => ({
	apiClient: {
		get: vi.fn(),
		post: vi.fn(),
		put: vi.fn(),
		delete: vi.fn()
	}
}));

describe('securityApi', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('calls getSummary endpoint', async () => {
		await securityApi.getSummary();
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/security/summary');
	});

	it('calls getSettings endpoint', async () => {
		await securityApi.getSettings();
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/security/settings');
	});

	it('calls updateSettings with payload', async () => {
		const payload = {
			recovery_email: 'recovery@example.com',
			notify_new_login: true,
			notify_suspicious_activity: true
		};
		await securityApi.updateSettings(payload);
		expect(apiClient.put).toHaveBeenCalledWith('/api/users/security/settings', payload);
	});

	it('calls enrollTotp endpoint', async () => {
		await securityApi.enrollTotp();
		expect(apiClient.post).toHaveBeenCalledWith('/api/users/security/totp/enroll', {});
	});

	it('calls confirmTotp with verification code', async () => {
		await securityApi.confirmTotp({ code: '123456' });
		expect(apiClient.post).toHaveBeenCalledWith('/api/users/security/totp/confirm', { code: '123456' });
	});

	it('calls disableTotp with step-up token header', async () => {
		await securityApi.disableTotp('step-up-sample-token');
		expect(apiClient.delete).toHaveBeenCalledWith('/api/users/security/totp', {
			headers: { 'x-step-up-token': 'step-up-sample-token' }
		});
	});

	it('calls getPasskeyOptions endpoint', async () => {
		await securityApi.getPasskeyOptions();
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/security/passkeys/options');
	});

	it('calls registerPasskey with credential data', async () => {
		const payload = {
			credential_id: 'cred_test_123',
			public_key_spki: 'spki_test_123',
			name: 'MacBook Touch ID'
		};
		await securityApi.registerPasskey(payload);
		expect(apiClient.post).toHaveBeenCalledWith('/api/users/security/passkeys', payload);
	});

	it('calls listPasskeys endpoint', async () => {
		await securityApi.listPasskeys();
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/security/passkeys');
	});

	it('calls deletePasskey with passkey id', async () => {
		await securityApi.deletePasskey('passkey-id-123');
		expect(apiClient.delete).toHaveBeenCalledWith('/api/users/security/passkeys/passkey-id-123');
	});

	it('calls getBackupCodesStatus endpoint', async () => {
		await securityApi.getBackupCodesStatus();
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/security/backup-codes/status');
	});

	it('calls generateBackupCodes with step-up header', async () => {
		await securityApi.generateBackupCodes('token-xyz');
		expect(apiClient.post).toHaveBeenCalledWith(
			'/api/users/security/backup-codes/generate',
			{},
			{ headers: { 'x-step-up-token': 'token-xyz' } }
		);
	});

	it('calls verifyBackupCode with code payload', async () => {
		await securityApi.verifyBackupCode('ABCD-1234');
		expect(apiClient.post).toHaveBeenCalledWith('/api/users/security/backup-codes/verify', { code: 'ABCD-1234' });
	});

	it('calls stepUp with challenge credentials', async () => {
		const payload = { password: 'SuperSecretPassword123!' };
		await securityApi.stepUp(payload);
		expect(apiClient.post).toHaveBeenCalledWith('/api/users/security/step-up', payload);
	});

	it('calls listDevices endpoint', async () => {
		await securityApi.listDevices();
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/security/devices');
	});

	it('calls trustDevice with device id', async () => {
		await securityApi.trustDevice('device-id-123');
		expect(apiClient.post).toHaveBeenCalledWith('/api/users/security/devices/device-id-123/trust', {});
	});

	it('calls untrustDevice with device id', async () => {
		await securityApi.untrustDevice('device-id-123');
		expect(apiClient.delete).toHaveBeenCalledWith('/api/users/security/devices/device-id-123/trust');
	});

	it('calls removeDevice with device id', async () => {
		await securityApi.removeDevice('device-id-123');
		expect(apiClient.delete).toHaveBeenCalledWith('/api/users/security/devices/device-id-123');
	});

	it('calls listAlerts endpoint', async () => {
		await securityApi.listAlerts();
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/security/alerts');
	});

	it('calls resolveAlert with alert id', async () => {
		await securityApi.resolveAlert('alert-id-123');
		expect(apiClient.post).toHaveBeenCalledWith('/api/users/security/alerts/alert-id-123/resolve', {});
	});

	it('calls getThreatIntelligence endpoint', async () => {
		await securityApi.getThreatIntelligence();
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/security/threat-intel');
	});

	it('calls getLoginHistory with limit parameter', async () => {
		await securityApi.getLoginHistory(20);
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/security/login-history?limit=20');
	});

	it('calls getRecoveryStatus endpoint', async () => {
		await securityApi.getRecoveryStatus();
		expect(apiClient.get).toHaveBeenCalledWith('/api/users/security/recovery');
	});

	it('calls revokeOtherSessions with step-up token header', async () => {
		await securityApi.revokeOtherSessions('step-up-token-456');
		expect(apiClient.post).toHaveBeenCalledWith(
			'/api/users/security/sessions/revoke-others',
			{},
			{ headers: { 'x-step-up-token': 'step-up-token-456' } }
		);
	});
});

