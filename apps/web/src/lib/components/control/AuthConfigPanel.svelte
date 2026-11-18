<script lang="ts">
	import SectionHeader from './SectionHeader.svelte';
	import Dropdown from '$lib/components/Dropdown.svelte';
	import AuthSplitShell from '$lib/components/AuthSplitShell.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import type {
		ApplicationEnvironmentDto,
		EnrollmentPolicy,
		EnvironmentKind,
		OAuthProviderConfigDto
	} from '$lib/api/types';

	interface Props {
		appId: string;
		appName: string;
		envKind: EnvironmentKind;
		environment: ApplicationEnvironmentDto;
		providers: OAuthProviderConfigDto[];
		canEditPolicies: boolean;
		canEditProviders: boolean;
		onSaved: () => Promise<void>;
	}

	let {
		appId,
		appName,
		envKind,
		environment,
		providers,
		canEditPolicies,
		canEditProviders,
		onSaved
	}: Props = $props();

	let enrollmentPolicy = $state<EnrollmentPolicy>('consent_required');
	let sessionTtlSecs = $state(604800);
	let accessTokenTtlSecs = $state(300);
	let requireEmailVerification = $state(true);
	let requireMfa = $state(false);
	let passwordLoginEnabled = $state(true);
	let minPasswordLength = $state(12);
	let requireUppercase = $state(true);
	let requireNumber = $state(true);

	let brandAppName = $state('');
	let brandLogoUrl = $state('');
	let brandSupportEmail = $state('');
	let previewTab = $state<'signin' | 'consent'>('signin');
	let savingPolicy = $state(false);

	let providerDrafts = $state<
		Record<
			string,
			{
				enabled: boolean;
				use_platform_credentials: boolean;
				client_id: string;
				client_secret: string;
				scopes: string;
			}
		>
	>({});
	let savingProvider = $state<string | null>(null);

	$effect(() => {
		enrollmentPolicy = environment.enrollment_policy;
		sessionTtlSecs = environment.session_ttl_secs;
		accessTokenTtlSecs = environment.access_token_ttl_secs;
		requireEmailVerification = environment.require_email_verification;
		requireMfa = environment.require_mfa;

		const settings = environment.settings ?? {};
		passwordLoginEnabled = settings['password_login_enabled'] !== false;
		minPasswordLength =
			typeof settings['min_password_length'] === 'number' ? settings['min_password_length'] : 12;
		requireUppercase = settings['password_require_uppercase'] !== false;
		requireNumber = settings['password_require_number'] !== false;

		const branding = environment.branding ?? {};
		brandAppName =
			typeof branding['app_name'] === 'string' && branding['app_name'].trim()
				? branding['app_name']
				: appName;
		brandLogoUrl = typeof branding['logo_url'] === 'string' ? branding['logo_url'] : '';
		brandSupportEmail =
			typeof branding['support_email'] === 'string' ? branding['support_email'] : '';
	});

	$effect(() => {
		const next: typeof providerDrafts = {};
		for (const p of providers) {
			next[p.provider] = {
				enabled: p.enabled,
				use_platform_credentials: p.use_platform_credentials,
				client_id: p.client_id ?? '',
				client_secret: '',
				scopes: p.scopes.join(', ')
			};
		}
		providerDrafts = next;
	});

	async function saveAuthAndBranding(e: SubmitEvent) {
		e.preventDefault();
		if (!canEditPolicies) return;
		savingPolicy = true;
		try {
			await controlApi.updateEnvironment(appId, envKind, {
				enrollment_policy: enrollmentPolicy,
				session_ttl_secs: Number(sessionTtlSecs),
				access_token_ttl_secs: Number(accessTokenTtlSecs),
				require_email_verification: requireEmailVerification,
				require_mfa: requireMfa,
				branding: {
					app_name: brandAppName.trim() || appName,
					logo_url: brandLogoUrl.trim() || null,
					support_email: brandSupportEmail.trim() || null
				},
				settings: {
					password_login_enabled: passwordLoginEnabled,
					min_password_length: Number(minPasswordLength),
					password_require_uppercase: requireUppercase,
					password_require_number: requireNumber
				}
			});
			addToast('Authentication, OIDC, and branding settings saved', 'success');
			await onSaved();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to save settings', 'error');
		} finally {
			savingPolicy = false;
		}
	}

	async function saveProvider(providerKey: 'google' | 'github') {
		if (!canEditProviders) return;
		const draft = providerDrafts[providerKey];
		if (!draft) return;
		savingProvider = providerKey;
		try {
			await controlApi.upsertOAuthProvider(appId, envKind, providerKey, {
				enabled: draft.enabled,
				use_platform_credentials: draft.use_platform_credentials,
				client_id: draft.use_platform_credentials ? null : draft.client_id.trim() || null,
				client_secret:
					!draft.use_platform_credentials && draft.client_secret.trim()
						? draft.client_secret.trim()
						: null,
				scopes: draft.scopes
					.split(',')
					.map((s) => s.trim())
					.filter(Boolean)
			});
			addToast(`${providerKey === 'google' ? 'Google' : 'GitHub'} OAuth provider updated`, 'success');
			await onSaved();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to update provider', 'error');
		} finally {
			savingProvider = null;
		}
	}

	async function copyText(value: string, label: string) {
		try {
			await navigator.clipboard.writeText(value);
			addToast(`${label} copied`, 'success');
		} catch {
			addToast('Copy failed', 'warning');
		}
	}
