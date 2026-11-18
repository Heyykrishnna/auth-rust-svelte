<script lang="ts">
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import Card from '$lib/components/Card.svelte';
	import Input from '$lib/components/Input.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import type {
		ApplicationDomainDto,
		CredentialSummaryDto,
		EnrollmentPolicy
	} from '$lib/api/types';
	import {
		maskSecretPrefix,
		validateDomainForEnv,
		validateOriginForEnv,
		validateRedirectUriForEnv,
		validateSlug
	} from '$lib/control/validation';
	import {
		controlStore,
		selectApplication,
		setApplications,
		showOneTimeSecret
	} from '$lib/stores/control.svelte';

	type TabKey = 'overview' | 'keys' | 'domains';

	let activeTab = $state<TabKey>('overview');
	let credentials = $state<CredentialSummaryDto[]>([]);
	let domains = $state<ApplicationDomainDto[]>([]);
	let loadingEnvResources = $state(false);

	let showCreateAppModal = $state(false);
	let newAppName = $state('');
	let newAppSlug = $state('');
	let newAppRedirect = $state('http://localhost:5173/auth/callback');
	let creatingApp = $state(false);

	let envPolicy = $state<EnrollmentPolicy>('consent_required');
	let envSessionTtl = $state('604800');
	let envAccessTtl = $state('300');
	let envRequireVerified = $state(true);
	let savingSettings = $state(false);

	let newCredName = $state('');
	let newCredType = $state<'secret' | 'publishable'>('secret');
	let selectedScopes = $state<string[]>(['users:read', 'sync:read']);
	let creatingCred = $state(false);

	let newDomain = $state('');
	let newOriginsText = $state('');
	let newRedirectsText = $state('');
	let domainValidationError = $state('');
	let creatingDomain = $state(false);

	const availableScopes = [
		'users:read',
		'users:write',
		'sessions:read',
		'sessions:revoke',
		'orgs:read',
		'orgs:manage',
		'sync:read'
	];

	async function loadEnvironmentResources() {
		const app = controlStore.activeApplication;
		const env = controlStore.activeEnvironment;
		if (!app || !env) {
			credentials = [];
			domains = [];
			return;
		}
		envPolicy = env.enrollment_policy;
		envSessionTtl = String(env.session_ttl_secs);
		envAccessTtl = String(env.access_token_ttl_secs);
		envRequireVerified = env.require_email_verification;

		loadingEnvResources = true;
		try {
			const [credsRes, domainsRes] = await Promise.all([
				controlApi.listCredentials(app.application.id, env.kind),
				controlApi.listDomains(app.application.id, env.kind)
			]);
			credentials = credsRes;
			domains = domainsRes;
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to load environment details', 'error');
		} finally {
			loadingEnvResources = false;
		}
	}

	$effect(() => {
		const appId = controlStore.activeApplication?.application.id;
		const envKind = controlStore.selectedEnvKind;
		if (controlStore.isAuthenticated && appId && envKind) {
			loadEnvironmentResources();
		}
	});

	async function refreshApps() {
		const wsId = controlStore.activeWorkspace?.id;
		if (!wsId) return;
		const apps = await controlApi.listApplications(wsId);
		setApplications(apps);
	}

	async function handleCreateApp() {
		const ws = controlStore.activeWorkspace;
		if (!ws) return;
		const slugErr = validateSlug(newAppSlug);
		if (slugErr) {
			addToast(slugErr, 'error');
			return;
		}
		if (newAppRedirect.trim()) {
			const uriErr = validateRedirectUriForEnv('development', newAppRedirect);
			if (uriErr) {
				addToast(uriErr, 'error');
				return;
			}
		}

		creatingApp = true;
		try {
			const created = await controlApi.createApplication({
				workspace_id: ws.id,
				name: newAppName.trim(),
				slug: newAppSlug.trim().toLowerCase(),
				initial_dev_redirect_uri: newAppRedirect.trim() || undefined
			});
			await refreshApps();
			selectApplication(created.application.id);
			showCreateAppModal = false;
			newAppName = '';
			newAppSlug = '';
			showOneTimeSecret({
				title: `${created.application.name} — Initial Development Credentials`,
				publishableKey: created.development_publishable_key,
				secretKey: created.initial_development_secret_key
			});
			addToast(`Application ${created.application.name} created`, 'success');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to create application', 'error');
		} finally {
			creatingApp = false;
		}
	}

	async function handleSaveEnvironmentSettings(e: SubmitEvent) {
		e.preventDefault();
		const app = controlStore.activeApplication;
		const env = controlStore.activeEnvironment;
		if (!app || !env) return;

		savingSettings = true;
		try {
			await controlApi.updateEnvironment(app.application.id, env.kind, {
				enrollment_policy: envPolicy,
				session_ttl_secs: Number(envSessionTtl),
				access_token_ttl_secs: Number(envAccessTtl),
				require_email_verification: envRequireVerified
			});
			await refreshApps();
			addToast(`Saved ${env.kind} environment settings`, 'success');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to update environment', 'error');
		} finally {
			savingSettings = false;
		}
	}

	async function handleToggleAppStatus() {
		const app = controlStore.activeApplication?.application;
		if (!app) return;
		const nextStatus = app.status === 'active' ? 'suspended' : 'active';
		try {
			await controlApi.updateApplication(app.id, { status: nextStatus });
			await refreshApps();
			addToast(`Application ${nextStatus}`, 'info');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to update status', 'error');
		}
	}

	async function handleDeleteApp() {
		const app = controlStore.activeApplication?.application;
		if (!app) return;
		try {
			await controlApi.deleteApplication(app.id);
			await refreshApps();
			addToast(`Application ${app.name} deleted and credentials revoked`, 'info');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to delete application', 'error');
		}
	}

	function toggleScope(scope: string) {
		if (selectedScopes.includes(scope)) {
			selectedScopes = selectedScopes.filter((s) => s !== scope);
		} else {
			selectedScopes = [...selectedScopes, scope];
		}
	}

	async function handleCreateCredential(e: SubmitEvent) {
		e.preventDefault();
		const app = controlStore.activeApplication;
		const env = controlStore.activeEnvironment;
		if (!app || !env) return;

		creatingCred = true;
		try {
			const res = await controlApi.createCredential(app.application.id, env.kind, {
				credential_type: newCredType,
				name: newCredName.trim(),
				scopes: newCredType === 'secret' ? selectedScopes : undefined
			});
			newCredName = '';
			await loadEnvironmentResources();
			if (res.secret_plaintext) {
				showOneTimeSecret({
					title: `${res.credential.name} (${env.kind})`,
					secretKey: res.secret_plaintext
				});
			} else {
				addToast('Publishable key created', 'success');
			}
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to create credential', 'error');
		} finally {
			creatingCred = false;
		}
	}

	async function handleRotateCredential(credId: string, name: string) {
		const app = controlStore.activeApplication;
		const env = controlStore.activeEnvironment;
		if (!app || !env) return;
		try {
			const res = await controlApi.rotateCredential(app.application.id, env.kind, credId, 0);
			await loadEnvironmentResources();
			if (res.secret_plaintext) {
				showOneTimeSecret({
					title: `Rotated Secret Key — ${name}`,
					secretKey: res.secret_plaintext
				});
			}
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to rotate secret key', 'error');
		}
	}

	async function handleRevokeCredential(credId: string) {
		const app = controlStore.activeApplication;
		const env = controlStore.activeEnvironment;
		if (!app || !env) return;
		try {
			await controlApi.revokeCredential(app.application.id, env.kind, credId);
			await loadEnvironmentResources();
			addToast('Credential revoked immediately', 'info');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to revoke credential', 'error');
		}
	}

	async function handleAddDomain(e: SubmitEvent) {
		e.preventDefault();
		const app = controlStore.activeApplication;
		const env = controlStore.activeEnvironment;
		if (!app || !env) return;

		domainValidationError = '';
		const domainErr = validateDomainForEnv(env.kind, newDomain);
		if (domainErr) {
			domainValidationError = domainErr;
			return;
		}

		const origins = newOriginsText
			.split('\n')
			.map((s) => s.trim())
			.filter(Boolean);
		for (const origin of origins) {
			const err = validateOriginForEnv(env.kind, origin);
			if (err) {
				domainValidationError = `Origin "${origin}": ${err}`;
				return;
			}
		}

		const redirects = newRedirectsText
			.split('\n')
			.map((s) => s.trim())
			.filter(Boolean);
		for (const uri of redirects) {
			const err = validateRedirectUriForEnv(env.kind, uri);
			if (err) {
				domainValidationError = `Redirect URI "${uri}": ${err}`;
				return;
			}
		}

		creatingDomain = true;
		try {
			await controlApi.createDomain(app.application.id, env.kind, {
				domain: newDomain.trim(),
				allowed_origins: origins,
				redirect_uris: redirects
			});
			newDomain = '';
			newOriginsText = '';
			newRedirectsText = '';
			await loadEnvironmentResources();
			addToast('Domain and dynamic CORS origins updated', 'success');
		} catch (err) {
			domainValidationError = err instanceof Error ? err.message : 'Failed to register domain';
		} finally {
			creatingDomain = false;
		}
	}

	async function handleDeleteDomain(domainId: string) {
		const app = controlStore.activeApplication;
		const env = controlStore.activeEnvironment;
		if (!app || !env) return;
		try {
			await controlApi.deleteDomain(app.application.id, env.kind, domainId);
			await loadEnvironmentResources();
			addToast('Domain removed and CORS cache invalidated', 'info');
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to delete domain', 'error');
		}
	}
