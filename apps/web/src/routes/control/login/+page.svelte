<script lang="ts">
	import { goto } from '$app/navigation';
	import AuthSplitShell from '$lib/components/AuthSplitShell.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import { setApplications, setDeveloper, setWorkspaces } from '$lib/stores/control.svelte';

	let mode = $state<'signin' | 'signup'>('signin');
	let email = $state('');
	let password = $state('');
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
					: await controlApi.signIn({ email: email.trim(), password });

			setDeveloper(res.developer);
			const workspaces = await controlApi.listWorkspaces();
			setWorkspaces(workspaces);
			if (workspaces[0]) {
				const apps = await controlApi.listApplications(workspaces[0].id);
				setApplications(apps);
			}
			addToast(
				mode === 'signup' ? 'Developer workspace created' : 'Signed in to Control Plane',
				'success'
			);
			goto('/control');
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Authentication failed';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>{mode === 'signin' ? 'Developer Login' : 'Create Developer Account'} — Signet</title>
</svelte:head>

<AuthSplitShell
	brandName="Signet"
	footerText="Signet Control Plane · Multi-Tenant Identity Infrastructure"
	topActionLabel={mode === 'signin' ? 'Create an account' : 'Sign in'}
	onTopAction={() => {
		mode = mode === 'signin' ? 'signup' : 'signin';
		errorMessage = '';
	}}
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
			<div class="editorial-grid-2">
				<div class="editorial-field">
					<label class="editorial-label" for="dev-display-name">Display Name</label>
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
			</div>
		{/if}

		<div class="editorial-grid-2">
			<div class="editorial-field">
				<label class="editorial-label" for="dev-email">Email</label>
				<input
					id="dev-email"
					type="email"
					class="editorial-input"
					bind:value={email}
					placeholder="dev@company.com"
					autocomplete="email"
					required
				/>
			</div>

			<div class="editorial-field">
				<label class="editorial-label" for="dev-password">Password</label>
				<div class="editorial-input-wrap">
					<input
						id="dev-password"
						type={showPassword ? 'text' : 'password'}
						class="editorial-input"
						bind:value={password}
						placeholder="••••••••••"
						autocomplete={mode === 'signin' ? 'current-password' : 'new-password'}
						required
					/>
					<button
						type="button"
						class="editorial-eye"
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
		</div>

		{#if mode === 'signin'}
			<div class="editorial-meta-row">
				<label class="editorial-check">
					<input type="checkbox" bind:checked={rememberMe} />
					<span>Remember me</span>
				</label>
				<a href="/login" class="editorial-quiet-link">End-user sign in</a>
			</div>
		{/if}

		<div class="editorial-actions-footer">
			<button
				type="button"
				class="editorial-quiet-link"
				onclick={() => {
					mode = mode === 'signin' ? 'signup' : 'signin';
					errorMessage = '';
				}}
			>
				{mode === 'signin' ? 'Need a workspace? Sign up' : 'Already registered? Sign in'}
			</button>

			<button type="submit" class="editorial-circle-cta" disabled={submitting}>
				{submitting ? '...' : mode === 'signin' ? 'SIGN IN' : 'CREATE'}
			</button>
		</div>
	</form>
</AuthSplitShell>