</script>

<section class="grid min-w-0 grid-cols-1 gap-5 sm:gap-6" aria-label="Authentication and OIDC configuration">
	<SectionHeader title="Authentication &amp; SSO" description="Configure sign-in methods, password rules, social OAuth, OIDC policies, and Hosted Portal branding.">
		{#if !canEditPolicies}
			<Badge variant="warning" size="sm">Admin role required for policy changes</Badge>
		{/if}
	</SectionHeader>

	<form class="two-col" onsubmit={saveAuthAndBranding}>
		<div class="surface-box">
			<h2 class="box-title">Methods &amp; OIDC Policy</h2>

			<div class="toggle-list">
				<label class="toggle-row">
					<div>
						<strong>Email &amp; Password Authentication</strong>
						<span>Allow password sign-in and self-service registration</span>
					</div>
					<input type="checkbox" bind:checked={passwordLoginEnabled} disabled={!canEditPolicies} />
				</label>

				<label class="toggle-row">
					<div>
						<strong>Require Email OTP Verification</strong>
						<span>Verify email ownership before issuing active sessions</span>
					</div>
					<input type="checkbox" bind:checked={requireEmailVerification} disabled={!canEditPolicies} />
				</label>

				<label class="toggle-row">
					<div>
						<strong>Require Multi-Factor Authentication</strong>
						<span>Enforce MFA policy flag for sensitive tenant sessions</span>
					</div>
					<input type="checkbox" bind:checked={requireMfa} disabled={!canEditPolicies} />
				</label>
			</div>

			<div class="grid-3">
				<div class="field">
					<span>Enrollment Policy</span>
					<Dropdown
						label="Enrollment policy"
						bind:value={enrollmentPolicy}
						disabled={!canEditPolicies}
						class="w-full"
						options={[
							{ value: 'consent_required', label: 'Consent required' },
							{ value: 'auto_join', label: 'Auto join' },
							{ value: 'invite_only', label: 'Invite only' }
						]}
						/>
				</div>

				<label class="field">
					<span>Access Token TTL (s)</span>
					<input type="number" min="60" max="86400" bind:value={accessTokenTtlSecs} disabled={!canEditPolicies} />
				</label>

				<label class="field">
					<span>Session TTL (s)</span>
					<input type="number" min="300" bind:value={sessionTtlSecs} disabled={!canEditPolicies} />
				</label>
			</div>

			<div class="grid-3">
				<label class="field">
					<span>Min Password Length</span>
					<input type="number" min="8" max="64" bind:value={minPasswordLength} disabled={!canEditPolicies} />
				</label>

				<label class="check-inline">
					<input type="checkbox" bind:checked={requireUppercase} disabled={!canEditPolicies} />
					<span>Require uppercase</span>
				</label>

				<label class="check-inline">
					<input type="checkbox" bind:checked={requireNumber} disabled={!canEditPolicies} />
					<span>Require number</span>
				</label>
			</div>

			<div class="form-actions">
				<Button type="submit" variant="primary" size="sm" loading={savingPolicy} disabled={!canEditPolicies || savingPolicy}>
					Save Auth &amp; Branding
				</Button>
			</div>
		</div>

		<div class="surface-box">
			<div class="box-header">
				<div>
					<h2 class="box-title">Hosted Portal Branding</h2>
					<p class="box-sub">Live monochrome preview of your OIDC sign-in &amp; consent portal</p>
				</div>
				<div class="seg-tabs">
					<button type="button" class:active={previewTab === 'signin'} onclick={() => (previewTab = 'signin')}>Sign In</button>
					<button type="button" class:active={previewTab === 'consent'} onclick={() => (previewTab = 'consent')}>Consent</button>
				</div>
			</div>

			<div class="grid-2">
				<label class="field">
					<span>Display Name</span>
					<input type="text" bind:value={brandAppName} disabled={!canEditPolicies} placeholder={appName} />
				</label>
				<label class="field">
					<span>Support Email</span>
					<input type="email" bind:value={brandSupportEmail} disabled={!canEditPolicies} placeholder="security@company.com" />
				</label>
			</div>

			<label class="field">
				<span>Logo URL (Optional)</span>
				<input type="url" bind:value={brandLogoUrl} disabled={!canEditPolicies} placeholder="https://example.com/logo.svg" />
			</label>

			<div class="preview-frame" aria-label="Hosted portal live preview">
				<AuthSplitShell
					compact
					brandName={brandAppName.trim() || appName}
					logoUrl={brandLogoUrl.trim() || null}
					footerText={brandSupportEmail.trim() ? `Support: ${brandSupportEmail.trim()}` : 'Central Identity Provider'}
					topActionLabel={previewTab === 'signin' ? 'Create an account' : 'Switch account'}
				>
					{#if previewTab === 'signin'}
						<div class="preview-heading">Login</div>
						<div class="preview-inputs">
							<div class="preview-line"><span>Email</span><strong>user@company.com</strong></div>
							<div class="preview-line"><span>Password</span><strong>••••••••••</strong></div>
						</div>
						<div class="preview-cta-row">
							<span class="preview-meta">OIDC + PKCE</span>
							<span class="preview-circle">SIGN IN</span>
						</div>
					{:else}
						<div class="preview-heading">Authorize</div>
						<div class="preview-consent-list">
							<div>Profile &amp; Email Address</div>
							<div>Application Session Access</div>
						</div>
						<div class="preview-cta-row">
							<span class="preview-meta">Deny</span>
							<span class="preview-circle">ALLOW</span>
						</div>
					{/if}
				</AuthSplitShell>
			</div>
		</div>
	</form>

	<div class="surface-box">
		<div class="box-header">
			<div>
				<h2 class="box-title">Social OAuth Providers</h2>
				<p class="box-sub">Enable Google or GitHub with platform credentials or AES-256-GCM encrypted custom secrets.</p>
			</div>
		</div>

		<div class="providers-grid">
			{#each providers as p}
				{@const draft = providerDrafts[p.provider]}
				{#if draft}
					<div class="provider-card">
						<div class="provider-head">
							<div>
								<strong>{p.provider === 'google' ? 'Google' : 'GitHub'}</strong>
								<Badge variant={draft.enabled ? 'success' : 'neutral'} size="sm" dot>
									{draft.enabled ? 'Enabled' : 'Disabled'}
								</Badge>
							</div>
							<label class="switch-label">
								<input type="checkbox" bind:checked={draft.enabled} disabled={!canEditProviders} />
								<span>Active</span>
							</label>
						</div>

						<div class="callback-row">
							<span class="cb-label">Callback URL</span>
							<code>{p.callback_url}</code>
							<button type="button" class="copy-btn" onclick={() => copyText(p.callback_url, 'Callback URL')}>Copy</button>
						</div>

						<label class="check-inline">
							<input type="checkbox" bind:checked={draft.use_platform_credentials} disabled={!canEditProviders} />
							<span>Use platform-managed OAuth credentials</span>
						</label>

						{#if !draft.use_platform_credentials}
							<div class="grid-2">
								<label class="field">
									<span>Client ID</span>
									<input type="text" bind:value={draft.client_id} disabled={!canEditProviders} placeholder="OAuth Client ID" />
								</label>
								<label class="field">
									<span>Client Secret {p.has_custom_secret ? '(Encrypted — leave blank to keep)' : ''}</span>
									<input type="password" bind:value={draft.client_secret} disabled={!canEditProviders} placeholder="Enter new secret" />
								</label>
							</div>
						{/if}

						<label class="field">
							<span>Scopes (comma separated)</span>
							<input type="text" bind:value={draft.scopes} disabled={!canEditProviders} />
						</label>

						<div class="form-actions">
							<Button
								variant="outline"
								size="sm"
								loading={savingProvider === p.provider}
								disabled={!canEditProviders || savingProvider === p.provider}
								onclick={() => saveProvider(p.provider as 'google' | 'github')}
							>
								Save {p.provider === 'google' ? 'Google' : 'GitHub'}
							</Button>
						</div>
					</div>
				{/if}
			{/each}
		</div>
	</div>
</section>

<style>
	.two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
	.surface-box { background-color: var(--color-brand-surface-card); border: 1px solid var(--color-brand-border); border-radius: 12px; padding: 1.35rem; display: flex; flex-direction: column; gap: 1.1rem; }
	.box-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
	.box-title { font-size: 0.98rem; font-weight: 600; color: var(--color-brand-text-primary); margin: 0; }
	.box-sub { font-size: 0.78rem; color: var(--color-brand-text-secondary); margin: 0.2rem 0 0; }
	.toggle-list { display: flex; flex-direction: column; border-top: 1px solid var(--color-brand-border); }
	.toggle-row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 0.75rem 0; border-bottom: 1px solid var(--color-brand-border); cursor: pointer; }
	.toggle-row div { display: flex; flex-direction: column; gap: 0.15rem; }
	.toggle-row strong { font-size: 0.85rem; color: var(--color-brand-text-primary); font-weight: 500; }
	.toggle-row span { font-size: 0.76rem; color: var(--color-brand-text-secondary); }
	.toggle-row input { accent-color: #ffffff; width: 16px; height: 16px; }
	.grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 0.85rem; }
	.grid-3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.85rem; align-items: end; }
	.field { display: flex; flex-direction: column; gap: 0.35rem; }
	.field span { font-size: 0.74rem; color: var(--color-brand-text-secondary); font-weight: 500; }
	.field input { background-color: var(--color-brand-surface); border: 1px solid var(--color-brand-border); border-radius: 6px; padding: 0.55rem 0.75rem; color: var(--color-brand-text-primary); font-size: 0.84rem; outline: none; }
	.field input:focus { border-color: #ffffff; }
	.check-inline { display: inline-flex; align-items: center; gap: 0.5rem; font-size: 0.8rem; color: #d4d4d4; cursor: pointer; padding-bottom: 0.35rem; }
	.check-inline input { accent-color: #ffffff; width: 15px; height: 15px; }
	.form-actions { display: flex; justify-content: flex-end; }
	.seg-tabs { display: inline-flex; background-color: var(--color-brand-surface); border: 1px solid var(--color-brand-border); border-radius: 6px; padding: 2px; }
	.seg-tabs button { background: transparent; border: none; color: var(--color-brand-text-secondary); font-size: 0.74rem; padding: 0.25rem 0.6rem; border-radius: 4px; cursor: pointer; }
	.seg-tabs button.active { background-color: #ffffff; color: var(--color-brand-surface); }
	.preview-frame { margin-top: 0.25rem; }
	.preview-heading { font-size: 1.35rem; font-weight: 400; color: #ffffff; margin-bottom: 1rem; }
	.preview-inputs { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; margin-bottom: 1.5rem; }
	.preview-line { border-bottom: 1px solid #2e2e2e; padding-bottom: 0.4rem; display: flex; flex-direction: column; gap: 0.2rem; }
	.preview-line span { font-size: 0.65rem; color: var(--color-brand-text-secondary); }
	.preview-line strong { font-size: 0.75rem; color: var(--color-brand-text-primary); font-weight: 400; }
	.preview-consent-list { display: flex; flex-direction: column; gap: 0.5rem; margin-bottom: 1.5rem; font-size: 0.78rem; color: #d4d4d4; }
	.preview-consent-list div { padding: 0.45rem 0; border-bottom: 1px solid var(--color-brand-border); }
	.preview-cta-row { display: flex; align-items: center; justify-content: space-between; }
	.preview-meta { font-size: 0.72rem; color: var(--color-brand-text-secondary); }
	.preview-circle { width: 58px; height: 58px; border-radius: 9999px; background-color: #ffffff; color: #000000; font-size: 0.62rem; font-weight: 600; display: inline-flex; align-items: center; justify-content: center; letter-spacing: 0.06em; }
	.providers-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
	.provider-card { background-color: var(--color-brand-surface); border: 1px solid var(--color-brand-border); border-radius: 10px; padding: 1.1rem; display: flex; flex-direction: column; gap: 0.85rem; }
	.provider-head { display: flex; align-items: center; justify-content: space-between; }
	.provider-head div { display: flex; align-items: center; gap: 0.55rem; }
	.switch-label { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 0.78rem; color: var(--color-brand-text-secondary); cursor: pointer; }
	.switch-label input { accent-color: #ffffff; }
	.callback-row { display: grid; grid-template-columns: auto 1fr auto; align-items: center; gap: 0.55rem; padding: 0.45rem 0.6rem; background-color: var(--color-brand-surface-card); border: 1px solid var(--color-brand-border); border-radius: 6px; }
	.cb-label { font-size: 0.7rem; color: var(--color-brand-text-secondary); }
	.callback-row code { font-size: 0.72rem; color: #d4d4d4; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.copy-btn { background: transparent; border: 1px solid var(--color-brand-border-hover); color: #d4d4d4; font-size: 0.7rem; padding: 0.2rem 0.45rem; border-radius: 4px; cursor: pointer; }
	@media (max-width: 1080px) {
		.two-col, .providers-grid, .grid-3, .grid-2 { grid-template-columns: 1fr; }
	}
</style>