</script>

<svelte:head>
	<title>Developer Control Plane — AuthPlatform</title>
</svelte:head>

<div class="control-header">
	<div>
		<h1 class="page-title">
			{controlStore.activeApplication?.application.name ?? 'Applications'}
		</h1>
		<p class="page-sub">
			Register applications, isolate dev & prod credentials, and configure live CORS origins and OIDC redirect URIs.
		</p>
	</div>
	<Button variant="primary" onclick={() => (showCreateAppModal = true)}>+ New Application</Button>
</div>

{#if !controlStore.activeApplication || !controlStore.activeEnvironment}
	<Card
		title="Create Your First Application"
		subtitle="Every application gets isolated Development and Production environments automatically."
	>
		<p class="empty-copy">
			Click <strong>+ New Application</strong> to mint your initial <code>pk_test_*</code> and
			<code>sk_test_*</code> keys and connect your first project.
		</p>
		<Button variant="primary" onclick={() => (showCreateAppModal = true)}>
			Launch Application Wizard
		</Button>
	</Card>
{:else}
	<div class="tab-bar" role="tablist">
		<button
			type="button"
			role="tab"
			aria-selected={activeTab === 'overview'}
			class="tab-btn"
			class:active={activeTab === 'overview'}
			onclick={() => (activeTab = 'overview')}
		>
			Overview & Settings
		</button>
		<button
			type="button"
			role="tab"
			aria-selected={activeTab === 'keys'}
			class="tab-btn"
			class:active={activeTab === 'keys'}
			onclick={() => (activeTab = 'keys')}
		>
			API Keys ({credentials.filter((c) => !c.revoked_at).length})
		</button>
		<button
			type="button"
			role="tab"
			aria-selected={activeTab === 'domains'}
			class="tab-btn"
			class:active={activeTab === 'domains'}
			onclick={() => (activeTab = 'domains')}
		>
			Domains & Redirects ({domains.length})
		</button>
	</div>

	{#if activeTab === 'overview'}
		<div class="grid-two">
			<Card
				title="Environment Policy ({controlStore.activeEnvironment.kind})"
				subtitle="Changes apply immediately to this environment without restarting auth-api."
			>
				<form class="stack" onsubmit={handleSaveEnvironmentSettings}>
					<label class="field">
						<span>First-Time SSO Enrollment Policy</span>
						<select bind:value={envPolicy}>
							<option value="consent_required">Consent Required (Recommended)</option>
							<option value="auto_join">Auto Join (First-Party Suite)</option>
							<option value="invite_only">Invite Only</option>
						</select>
					</label>

					<Input
						id="env-access-ttl"
						type="number"
						label="Access Token TTL (seconds, 60–86400)"
						bind:value={envAccessTtl}
					/>
					<Input
						id="env-session-ttl"
						type="number"
						label="Session TTL (seconds)"
						bind:value={envSessionTtl}
					/>

					<label class="checkbox-row">
						<input type="checkbox" bind:checked={envRequireVerified} />
						<span>Require verified email before issuing sessions</span>
					</label>

					<Button type="submit" variant="primary" loading={savingSettings}>
						Save Environment Settings
					</Button>
				</form>
			</Card>

			<Card title="Application Lifecycle" subtitle="Slug: {controlStore.activeApplication.application.slug}">
				<div class="stack">
					<div class="meta-row">
						<span>Application ID</span>
						<code>{controlStore.activeApplication.application.id}</code>
					</div>
					<div class="meta-row">
						<span>Environment ID</span>
						<code>{controlStore.activeEnvironment.id}</code>
					</div>
					<div class="meta-row">
						<span>Status</span>
						<Badge
							variant={controlStore.activeApplication.application.status === 'active'
								? 'success'
								: 'warning'}
						>
							{controlStore.activeApplication.application.status}
						</Badge>
					</div>

					<div class="action-row">
						<Button variant="outline" onclick={handleToggleAppStatus}>
							{controlStore.activeApplication.application.status === 'active'
								? 'Suspend Application'
								: 'Reactivate Application'}
						</Button>
						<Button variant="danger" onclick={handleDeleteApp}>Delete Application</Button>
					</div>
				</div>
			</Card>
		</div>
	{:else if activeTab === 'keys'}
		<div class="grid-two">
			<Card title="Mint New Credential" subtitle="Secret keys are displayed in plaintext exactly once.">
				<form class="stack" onsubmit={handleCreateCredential}>
					<label class="field">
						<span>Credential Type</span>
						<select bind:value={newCredType}>
							<option value="secret">Secret Key (Server-Side sk_*)</option>
							<option value="publishable">Publishable Key (Client SDK pk_*)</option>
						</select>
					</label>

					<Input
						id="cred-name"
						label="Credential Label"
						bind:value={newCredName}
						placeholder="Ferrum Backend Sync"
						required
					/>

					{#if newCredType === 'secret'}
						<div class="field">
							<span>Permission Scopes</span>
							<div class="scope-grid">
								{#each availableScopes as scope}
									<label class="checkbox-row">
										<input
											type="checkbox"
											checked={selectedScopes.includes(scope)}
											onchange={() => toggleScope(scope)}
										/>
										<code>{scope}</code>
									</label>
								{/each}
							</div>
						</div>
					{/if}

					<Button type="submit" variant="primary" loading={creatingCred}>
						Generate {newCredType === 'secret' ? 'Secret Key' : 'Publishable Key'}
					</Button>
				</form>
			</Card>

			<Card title="Active & Historical Keys" subtitle={loadingEnvResources ? 'Refreshing...' : ''}>
				<div class="stack">
					{#each credentials as cred}
						<div class="item-row">
							<div class="item-meta">
								<div class="item-head">
									<strong>{cred.name}</strong>
									<Badge variant={cred.revoked_at ? 'neutral' : 'primary'}>
										{cred.revoked_at ? 'Revoked' : cred.credential_type}
									</Badge>
								</div>
								<code class="key-preview">
									{cred.public_key ?? maskSecretPrefix(cred.key_prefix)}
								</code>
								{#if cred.scopes.length > 0}
									<span class="sub-meta">Scopes: {cred.scopes.join(', ')}</span>
								{/if}
								<span class="sub-meta">
									Last used: {cred.last_used_at ? new Date(cred.last_used_at).toLocaleString() : 'Never'}
								</span>
							</div>
							{#if !cred.revoked_at}
								<div class="row-actions">
									{#if cred.credential_type === 'secret'}
										<Button
											variant="outline"
											size="sm"
											onclick={() => handleRotateCredential(cred.id, cred.name)}
										>
											Rotate
										</Button>
									{/if}
									<Button
										variant="danger"
										size="sm"
										onclick={() => handleRevokeCredential(cred.id)}
									>
										Revoke
									</Button>
								</div>
							{/if}
						</div>
					{:else}
						<p class="empty-copy">No credentials found for this environment.</p>
					{/each}
				</div>
			</Card>
		</div>
	{:else}
		<div class="grid-two">
			<Card
				title="Register Domain & Origins ({controlStore.activeEnvironment.kind})"
				subtitle={controlStore.activeEnvironment.kind === 'production'
					? 'Production strictly requires https:// origins and exact redirect URIs.'
					: 'Development permits http://localhost:<port> and https:// origins.'}
			>
				<form class="stack" onsubmit={handleAddDomain}>
					{#if domainValidationError}
						<div class="form-error" role="alert">{domainValidationError}</div>
					{/if}

					<Input
						id="domain-host"
						label="Hostname"
						bind:value={newDomain}
						placeholder={controlStore.activeEnvironment.kind === 'production'
							? 'ferrum.app'
							: 'localhost:5173'}
						required
					/>

					<label class="field">
						<span>Allowed CORS Origins (one per line)</span>
						<textarea
							rows="3"
							bind:value={newOriginsText}
							placeholder={controlStore.activeEnvironment.kind === 'production'
								? 'https://ferrum.app'
								: 'http://localhost:5173'}
						></textarea>
					</label>

					<label class="field">
						<span>Exact OIDC Redirect URIs (one per line)</span>
						<textarea
							rows="3"
							bind:value={newRedirectsText}
							placeholder={controlStore.activeEnvironment.kind === 'production'
								? 'https://ferrum.app/auth/callback'
								: 'http://localhost:5173/auth/callback'}
						></textarea>
					</label>

					<Button type="submit" variant="primary" loading={creatingDomain}>
						Save Domain & Update Live CORS
					</Button>
				</form>
			</Card>

			<Card title="Registered Domains" subtitle="Changes invalidate CORS caches immediately.">
				<div class="stack">
					{#each domains as dom}
						<div class="item-row">
							<div class="item-meta">
								<div class="item-head">
									<strong>{dom.domain}</strong>
									<Badge variant={dom.is_verified ? 'success' : 'warning'}>
										{dom.is_verified ? 'Verified' : 'DNS Verification Pending'}
									</Badge>
								</div>
								{#if dom.verification_token}
									<code class="sub-meta">TXT: {dom.verification_token}</code>
								{/if}
								<span class="sub-meta">
									Origins: {dom.allowed_origins.length > 0 ? dom.allowed_origins.join(', ') : 'None'}
								</span>
								<span class="sub-meta">
									Redirects: {dom.redirect_uris.length > 0 ? dom.redirect_uris.join(', ') : 'None'}
								</span>
							</div>
							<Button variant="danger" size="sm" onclick={() => handleDeleteDomain(dom.id)}>
								Remove
							</Button>
						</div>
					{:else}
						<p class="empty-copy">No domains registered in this environment yet.</p>
					{/each}
				</div>
			</Card>
		</div>
	{/if}
{/if}

<Modal
	open={showCreateAppModal}
	title="Register New Application"
	description="Creates isolated Development and Production environments, default roles, and initial API keys."
	confirmText="Create Application"
	loading={creatingApp}
	onconfirm={handleCreateApp}
	oncancel={() => (showCreateAppModal = false)}
>
	<div class="stack">
		<Input
			id="wizard-app-name"
			label="Application Name"
			bind:value={newAppName}
			placeholder="Ferrum"
			required
		/>
		<Input
			id="wizard-app-slug"
			label="Application Slug"
			bind:value={newAppSlug}
			placeholder="ferrum"
			required
		/>
		<Input
			id="wizard-app-redirect"
			label="Initial Development Redirect URI (Optional)"
			bind:value={newAppRedirect}
			placeholder="http://localhost:5173/auth/callback"
		/>
	</div>
</Modal>

<style>
	.control-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
		margin-bottom: 1.5rem;
	}

	.page-title {
		margin: 0 0 0.25rem;
		font-size: 1.75rem;
		color: #f8fafc;
	}

	.page-sub {
		margin: 0;
		color: #94a3b8;
		font-size: 0.92rem;
	}

	.tab-bar {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		padding-bottom: 0.5rem;
	}

	.tab-btn {
		background: transparent;
		border: 1px solid transparent;
		color: #94a3b8;
		font-weight: 600;
		font-size: 0.9rem;
		padding: 0.5rem 0.95rem;
		border-radius: 8px;
		cursor: pointer;
	}

	.tab-btn.active {
		color: #ffffff;
		background: rgba(99, 102, 241, 0.18);
		border-color: rgba(99, 102, 241, 0.4);
	}

	.grid-two {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
		gap: 1.5rem;
	}

	.stack {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		font-size: 0.875rem;
		color: #cbd5e1;
	}

	select,
	textarea {
		background: rgba(15, 23, 42, 0.8);
		color: #f8fafc;
		border: 1px solid rgba(255, 255, 255, 0.14);
		border-radius: 8px;
		padding: 0.6rem 0.75rem;
		font-family: inherit;
		font-size: 0.875rem;
	}

	.checkbox-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		color: #cbd5e1;
	}

	.scope-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.5rem;
	}

	.meta-row,
	.item-row {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 1rem;
		padding: 0.85rem;
		border-radius: 10px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
	}

	.item-meta {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		min-width: 0;
	}

	.item-head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.key-preview {
		font-size: 0.8rem;
		color: #e2e8f0;
		word-break: break-all;
	}

	.sub-meta {
		font-size: 0.78rem;
		color: #94a3b8;
		word-break: break-all;
	}

	.row-actions,
	.action-row {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.empty-copy {
		color: #94a3b8;
		font-size: 0.9rem;
		margin: 0 0 1rem;
	}

	.form-error {
		padding: 0.75rem 1rem;
		border-radius: 8px;
		background: rgba(239, 68, 68, 0.12);
		border: 1px solid rgba(239, 68, 68, 0.35);
		color: #fca5a5;
		font-size: 0.875rem;
	}
</style>

