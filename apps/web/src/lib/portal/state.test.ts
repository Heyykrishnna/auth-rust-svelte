import { describe, expect, it } from 'vitest';
import {
	describeOidcScopes,
	isGlobalEmailCollisionError,
	resolvePortalBrandTheme
} from './state';

describe('Hosted Auth Portal state helpers', () => {
	it('resolves safe portal branding and rejects unsafe logo schemes', () => {
		const safe = resolvePortalBrandTheme(
			{
				app_name: 'Ferrum Cloud',
				primary_color: '#10b981',
				logo_url: 'https://cdn.ferrum.app/logo.svg',
				support_email: 'help@ferrum.app'
			},
			'Ferrum'
		);
		expect(safe.appName).toBe('Ferrum Cloud');
		expect(safe.primaryColor).toBe('#10b981');
		expect(safe.logoUrl).toBe('https://cdn.ferrum.app/logo.svg');
		expect(safe.supportEmail).toBe('help@ferrum.app');

		const unsafe = resolvePortalBrandTheme(
			{
				primary_color: 'javascript:alert(1)',
				logo_url: 'javascript:alert(1)'
			},
			'Elysiar'
		);
		expect(unsafe.appName).toBe('Elysiar');
		expect(unsafe.primaryColor).toBe('#6366f1');
		expect(unsafe.logoUrl).toBeNull();
	});

	it('deduplicates and describes requested OIDC scopes', () => {
		const claims = describeOidcScopes(['openid', 'email', 'profile', 'email', 'offline_access']);
		expect(claims.map((c) => c.scope)).toEqual(['openid', 'email', 'profile', 'offline_access']);
		expect(claims[1]?.title).toBe('Email address');
	});

	it('detects global email collision errors for smooth sign-in handoff', () => {
		expect(isGlobalEmailCollisionError('Email address already exists')).toBe(true);
		expect(isGlobalEmailCollisionError('EMAIL_ALREADY_EXISTS')).toBe(true);
		expect(isGlobalEmailCollisionError('Invalid verification code')).toBe(false);
	});
});

