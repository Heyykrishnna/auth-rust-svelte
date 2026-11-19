<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { ApiError } from '$lib/api/client';
	import {
		controlApi,
		type DeveloperSecurityOverview,
		type DeveloperSession
	} from '$lib/api/control';
	import Button from '$lib/components/Button.svelte';
	import Card from '$lib/components/Card.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { setDeveloper } from '$lib/stores/control.svelte';
	import SectionHeader from './SectionHeader.svelte';
	import DeveloperAuthenticators from './DeveloperAuthenticators.svelte';
	import DeveloperSignInProtection from './DeveloperSignInProtection.svelte';

	let overview = $state<DeveloperSecurityOverview>();
	let verified = $state(false);

	$effect(() => {
		const until = overview?.verified_until;
		const remaining = until ? new Date(until).getTime() - Date.now() : 0;
		verified = remaining > 0;
		if (remaining > 0) {
			const timer = setTimeout(() => {
				verified = false;
			}, remaining);
			return () => clearTimeout(timer);
		}
	});
	let loading = $state(true);
	let overviewError = $state('');
	let currentPassword = $state('');
	let newPassword = $state('');
	let confirmation = $state('');
	let changingPassword = $state(false);
	let passwordError = $state('');
	let selectedSession = $state<DeveloperSession>();
	let revoking = $state(false);
	let sessionError = $state('');
	const panelId = $props.id();
	const date = new Intl.DateTimeFormat(undefined, {
		dateStyle: 'medium',
		timeStyle: 'short'
	});
	const activityLabel: Record<string, string> = {
		DEVELOPER_REGISTERED: 'Developer account created',
		DEVELOPER_SIGNED_IN: 'Signed in',
		DEVELOPER_SIGN_IN_FAILED: 'Failed sign-in',
		DEVELOPER_SESSION_REVOKED: 'Session ended',
		DEVELOPER_PASSWORD_CHANGED: 'Password updated',
		DEVELOPER_PASSWORD_RESET: 'Password reset by email',
		DEVELOPER_MFA_ENABLED: 'Authenticator enabled',
		DEVELOPER_MFA_DISABLED: 'Authenticator removed',
		DEVELOPER_BACKUP_CODES_REPLACED: 'Backup codes replaced',
		DEVELOPER_BACKUP_CODE_USED: 'Backup code used',
		DEVELOPER_PASSKEY_ADDED: 'Passkey registered',
		DEVELOPER_PASSKEY_REMOVED: 'Passkey removed',
		DEVELOPER_DEVICE_TRUSTED: 'Browser trusted',
		DEVELOPER_DEVICE_FORGOTTEN: 'Browser forgotten',
		DEVELOPER_NOTIFICATIONS_UPDATED: 'Notification preferences updated',
		DEVELOPER_IDENTITY_VERIFIED: 'Identity verified',
		DEVELOPER_SUSPICIOUS_SIGN_IN: 'Suspicious sign-in',
		DEVELOPER_CREDENTIAL_STUFFING: 'Network sign-in attempts limited'
	};
	const riskLabel: Record<string, string> = {
		backup_code_used: 'A backup code was used',
		recent_failed_sign_ins: 'Several prior sign-in attempts failed',
		ip_reputation: 'The network has an elevated reputation risk',
		unfamiliar_browser:
			'An unfamiliar browser signed in from a different network',
		impossible_travel:
			'The reported locations changed faster than local travel would allow',
		possible_account_takeover:
			'Multiple indicators suggest possible account takeover'
	};
	const inputClass =
		'min-h-10 w-full min-w-0 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 py-2 text-sm text-brand-text-primary outline-none focus:border-neutral-300 focus:ring-1 focus:ring-neutral-300';

	onMount(() => {
		void load();
	});

	async function handleSessionExpiry(error: unknown) {
		if (
			!(error instanceof ApiError) ||
			error.status !== 401 ||
			error.code === 'INVALID_CREDENTIALS'
		)
			return false;
		setDeveloper(null);
		const returnTo = page.url.pathname + page.url.search;
		await goto(`/control/login?${new URLSearchParams({ returnTo })}`, {
			replace: true
		});
		return true;
	}

	async function handleSecurityError(error: unknown) {
		if (error instanceof ApiError && error.code === 'STEP_UP_REQUIRED') {
			if (overview) overview = { ...overview, verified_until: null };
			addToast('Verify your identity to continue.', 'warning');
			document.getElementById('verify-developer-identity')?.focus();
			return true;
		}
		if (
			error instanceof ApiError &&
			['INVALID_SECOND_FACTOR', 'SECOND_FACTOR_REQUIRED'].includes(
				error.code ?? ''
			)
		) {
			return false;
		}
		return handleSessionExpiry(error);
	}

	async function load() {
		overviewError = '';
		loading = true;
		try {
			overview = await controlApi.securityOverview();
		} catch (error) {
			if (!(await handleSessionExpiry(error))) {
				overviewError =
					'Security activity could not be loaded. Try again in a moment.';
			}
		} finally {
			loading = false;
		}
	}

	async function changePassword(event: SubmitEvent) {
		event.preventDefault();
		if (changingPassword) return;
		passwordError = '';
		if (newPassword !== confirmation) {
			passwordError = 'Passwords do not match.';
			return;
		}
		changingPassword = true;
		try {
			await controlApi.changePassword({
				current_password: currentPassword,
				new_password: newPassword
			});
			newPassword = confirmation = '';
			addToast(
				'Password updated. Other developer sessions were signed out.',
				'success'
			);
			await load();
		} catch (error) {
			if (!(await handleSecurityError(error))) {
				passwordError =
					error instanceof Error
						? error.message
						: 'Could not update your password.';
			}
		} finally {
			currentPassword = '';
			changingPassword = false;
		}
	}

	async function revokeSession() {
		const session = selectedSession;
		if (!session || revoking) return;
		revoking = true;
		sessionError = '';
		try {
			if (session.current) {
				await controlApi.signOut();
				setDeveloper(null);
				await goto('/control/login', { replace: true });
			} else {
				await controlApi.revokeDeveloperSession(session.id);
				selectedSession = undefined;
				addToast('Developer session ended.', 'success');
				await load();
			}
		} catch (error) {
			if (!(await handleSecurityError(error))) {
				sessionError =
					error instanceof Error
						? error.message
						: 'Could not end this session.';
			}
		} finally {
			revoking = false;
		}
	}
