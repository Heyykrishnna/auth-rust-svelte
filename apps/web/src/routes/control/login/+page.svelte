<script lang="ts">
	import { goto } from '$app/navigation';
	import Button from '$lib/components/Button.svelte';
	import Card from '$lib/components/Card.svelte';
	import Input from '$lib/components/Input.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';
	import { setApplications, setDeveloper, setWorkspaces } from '$lib/stores/control.svelte';

	let mode = $state<'signin' | 'signup'>('signin');
	let email = $state('');
	let password = $state('');
	let displayName = $state('');
	let workspaceName = $state('');
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
							email,
							password,
							display_name: displayName,
							workspace_name: workspaceName || undefined
						})
					: await controlApi.signIn({ email, password });

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
	<title>Developer Sign In — Control Plane</title>
</svelte:head>

<div class="auth-center">
	<Card
		title={mode === 'signin' ? 'Developer Control Plane Sign In' : 'Create Developer Account'}
		subtitle="Manage applications, dev/prod environments, API keys, and dynamic domains."
	>
		<form class="auth-form" onsubmit={handleSubmit}>
			{#if errorMessage}
				<div class="form-error" role="alert">{errorMessage}</div>
			{/if}

			{#if mode === 'signup'}
				<Input
					id="dev-display-name"
					label="Display Name"
					bind:value={displayName}
					placeholder="Ada Lovelace"
					required
				/>
				<Input
					id="dev-workspace-name"
					label="Workspace Name (Optional)"
					bind:value={workspaceName}
					placeholder="Acme Engineering"
				/>
			{/if}

			<Input
				id="dev-email"
				type="email"
				label="Developer Email"
				bind:value={email}
				placeholder="dev@company.com"
				required
			/>
			<Input
				id="dev-password"
				type="password"
				label="Password"
				bind:value={password}
				placeholder="At least 8 characters"
				required
			/>

			<Button type="submit" variant="primary" loading={submitting} disabled={submitting}>
				{mode === 'signin' ? 'Sign In' : 'Create Developer Workspace'}
			</Button>
		</form>

		{#snippet footer()}
			<button
				type="button"
				class="mode-switch"
				onclick={() => {
					mode = mode === 'signin' ? 'signup' : 'signin';
					errorMessage = '';
				}}
			>
				{mode === 'signin'
					? 'New developer? Create a Control Plane account'
					: 'Already have a developer account? Sign in'}
			</button>
		{/snippet}
	</Card>
</div>

<style>
	.auth-center {
		max-width: 480px;
		width: 100%;
		margin: 3.5rem auto;
		padding: 0 1.25rem;
	}

	.auth-form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.form-error {
		padding: 0.75rem 1rem;
		border-radius: 8px;
		background: rgba(239, 68, 68, 0.12);
		border: 1px solid rgba(239, 68, 68, 0.35);
		color: #fca5a5;
		font-size: 0.875rem;
	}

	.mode-switch {
		background: transparent;
		border: none;
		color: #818cf8;
		cursor: pointer;
		font-size: 0.875rem;
		padding: 0;
	}

	.mode-switch:hover {
		text-decoration: underline;
	}
</style>

