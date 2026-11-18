export interface PortalBrandTheme {
	appName: string;
	primaryColor: string;
	logoUrl: string | null;
	supportEmail: string | null;
}

export interface ScopeClaimDescription {
	scope: string;
	title: string;
	description: string;
}

const HEX_COLOR_RE = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

export function resolvePortalBrandTheme(
	branding: Record<string, unknown> | null | undefined,
	fallbackAppName: string
): PortalBrandTheme {
	const rawName = typeof branding?.app_name === 'string' ? branding.app_name.trim() : '';
	const rawColor =
		typeof branding?.primary_color === 'string' ? branding.primary_color.trim() : '';
	const rawLogo = typeof branding?.logo_url === 'string' ? branding.logo_url.trim() : '';
	const rawSupport =
		typeof branding?.support_email === 'string' ? branding.support_email.trim() : '';

	const safeLogo =
		rawLogo.startsWith('https://') || rawLogo.startsWith('http://localhost') ? rawLogo : null;

	return {
		appName: rawName || fallbackAppName,
		primaryColor: HEX_COLOR_RE.test(rawColor) ? rawColor : '#6366f1',
		logoUrl: safeLogo,
		supportEmail: rawSupport || null
	};
}

export function describeOidcScopes(scopes: string[]): ScopeClaimDescription[] {
	const known: Record<string, { title: string; description: string }> = {
		openid: {
			title: 'Unique identity identifier',
			description: 'Verify your identity across applications using a stable account ID.'
		},
		email: {
			title: 'Email address',
			description: 'Read your primary verified email address and verification status.'
		},
		profile: {
			title: 'Basic profile',
			description: 'Read your display name and profile avatar.'
		},
		offline_access: {
			title: 'Stay signed in',
			description: 'Maintain your session in the background with rotated refresh tokens.'
		}
	};

	const seen = new Set<string>();
	const result: ScopeClaimDescription[] = [];
	for (const raw of scopes) {
		const scope = raw.trim();
		if (!scope || seen.has(scope)) continue;
		seen.add(scope);
		const info = known[scope] ?? {
			title: scope,
			description: `Grant access to the '${scope}' scope.`
		};
		result.push({ scope, title: info.title, description: info.description });
	}
	return result;
}

export function isGlobalEmailCollisionError(message: string): boolean {
	const lower = message.toLowerCase();
	return lower.includes('already exists') || lower.includes('email_already_exists');
}

