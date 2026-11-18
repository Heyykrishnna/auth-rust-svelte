<script lang="ts">
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import { showOneTimeSecret } from '$lib/stores/control.svelte';
	import type {
		ApplicationDomainDto,
		CredentialSummaryDto,
		CredentialType,
		EnvironmentKind
	} from '$lib/api/types';
	import { parseLineList, validateDomainForEnv, validateOriginForEnv, validateRedirectUriForEnv } from '$lib/control/validation';

	interface Props {
		appId: string;
		appName: string;
		envKind: EnvironmentKind;
		credentials: CredentialSummaryDto[];
		domains: ApplicationDomainDto[];
		canMutate: boolean;
		canDeleteApp: boolean;
		onReload: () => Promise<void>;
		onAppDeleted: () => Promise<void>;
	}

	let {
		appId,
		appName,
		envKind,
		credentials,
		domains,
		canMutate,
		canDeleteApp,
		onReload,
		onAppDeleted
	}: Props = $props();

	let credType = $state<CredentialType>('secret');
	let credName = $state('');
	let creatingCred = $state(false);

	let domainInput = $state('');
	let originsInput = $state('');
	let redirectsInput = $state('');
	let postLogoutInput = $state('');
	let domainError = $state('');
	let addingDomain = $state(false);
	let deletingApp = $state(false);

	async function handleCreateCredential(e: SubmitEvent) {
		e.preventDefault();
		if (!canMutate) return;
		creatingCred = true;
		try {
			const res = await controlApi.createCredential(appId, envKind, {
				credential_type: credType,
				name: credName.trim() || `${envKind} ${credType} key`
			});
			credName = '';
			if (res.secret_plaintext) {
				showOneTimeSecret({
					title: 'Secret Key Created',
					secretKey: res.secret_plaintext
				});
			}
			addToast('Credential created', 'success');
			await onReload();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to create credential', 'error');
		} finally {
			creatingCred = false;
		}
	}

	async function handleRotateCredential(credId: string) {
		if (!canMutate) return;
		try {
			const res = await controlApi.rotateCredential(appId, envKind, credId, 3600);
			if (res.secret_plaintext) {
				showOneTimeSecret({
					title: 'Rotated Secret Key (1h Grace Window)',
					secretKey: res.secret_plaintext
				});
			}
			addToast('Credential rotated with 1-hour grace window', 'success');
			await onReload();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to rotate credential', 'error');
		}
	}

	async function handleRevokeCredential(credId: string) {
		if (!canMutate) return;
		try {
			await controlApi.revokeCredential(appId, envKind, credId);
			addToast('Credential revoked', 'info');
			await onReload();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to revoke credential', 'error');
		}
	}

	async function handleAddDomain(e: SubmitEvent) {
		e.preventDefault();
		if (!canMutate) return;
		domainError = '';
		const domainErr = validateDomainForEnv(envKind, domainInput);
		if (domainErr) {
			domainError = domainErr;
			return;
		}
		const origins = parseLineList(originsInput);
		for (const o of origins) {
			const err = validateOriginForEnv(envKind, o);
			if (err) {
				domainError = err;
				return;
			}
		}
		const redirects = parseLineList(redirectsInput);
		for (const r of redirects) {
			const err = validateRedirectUriForEnv(envKind, r);
			if (err) {
				domainError = err;
				return;
			}
		}
		const postLogout = parseLineList(postLogoutInput);
		for (const p of postLogout) {
			const err = validateRedirectUriForEnv(envKind, p);
			if (err) {
				domainError = err;
				return;
			}
		}

		addingDomain = true;
		try {
			await controlApi.createDomain(appId, envKind, {
				domain: domainInput.trim(),
				allowed_origins: origins,
				redirect_uris: redirects,
				post_logout_redirect_uris: postLogout
			});
			domainInput = '';
			originsInput = '';
			redirectsInput = '';
			postLogoutInput = '';
			addToast('Domain registered', 'success');
			await onReload();
		} catch (err) {
			domainError = err instanceof Error ? err.message : 'Failed to register domain';
		} finally {
			addingDomain = false;
		}
	}

	async function handleDeleteDomain(domainId: string) {
		if (!canMutate) return;
		try {
			await controlApi.deleteDomain(appId, envKind, domainId);
			addToast('Domain removed', 'info');
			await onReload();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to remove domain', 'error');
		}
	}

	async function handleDeleteApplication() {
		if (!canDeleteApp) return;
		deletingApp = true;
		try {
			await controlApi.deleteApplication(appId);
			addToast(`Application '${appName}' deleted`, 'warning');
			await onAppDeleted();
		} catch (err) {
			addToast(err instanceof Error ? err.message : 'Failed to delete application', 'error');
		} finally {
			deletingApp = false;
		}
	}

	async function copyText(val: string, label: string) {
		try {
			await navigator.clipboard.writeText(val);
			addToast(`${label} copied`, 'success');
		} catch {
			addToast('Copy failed', 'warning');
		}
	}