</script>

<div class="workspace-theme mx-auto grid w-full max-w-6xl min-w-0 gap-6">
	<SectionHeader
		title="Account security"
		description="Manage how you sign in, where you’re signed in, and how Signet alerts you about unusual activity."
	/>

	{#if overviewError}
		<div
			class="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-neutral-700 bg-neutral-900 p-4"
		>
			<p class="text-sm text-neutral-200" role="alert">{overviewError}</p>
			<Button size="sm" onclick={load}>Retry</Button>
		</div>
	{/if}

	{#if overview && !verified}
		<p class="text-sm text-brand-text-secondary">
			Verify your identity to enable changes to authenticators, trusted
			browsers, and notifications.
		</p>
	{/if}

	<section
		class="grid min-w-0 items-start gap-4 lg:grid-cols-2"
		aria-label="Sign-in and sessions"
	>
		<Card
			title="Change password"
			subtitle="Changing your password signs out your other developer sessions."
		>
			<form class="grid gap-4 text-sm" onsubmit={changePassword}>
				{#if passwordError}<p class="text-sm text-neutral-200" role="alert">
						{passwordError}
					</p>{/if}
				<label class="grid gap-1.5 font-medium">
					Current password
					<input
						class={inputClass}
						type="password"
						bind:value={currentPassword}
						autocomplete="current-password"
						required
					/>
				</label>
				<label class="grid gap-1.5 font-medium">
					New password
					<input
						class={inputClass}
						type="password"
						bind:value={newPassword}
						autocomplete="new-password"
						minlength="12"
						maxlength="128"
						aria-describedby={`${panelId}-password-length`}
						required
					/>
					<span
						id={`${panelId}-password-length`}
						class="text-xs font-normal text-brand-text-secondary"
					>
						12–128 characters
					</span>
				</label>
				<label class="grid gap-1.5 font-medium">
					Confirm new password
					<input
						class={inputClass}
						type="password"
						bind:value={confirmation}
						autocomplete="new-password"
						minlength="12"
						maxlength="128"
						required
					/>
				</label>
				<div class="justify-self-start">
					<Button type="submit" loading={changingPassword}>
						Update password
					</Button>
				</div>
			</form>
		</Card>

		<Card
			title="Sessions and devices"
			subtitle="See where you’re signed in and end any session you no longer use."
		>
			{#if loading && !overview}
				<p role="status" class="text-sm text-brand-text-secondary">
					Loading sessions…
				</p>
			{:else if overview?.sessions.length}
				<ul class="m-0 list-none divide-y divide-brand-border p-0">
					{#each overview.sessions as session (session.id)}
						<li
							class="flex flex-wrap items-start justify-between gap-3 py-3 first:pt-0 last:pb-0"
						>
							<div class="grid min-w-0 flex-1 basis-44 gap-1">
								<p class="text-sm font-medium">
									{session.current
										? 'This session'
										: 'Developer session'}{#if session.trusted_device}
										· {session.trusted_device}{/if}
								</p>
								{#if session.user_agent}<p
										class="text-xs break-words text-brand-text-secondary"
									>
										{session.user_agent}
									</p>{/if}
								<p class="text-xs text-brand-text-secondary">
									{#if session.ip_address}{session.ip_address} ·
									{/if}
									<time datetime={session.created_at}>
										{date.format(new Date(session.created_at))}
									</time>
								</p>
							</div>
							<Button
								size="sm"
								variant="danger"
								onclick={() => {
									selectedSession = session;
									sessionError = '';
								}}
							>
								{session.current ? 'Sign out' : 'End session'}
							</Button>
						</li>
					{/each}
				</ul>
			{:else if overview}
				<p class="text-sm text-brand-text-secondary">No active sessions.</p>
			{:else}
				<p class="text-sm text-brand-text-secondary">
					Session activity is unavailable.
				</p>
			{/if}
		</Card>
	</section>

	{#if overview}
		<div class="grid min-w-0 items-start gap-4 lg:grid-cols-2">
			<DeveloperAuthenticators
				security={overview}
				{verified}
				refresh={load}
				handleError={handleSecurityError}
			/>
			<DeveloperSignInProtection
				security={overview}
				{verified}
				bruteForce={overview.brute_force_protection}
				refresh={load}
				handleError={handleSecurityError}
			/>
		</div>
	{/if}

	<Card
		title="Login history"
		subtitle="Recent sign-ins, password changes, and session activity for your developer account."
	>
		{#if loading && !overview}
			<p role="status" class="text-sm text-brand-text-secondary">
				Loading login history…
			</p>
		{:else if overview?.recent_activity.length}
			<ul class="m-0 list-none divide-y divide-brand-border p-0">
				{#each overview.recent_activity as activity (activity.id)}
					<li class="grid min-w-0 gap-1 py-3 first:pt-0 last:pb-0">
						<div
							class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1"
						>
							<p class="text-sm font-medium">
								{activityLabel[activity.event] ?? 'Developer account activity'}
							</p>
							<time
								class="text-xs text-brand-text-secondary"
								datetime={activity.created_at}
							>
								{date.format(new Date(activity.created_at))}
							</time>
						</div>
						{#if activity.ip_address || activity.user_agent}
							<p class="text-xs break-words text-brand-text-secondary">
								{[activity.ip_address, activity.user_agent]
									.filter(Boolean)
									.join(' · ')}
							</p>
						{/if}
						{#if activity.flags.length}
							<ul class="grid list-none gap-1 p-0">
								{#each activity.flags as flag}
									{#if riskLabel[flag]}<li
											class="text-xs text-brand-text-secondary"
										>
											{riskLabel[flag]}
										</li>{/if}
								{/each}
							</ul>
						{/if}
					</li>
				{/each}
			</ul>
		{:else if overview}
			<p class="text-sm text-brand-text-secondary">
				No recent developer activity.
			</p>
		{:else}
			<p class="text-sm text-brand-text-secondary">
				Login history is unavailable.
			</p>
		{/if}
	</Card>

	<Modal
		open={selectedSession !== undefined}
		title={selectedSession?.current ? 'Sign out?' : 'End this session?'}
		description={selectedSession?.current
			? 'You’ll need to sign in again to use the Developer Console.'
			: 'The device will need to sign in again to access the Developer Console.'}
		confirmText={selectedSession?.current ? 'Sign out' : 'End session'}
		variant="danger"
		loading={revoking}
		onconfirm={revokeSession}
		oncancel={() => (selectedSession = undefined)}
	>
		{#if sessionError}<p class="text-sm text-neutral-200" role="alert">
				{sessionError}
			</p>{/if}
	</Modal>
</div>
