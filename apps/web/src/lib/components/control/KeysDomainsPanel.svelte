<script lang="ts">
	import SectionHeader from './SectionHeader.svelte';
	import Dropdown from '$lib/components/Dropdown.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import Button from '$lib/components/Button.svelte';
	import CopyButton from '$lib/components/CopyButton.svelte';
	import DeleteButton from '$lib/components/DeleteButton.svelte';
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

<section class="grid min-w-0 grid-cols-1 gap-5 sm:gap-6" aria-label="API keys and domains">
	<SectionHeader title="API Keys &amp; Domains" description="Manage environment credentials, allowed CORS origins, and OIDC redirect URIs." />

	<div class="surface-box task-surface">
		<div class="box-header">
			<div>
				<h2 class="box-title">API Credentials ({credentials.length})</h2>
				<p class="box-sub">Secret keys are shown once at creation or rotation and stored as SHA-256 digests.</p>
			</div>
		</div>

		<form class="flex min-w-0 flex-wrap items-center gap-2 [&>button]:h-9 [&>input]:h-9 [&>input]:max-w-full" onsubmit={handleCreateCredential}>
			<Dropdown
				label="Credential type"
				bind:value={credType}
				disabled={!canMutate}
				options={[
					{ value: 'secret', label: 'Secret key (sk_*)' },
					{ value: 'publishable', label: 'Publishable key (pk_*)' }
				]}
				/>
			<input class="control-input" type="text" bind:value={credName} placeholder="Key label (e.g. Backend Sync Worker)" disabled={!canMutate} required />
			<Button type="submit" size="sm" loading={creatingCred} disabled={!canMutate || creatingCred}>
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
									<CopyButton label="Copy publishable key" onclick={() => copyText(c.public_key ?? '', 'Publishable key')} />
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
									<button type="button" class="action-button inline-flex min-h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-semibold" disabled={!canMutate} onclick={() => handleRotateCredential(c.id)}>Rotate</button>
									<DeleteButton label="Revoke credential" disabled={!canMutate} onclick={() => handleRevokeCredential(c.id)} />
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>

	<div class="surface-box task-surface">
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
				<Button type="submit" size="sm" loading={addingDomain} disabled={!canMutate || addingDomain}>
					Register Domain
				</Button>
			</div>
		</form>

		{#if domains.length > 0}
			<div class="domain-list">
				{#each domains as d}
					<div class="domain-card task-surface">
						<div class="domain-top">
							<div class="domain-head">
								<code>{d.domain}</code>
								<Badge variant={d.is_verified ? 'success' : 'warning'} size="sm" dot>
									{d.is_verified ? 'Verified' : 'Pending DNS'}
								</Badge>
							</div>
							<DeleteButton label="Remove domain" disabled={!canMutate} onclick={() => handleDeleteDomain(d.id)} />
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
			<Button size="sm" loading={deletingApp} disabled={deletingApp} onclick={handleDeleteApplication}>
				Delete Application
			</Button>
		</div>
	{/if}
</section>

<style>
	.surface-box { padding: 1.35rem; display: flex; flex-direction: column; gap: 1rem; }
	.box-title { font-size: 0.98rem; font-weight: 600; color: var(--color-brand-text-primary); margin: 0; }
	.box-sub { font-size: 0.78rem; color: var(--color-brand-text-secondary); margin: 0.2rem 0 0; }
	.control-input { background-color: var(--color-brand-surface); border: 1px solid var(--color-brand-border); border-radius: 6px; padding: 0.5rem 0.75rem; color: var(--color-brand-text-primary); font-size: 0.84rem; outline: none; }
	.control-input { flex: 1; min-width: 200px; }
	.table-wrap { overflow-x: auto; border: 1px solid var(--color-brand-border); border-radius: 8px; }
	.data-table { width: 100%; border-collapse: collapse; text-align: left; }
	.data-table th { font-size: 0.72rem; color: var(--color-brand-text-secondary); padding: 0.7rem 0.9rem; border-bottom: 1px solid var(--color-brand-border); background-color: var(--color-brand-surface); }
	.data-table td { padding: 0.75rem 0.9rem; border-bottom: 1px solid var(--color-brand-border); font-size: 0.82rem; color: #d4d4d4; }
	.mono { font-family: var(--font-mono); font-size: 0.76rem; }
	.row-actions { display: flex; gap: 0.4rem; justify-content: flex-end; }


	.domain-form { display: flex; flex-direction: column; gap: 0.85rem; }
	.grid-3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.75rem; }
	.field { display: flex; flex-direction: column; gap: 0.3rem; }
	.field span { font-size: 0.74rem; color: var(--color-brand-text-secondary); }
	.field input, .field textarea { background-color: var(--color-brand-surface); border: 1px solid var(--color-brand-border); border-radius: 6px; padding: 0.55rem 0.7rem; color: var(--color-brand-text-primary); font-size: 0.8rem; font-family: var(--font-mono); outline: none; }
	.form-actions { display: flex; justify-content: flex-end; }
	.domain-list { display: flex; flex-direction: column; gap: 0.65rem; }
	.domain-card { padding: 0.9rem; display: flex; flex-direction: column; gap: 0.5rem; }
	.domain-top { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
	.domain-head { display: flex; align-items: center; gap: 0.6rem; }
	.domain-head code { color: #ffffff; font-size: 0.86rem; }
	.domain-meta { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.75rem; color: var(--color-brand-text-secondary); }
	.danger-box { background-color: #141414; border: 1px solid #2e2e2e; border-radius: 10px; padding: 1.1rem 1.35rem; display: flex; align-items: center; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
	.danger-box strong { color: #b7b7b7; font-size: 0.9rem; }
	.danger-box p { margin: 0.2rem 0 0; font-size: 0.78rem; color: var(--color-brand-text-secondary); }
	@media (max-width: 960px) {
		.grid-3 { grid-template-columns: 1fr; }
	}
</style>