</script>

<section class="panel-stack" aria-label="API keys and domains">
	<div class="section-header">
		<div>
			<h1 class="page-title">API Keys &amp; Domains</h1>
			<p class="page-subtitle">Manage environment credentials, allowed CORS origins, and OIDC redirect URIs.</p>
		</div>
	</div>

	<div class="surface-box">
		<div class="box-header">
			<div>
				<h2 class="box-title">API Credentials ({credentials.length})</h2>
				<p class="box-sub">Secret keys are shown once at creation or rotation and stored as SHA-256 digests.</p>
			</div>
		</div>

		<form class="inline-form" onsubmit={handleCreateCredential}>
			<select bind:value={credType} disabled={!canMutate} aria-label="Credential type">
				<option value="secret">Secret Key (sk_*)</option>
				<option value="publishable">Publishable Key (pk_*)</option>
			</select>
			<input type="text" bind:value={credName} placeholder="Key label (e.g. Backend Sync Worker)" disabled={!canMutate} required />
			<Button type="submit" variant="primary" size="sm" loading={creatingCred} disabled={!canMutate || creatingCred}>
				Create Key
			</Button>
		</form>

		<div class="table-wrap">
			<table class="data-table">
				<thead>
					<tr>
						<th>Name</th>
						<th>Type</th>
						<th>Key / Prefix</th>
						<th>Status</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each credentials as c}
						<tr>
							<td><strong>{c.name}</strong></td>
							<td class="mono">{c.credential_type}</td>
							<td class="mono">
								{#if c.public_key}
									<span>{c.public_key}</span>
									<button type="button" class="mini-btn" onclick={() => copyText(c.public_key ?? '', 'Publishable key')}>Copy</button>
								{:else}
									<span>{c.key_prefix}••••••••</span>
								{/if}
							</td>
							<td>
								<Badge variant={c.revoked_at ? 'danger' : 'success'} size="sm" dot>
									{c.revoked_at ? 'Revoked' : 'Active'}
								</Badge>
							</td>
							<td class="row-actions">
								{#if !c.revoked_at}
									<button type="button" class="mini-btn" disabled={!canMutate} onclick={() => handleRotateCredential(c.id)}>Rotate</button>
									<button type="button" class="mini-btn mini-btn-danger" disabled={!canMutate} onclick={() => handleRevokeCredential(c.id)}>Revoke</button>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>

	<div class="surface-box">
		<div class="box-header">
			<div>
				<h2 class="box-title">Allowed Domains, Origins &amp; Redirect URIs ({domains.length})</h2>
				<p class="box-sub">Enforces dynamic CORS allowlists and exact OIDC redirect URI matching.</p>
			</div>
		</div>

		{#if domainError}
			<p class="error-text" role="alert">{domainError}</p>
		{/if}

		<form class="domain-form" onsubmit={handleAddDomain}>
			<label class="field">
				<span>Domain</span>
				<input type="text" bind:value={domainInput} placeholder={envKind === 'development' ? 'localhost:5174' : 'app.ferrum.io'} disabled={!canMutate} required />
			</label>
			<div class="grid-3">
				<label class="field">
					<span>Allowed Origins (one per line)</span>
					<textarea rows="3" bind:value={originsInput} placeholder={envKind === 'development' ? 'http://localhost:5174' : 'https://app.ferrum.io'} disabled={!canMutate}></textarea>
				</label>
				<label class="field">
					<span>OIDC Redirect URIs (one per line)</span>
					<textarea rows="3" bind:value={redirectsInput} placeholder={envKind === 'development' ? 'http://localhost:5174/auth/callback' : 'https://app.ferrum.io/auth/callback'} disabled={!canMutate}></textarea>
				</label>
				<label class="field">
					<span>Post-Logout Redirect URIs</span>
					<textarea rows="3" bind:value={postLogoutInput} placeholder={envKind === 'development' ? 'http://localhost:5174/' : 'https://app.ferrum.io/'} disabled={!canMutate}></textarea>
				</label>
			</div>
			<div class="form-actions">
				<Button type="submit" variant="primary" size="sm" loading={addingDomain} disabled={!canMutate || addingDomain}>
					Register Domain
				</Button>
			</div>
		</form>

		{#if domains.length > 0}
			<div class="domain-list">
				{#each domains as d}
					<div class="domain-card">
						<div class="domain-top">
							<div class="domain-head">
								<code>{d.domain}</code>
								<Badge variant={d.is_verified ? 'success' : 'warning'} size="sm" dot>
									{d.is_verified ? 'Verified' : 'Pending DNS'}
								</Badge>
							</div>
							<Button variant="danger" size="sm" disabled={!canMutate} onclick={() => handleDeleteDomain(d.id)}>Remove</Button>
						</div>
						<div class="domain-meta">
							<span>Origins: {d.allowed_origins.join(', ') || 'None'}</span>
							<span>Redirects: {d.redirect_uris.join(', ') || 'None'}</span>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	{#if canDeleteApp}
		<div class="danger-box">
			<div>
				<strong>Delete Application</strong>
				<p>Soft-deletes '{appName}' and revokes all development and production credentials immediately.</p>
			</div>
			<Button variant="danger" size="sm" loading={deletingApp} disabled={deletingApp} onclick={handleDeleteApplication}>
				Delete Application
			</Button>
		</div>
	{/if}
</section>

<style>
	.panel-stack { display: flex; flex-direction: column; gap: 1.5rem; }
	.page-title { font-size: 1.75rem; font-weight: 600; color: #ffffff; margin: 0; }
	.page-subtitle { font-size: 0.85rem; color: #71717a; margin: 0.25rem 0 0; }
	.surface-box { background-color: #15171b; border: 1px solid #23262e; border-radius: 12px; padding: 1.35rem; display: flex; flex-direction: column; gap: 1rem; }
	.box-title { font-size: 0.98rem; font-weight: 600; color: #f4f4f5; margin: 0; }
	.box-sub { font-size: 0.78rem; color: #71717a; margin: 0.2rem 0 0; }
	.inline-form { display: flex; gap: 0.65rem; flex-wrap: wrap; }
	.inline-form select, .inline-form input { background-color: #0b0c0e; border: 1px solid #23262e; border-radius: 6px; padding: 0.5rem 0.75rem; color: #f4f4f5; font-size: 0.84rem; outline: none; }
	.inline-form input { flex: 1; min-width: 200px; }
	.table-wrap { overflow-x: auto; border: 1px solid #23262e; border-radius: 8px; }
	.data-table { width: 100%; border-collapse: collapse; text-align: left; }
	.data-table th { font-size: 0.72rem; color: #71717a; padding: 0.7rem 0.9rem; border-bottom: 1px solid #23262e; background-color: #0b0c0e; }
	.data-table td { padding: 0.75rem 0.9rem; border-bottom: 1px solid #1f2228; font-size: 0.82rem; color: #d4d4d8; }
	.mono { font-family: var(--font-mono); font-size: 0.76rem; }
	.row-actions { display: flex; gap: 0.4rem; justify-content: flex-end; }
	.mini-btn { background: transparent; border: 1px solid #2e333d; color: #d4d4d8; border-radius: 4px; padding: 0.22rem 0.5rem; font-size: 0.72rem; cursor: pointer; margin-left: 0.35rem; }
	.mini-btn-danger { border-color: #572327; color: #f87171; }
	.mini-btn:disabled { opacity: 0.45; cursor: not-allowed; }
	.domain-form { display: flex; flex-direction: column; gap: 0.85rem; }
	.grid-3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.75rem; }
	.field { display: flex; flex-direction: column; gap: 0.3rem; }
	.field span { font-size: 0.74rem; color: #a1a1aa; }
	.field input, .field textarea { background-color: #0b0c0e; border: 1px solid #23262e; border-radius: 6px; padding: 0.55rem 0.7rem; color: #f4f4f5; font-size: 0.8rem; font-family: var(--font-mono); outline: none; }
	.form-actions { display: flex; justify-content: flex-end; }
	.domain-list { display: flex; flex-direction: column; gap: 0.65rem; }
	.domain-card { background-color: #0b0c0e; border: 1px solid #23262e; border-radius: 8px; padding: 0.9rem; display: flex; flex-direction: column; gap: 0.5rem; }
	.domain-top { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
	.domain-head { display: flex; align-items: center; gap: 0.6rem; }
	.domain-head code { color: #ffffff; font-size: 0.86rem; }
	.domain-meta { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.75rem; color: #71717a; }
	.danger-box { background-color: #1c1213; border: 1px solid #572327; border-radius: 10px; padding: 1.1rem 1.35rem; display: flex; align-items: center; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
	.danger-box strong { color: #fca5a5; font-size: 0.9rem; }
	.danger-box p { margin: 0.2rem 0 0; font-size: 0.78rem; color: #a1a1aa; }
	@media (max-width: 960px) {
		.grid-3 { grid-template-columns: 1fr; }
	}
</style>

