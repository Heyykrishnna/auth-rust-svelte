import { describe, expect, it } from 'vitest';
import {
	maskSecretPrefix,
	validateDomainForEnv,
	validateOriginForEnv,
	validateRedirectUriForEnv,
	validateSlug
} from './validation';

describe('control plane validation helpers', () => {
	it('validates application slugs strictly', () => {
		expect(validateSlug('ferrum-app')).toBeNull();
		expect(validateSlug('a1')).toBeNull();
		expect(validateSlug('x')).not.toBeNull();
		expect(validateSlug('-leading')).not.toBeNull();
		expect(validateSlug('invalid_slug')).not.toBeNull();
	});

	it('enforces development vs production domain rules', () => {
		expect(validateDomainForEnv('development', 'localhost:5173')).toBeNull();
		expect(validateDomainForEnv('development', '127.0.0.1:3000')).toBeNull();
		expect(validateDomainForEnv('production', 'ferrum.app')).toBeNull();
		expect(validateDomainForEnv('production', 'localhost:5173')).not.toBeNull();
		expect(validateDomainForEnv('production', '*.ferrum.app')).not.toBeNull();
		expect(validateDomainForEnv('production', 'https://ferrum.app')).not.toBeNull();
	});

	it('enforces development vs production origin rules', () => {
		expect(validateOriginForEnv('development', 'http://localhost:5173')).toBeNull();
		expect(validateOriginForEnv('development', 'http://evil.com')).not.toBeNull();
		expect(validateOriginForEnv('production', 'https://ferrum.app')).toBeNull();
		expect(validateOriginForEnv('production', 'http://ferrum.app')).not.toBeNull();
		expect(validateOriginForEnv('production', 'https://localhost:3000')).not.toBeNull();
		expect(validateOriginForEnv('production', 'https://ferrum.app/path')).not.toBeNull();
	});

	it('rejects unsafe redirect URIs across environments', () => {
		expect(
			validateRedirectUriForEnv('development', 'http://localhost:5173/auth/callback')
		).toBeNull();
		expect(validateRedirectUriForEnv('production', 'https://ferrum.app/auth/callback')).toBeNull();
		expect(
			validateRedirectUriForEnv('production', 'https://ferrum.app/callback#fragment')
		).not.toBeNull();
		expect(
			validateRedirectUriForEnv('production', 'https://ferrum.app/../escape')
		).not.toBeNull();
		expect(
			validateRedirectUriForEnv('production', 'https://ferrum.app@evil.com/cb')
		).not.toBeNull();
		expect(validateRedirectUriForEnv('production', 'https://*.ferrum.app/cb')).not.toBeNull();
	});

	it('masks secret key prefixes without exposing secret entropy', () => {
		const masked = maskSecretPrefix('sk_live_');
		expect(masked.startsWith('sk_live_')).toBe(true);
		expect(masked.includes('•')).toBe(true);
	});
});

