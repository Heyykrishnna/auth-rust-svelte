<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import SelectionRow from '$lib/components/SelectionRow.svelte';
  import AuthSplitShell from '$lib/components/AuthSplitShell.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { ApiError } from '$lib/api/client';
	import OtpInput from '$lib/components/OtpInput.svelte';
	import { controlApi } from '$lib/api/control';
	import { setDeveloper, setWorkspaces } from '$lib/stores/control.svelte';

	let mode = $state<'signin' | 'signup'>('signin');
	let email = $state('');
	let password = $state('');
	let factorRequired = $state(false);
	let usingBackup = $state(false);
	let factorCode = $state('');
	let displayName = $state('');
	let workspaceName = $state('');
	let showPassword = $state(false);
	let rememberMe = $state(true);
	let submitting = $state(false);
	let errorMessage = $state('');

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		errorMessage = '';
		submitting = true;
		try {
			const res =
				mode === 'signup'
					? await controlApi.signUp({
							email: email.trim(),
							password,
							display_name: displayName.trim(),
							workspace_name: workspaceName.trim() || undefined
						})
					: await controlApi.signIn({ email: email.trim(), password, second_factor_code: factorRequired ? factorCode.trim() : undefined });

			setDeveloper(res.developer);
			const workspaces = await controlApi.listWorkspaces();
			setWorkspaces(workspaces);
			addToast(
				mode === 'signup' ? 'Developer workspace created' : 'Signed in to Control Plane',
				'success'
			);
			const next = page.url.searchParams.get('returnTo');
			await goto(next?.startsWith('/control?') ? next : '/control');
		} catch (err) {
			if (err instanceof ApiError && (err.code === 'SECOND_FACTOR_REQUIRED' || err.code === 'INVALID_SECOND_FACTOR')) {
				factorRequired = true;
				factorCode = '';
				if (err.code === 'INVALID_SECOND_FACTOR') errorMessage = err.message;
			} else {
				errorMessage = err instanceof Error ? err.message : 'Authentication failed';
			}
		} finally {
			submitting = false;
		}
	}
	function switchMode() {
		mode = mode === 'signin' ? 'signup' : 'signin';
		factorRequired = false;
		factorCode = '';
		usingBackup = false;
		errorMessage = '';
	}

</script>

<svelte:head>
	<title>{mode === 'signin' ? 'Developer Login' : 'Create Developer Account'} — Signet</title>
</svelte:head>

<AuthSplitShell
	brandName="Signet"
	footerText="Signet Control Plane · Multi-Tenant Identity Infrastructure"
	topActionLabel={mode === 'signin' ? 'Create an account' : 'Sign in'}
	onTopAction={switchMode}
>
	<h1 class="editorial-title">{mode === 'signin' ? 'Login' : 'Create Account'}</h1>
	<p class="editorial-subtitle">
		{mode === 'signin'
			? 'Access your developer workspaces, applications, keys, and webhooks.'
			: 'Provision a new developer workspace with isolated dev and prod environments.'}
	</p>

	{#if errorMessage}
		<div class="editorial-alert" role="alert">{errorMessage}</div>
	{/if}

	<form class="editorial-stack" onsubmit={handleSubmit}>
		{#if mode === 'signup'}
			<div class="editorial-field">
				<label class="editorial-label" for="dev-display-name">Display Name *</label>
				<input
					id="dev-display-name"
					type="text"
					class="editorial-input"
					bind:value={displayName}
					placeholder="Ada Lovelace"
					required
				/>
			</div>

			<div class="editorial-field">
				<label class="editorial-label" for="dev-workspace-name">Workspace Name</label>
				<input
					id="dev-workspace-name"
					type="text"
					class="editorial-input"
					bind:value={workspaceName}
					placeholder="Acme Engineering"
				/>
			</div>
		{/if}

		<div class="editorial-field">
			<label class="editorial-label" for="dev-email">Email address *</label>
			<input
				id="dev-email"
				type="email"
				class="editorial-input"
				bind:value={email}
				placeholder="e.g. dev@company.com"
				autocomplete="email"
				required
			/>
		</div>

		<div class="editorial-field">
			<label class="editorial-label" for="dev-password">Password *</label>
			<div class="editorial-input-wrap">
				<input
					id="dev-password"
					type={showPassword ? 'text' : 'password'}
					class="editorial-input"
					bind:value={password}
					placeholder="••••••••••••"
					autocomplete={mode === 'signin' ? 'current-password' : 'new-password'}
					required
				/>
				<button
					type="button"
					class="action-button inline-grid size-8 shrink-0 place-items-center rounded-md absolute right-1.5 top-1/2 -translate-y-1/2"
					onclick={() => (showPassword = !showPassword)}
					aria-label={showPassword ? 'Hide password' : 'Show password'}
				>
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true">
						<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
						<circle cx="12" cy="12" r="3" />
					</svg>
				</button>
			</div>
		</div>

		{#if mode === 'signin'}
			{#if factorRequired}
				<div class="grid gap-2">
					{#if usingBackup}
						<label class="grid gap-1.5 text-sm font-medium">
							Backup code
							<input class="editorial-input" autocomplete="one-time-code" maxlength="27" bind:value={factorCode} required />
						</label>
					{:else}
						<p class="text-sm font-medium">Authenticator code</p>
						<OtpInput bind:value={factorCode} disabled={submitting} />
					{/if}
					<button class="editorial-quiet-link justify-self-start" type="button" onclick={() => { usingBackup = !usingBackup; factorCode = ''; errorMessage = ''; }}>
						{usingBackup ? 'Use an authenticator code' : 'Use a backup code'}
					</button>
				</div>
			{/if}
			<div class="editorial-meta-row">
				<a href="/control/login/recovery" class="editorial-quiet-link">Forgot password?</a>
				<SelectionRow label="Remember me" bind:checked={rememberMe} />
				<a href="/login" class="editorial-quiet-link">End-user sign in</a>
			</div>
		{/if}

		<div class="editorial-actions-footer">
			<button
				type="submit"
				class="action-button w-full min-h-11 shrink-0 inline-flex items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold cursor-pointer"
				disabled={submitting}
			>
				{submitting ? '...' : mode === 'signin' ? 'Sign in' : 'Create workspace'}
			</button>

			<div class="text-center text-sm text-zinc-400">
				<button
					type="button"
					class="text-white hover:underline font-medium bg-transparent border-0 p-0 cursor-pointer text-sm"
					onclick={switchMode}
				>
					{mode === 'signin' ? 'Need a developer workspace? Sign up' : 'Already registered? Sign in'}
				</button>
			</div>

			<p class="text-xs text-zinc-500 text-center m-0">
				By continuing, you agree to our Terms of Service and Privacy Policy.
			</p>
		</div>
	</form>
</AuthSplitShell>
