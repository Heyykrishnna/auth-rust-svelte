import type { EnvironmentKind } from '$lib/api/types';

export function validateSlug(raw: string): string | null {
	const slug = raw.trim().toLowerCase();
	if (slug.length < 2 || slug.length > 64) {
		return 'Slug must be between 2 and 64 characters';
	}
	if (!/^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/.test(slug)) {
		return 'Slug may only contain lowercase letters, digits, and hyphens';
	}
	return null;
}

export function validateDomainForEnv(kind: EnvironmentKind, rawDomain: string): string | null {
	const domain = rawDomain.trim().toLowerCase();
	if (!domain) {
		return 'Domain is required';
	}
	if (domain.includes('://') || domain.includes('/') || domain.includes('*') || domain.includes('@')) {
		return 'Enter a bare hostname without scheme, path, wildcard, or credentials';
	}
	const hostOnly = domain.split(':')[0] ?? '';
	const isLocal = hostOnly === 'localhost' || hostOnly === '127.0.0.1';

	if (kind === 'production') {
		if (isLocal || hostOnly === '0.0.0.0' || domain.includes(':')) {
			return 'Production domains must be public hostnames without localhost or ports';
		}
		if (!hostOnly.includes('.')) {
			return 'Production domain must be a fully qualified domain name';
		}
		return null;
	}

	if (!isLocal && !hostOnly.includes('.')) {
		return 'Domain must be localhost, 127.0.0.1, or a valid hostname';
	}
	return null;
}

export function validateOriginForEnv(kind: EnvironmentKind, rawOrigin: string): string | null {
	const origin = rawOrigin.trim().replace(/\/+$/, '');
	if (!origin) {
		return 'Origin is required';
	}
	if (origin.includes('*') || origin.includes('@') || origin.includes('#') || origin.includes('?')) {
		return 'Origin must not contain wildcards, credentials, query strings, or fragments';
	}
	let parsed: URL;
	try {
		parsed = new URL(origin);
	} catch {
		return 'Origin must be a valid http:// or https:// URL';
	}
	if (parsed.pathname !== '/' && parsed.pathname !== '') {
		return 'Origin must not include a path';
	}
	const host = parsed.hostname.toLowerCase();
	const isLocal = host === 'localhost' || host === '127.0.0.1';

	if (kind === 'production') {
		if (parsed.protocol !== 'https:') {
			return 'Production origins must use https://';
		}
		if (isLocal || host === '0.0.0.0') {
			return 'Production origins cannot target localhost or loopback addresses';
		}
		return null;
	}

	if (parsed.protocol === 'http:' && !isLocal) {
		return 'Development http:// origins are restricted to localhost or 127.0.0.1';
	}
	if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
		return 'Origin scheme must be http or https';
	}
	return null;
}

export function validateRedirectUriForEnv(kind: EnvironmentKind, rawUri: string): string | null {
	const uri = rawUri.trim();
	if (!uri) {
		return 'Redirect URI is required';
	}
	if (uri.includes('*') || uri.includes('#') || uri.includes(' ')) {
		return 'Redirect URI must be an exact URI without wildcards, fragments, or spaces';
	}
	if (uri.includes('/../') || uri.endsWith('/..') || uri.includes('/./')) {
		return 'Redirect URI must not contain relative path traversal segments';
	}
	let parsed: URL;
	try {
		parsed = new URL(uri);
	} catch {
		return 'Redirect URI must be a valid URL';
	}
	if (parsed.username || parsed.password) {
		return 'Redirect URI must not contain embedded credentials';
	}
	return validateOriginForEnv(kind, parsed.origin);
}

export function maskSecretPrefix(prefix: string): string {
	return `${prefix}••••••••••••••••••••••••`;
}


export function parseLineList(raw: string): string[] {
	return raw
		.split(/[\n,]/)
		.map((s) => s.trim())
		.filter((s) => s.length > 0);
}
