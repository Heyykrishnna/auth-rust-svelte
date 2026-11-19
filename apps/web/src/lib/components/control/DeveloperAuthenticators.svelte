<script lang="ts">
	import { onMount } from 'svelte';
	import Button from '$lib/components/Button.svelte';
	import Card from '$lib/components/Card.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import OtpInput from '$lib/components/OtpInput.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import {
		developerSecurityApi as api,
		supportsPasskeys,
		type DeveloperProtection
	} from '$lib/api/developer-security';

	let {
		security,
		verified,
		refresh,
		handleError
	}: {
		security: DeveloperProtection;
		verified: boolean;
		refresh: () => Promise<void>;
		handleError: (error: unknown) => Promise<boolean>;
	} = $props();

	const id = $props.id();
	const field =
		'min-h-10 w-full min-w-0 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 py-2 text-sm text-brand-text-primary outline-none focus:border-action focus:ring-1 focus:ring-action';
	let password = $state('');
	let secondFactor = $state('');
	let otp = $state('');
	let label = $state('');
	let pendingSetup = $state<{ secret: string; uri: string }>();
	let codes = $state<string[]>([]);
	let passkeySupported = $state(false);
	let busy = $state(false);
	let error = $state('');
	let action = $state<{
		title: string;
		detail: string;
		run: () => Promise<unknown>;
	}>();

	onMount(() => {
		passkeySupported = supportsPasskeys();
	});

	async function run(
		operation: () => Promise<unknown>,
		success?: string,
		reload = true
	) {
		if (busy) return;
		busy = true;
		error = '';
		try {
			await operation();
			if (success) addToast(success, 'success');
			if (reload) await refresh();
		} catch (err) {
			if (!(await handleError(err))) {
				error =
					err instanceof Error
						? err.message
						: 'Could not update security settings.';
			}
		} finally {
			busy = false;
		}
	}

	function verify(event: SubmitEvent) {
		event.preventDefault();
		void run(async () => {
			try {
				await api.verifyPassword(password, secondFactor.trim() || undefined);
			} finally {
				password = secondFactor = '';
			}
		}, 'Identity verified.');
	}

	async function copy(text: string) {
		try {
			await navigator.clipboard.writeText(text);
			addToast('Copied to clipboard.', 'success');
		} catch {
			addToast('Copy failed. Select the text and copy it manually.', 'warning');
		}
	}

	function submitOtp(event: SubmitEvent) {
		event.preventDefault();
		void run(async () => {
			codes = (await api.confirmTotp(otp)).codes;
			pendingSetup = undefined;
			otp = '';
		}, 'Two-factor authentication enabled.');
	}

	function addPasskey(event: SubmitEvent) {
		event.preventDefault();
		void run(async () => {
			await api.registerPasskey(label.trim());
			label = '';
		}, 'Passkey registered.');
	}

	function confirmAction() {
		const pending = action;
		if (!pending) return;
		void run(async () => {
			await pending.run();
			action = undefined;
		}, 'Security settings updated.');
	}
</script>

<section class="grid min-w-0 gap-4" aria-label="Sign-in methods">
	<div id="verify-developer-identity" tabindex="-1" class="scroll-mt-6">
		<Card
			title="Verify your identity"
			subtitle="Unlock sensitive changes on this session for five minutes."
		>
			{#if verified}
				<p role="status" class="text-sm text-brand-text-primary">
					Identity verified for this session.
				</p>
			{:else}
				<form class="grid gap-3" onsubmit={verify}>
					<label class="grid gap-1.5 text-sm font-medium">
						Current password
						<input
							class={field}
							type="password"
							autocomplete="current-password"
							bind:value={password}
							required
						/>
					</label>
					{#if security.totp_enabled}
						<label class="grid gap-1.5 text-sm font-medium">
							Authenticator or backup code
							<input
								class={field}
								autocomplete="one-time-code"
								bind:value={secondFactor}
								maxlength="27"
								required
							/>
						</label>
					{/if}
					<div class="flex flex-wrap gap-2">
						<Button type="submit" size="sm" disabled={busy}>
							Verify identity
						</Button>
						{#if security.passkeys_configured && passkeySupported && security.passkeys.length}
							<Button
								size="sm"
								disabled={busy}
								onclick={() => run(api.verifyPasskey, 'Identity verified.')}
							>
								Use a passkey
							</Button>
						{/if}
					</div>
				</form>
			{/if}
			{#if error}<p role="alert" class="mt-3 text-sm text-brand-danger-text">
					{error}
				</p>{/if}
		</Card>
	</div>

	<Card
		title="Two-factor authentication"
		subtitle="Require an authenticator or a one-time backup code when signing in."
	>
		{#if security.totp_enabled}
			<p class="mb-3 text-sm text-brand-text-secondary">
				An authenticator app is enrolled.
			</p>
			<Button
				size="sm"
				variant="danger"
				disabled={busy || !verified}
				onclick={() => {
					action = {
						title: 'Disable two-factor authentication?',
						detail:
							'This removes your authenticator and backup codes and signs out your other sessions.',
						run: async () => {
							await api.removeAuthenticator();
							codes = [];
						}
					};
				}}
			>
				Disable 2FA
			</Button>
		{:else if pendingSetup}
			<form class="grid gap-3" onsubmit={submitOtp}>
				<p class="text-sm text-brand-text-secondary">
					Enter this setup key in your authenticator app, then enter its
					six-digit code.
				</p>
				<div class="flex flex-wrap items-center gap-2">
					<code class="min-w-0 text-sm break-all">{pendingSetup.secret}</code>
					<Button
						size="sm"
						disabled={busy}
						onclick={() => copy(pendingSetup?.secret ?? '')}
					>
						Copy setup key
					</Button>
					<Button size="sm" href={pendingSetup.uri}>
						Open in authenticator
					</Button>
				</div>
				<OtpInput bind:value={otp} disabled={busy || !verified} />
				<div class="flex flex-wrap gap-2">
					<Button
						type="submit"
						size="sm"
						disabled={!verified || otp.length !== 6 || busy}
					>
						Confirm setup
					</Button>
					<Button
						size="sm"
						variant="danger"
						disabled={busy}
						onclick={() => {
							pendingSetup = undefined;
							otp = '';
						}}
					>
						Cancel
					</Button>
				</div>
			</form>
		{:else}
			<Button
				size="sm"
				disabled={busy || !verified}
				onclick={() =>
					run(
						async () => {
							pendingSetup = await api.beginTotp();
							otp = '';
						},
						undefined,
						false
					)}
			>
				Set up authenticator
			</Button>
		{/if}
	</Card>

	<Card
		title="Backup codes"
		subtitle="Each code can replace your authenticator code once."
	>
		{#if security.totp_enabled}
			<p class="mb-3 text-sm text-brand-text-secondary">
				{security.backup_codes_remaining} unused codes
			</p>
			<Button
				size="sm"
				disabled={busy || !verified}
				onclick={() => {
					action = {
						title: 'Replace backup codes?',
						detail: 'Any unused codes from the previous set will stop working.',
						run: async () => {
							codes = (await api.rotateBackupCodes()).codes;
						}
					};
				}}
			>
				Replace backup codes
			</Button>
		{:else}
			<p class="text-sm text-brand-text-secondary">
				Set up an authenticator to generate backup codes.
			</p>
		{/if}
	</Card>

	<Card
		title="Passkeys"
		subtitle="Register a passkey to verify your identity for security changes."
	>
		{#if security.passkeys.length}
			<ul class="mb-3 list-none divide-y divide-brand-border p-0">
				{#each security.passkeys as passkey (passkey.id)}
					<li class="flex flex-wrap items-center justify-between gap-2 py-2">
						<span class="min-w-0 text-sm break-words">{passkey.label}</span>
						<Button
							size="sm"
							variant="danger"
							disabled={busy || !verified}
							onclick={() => {
								action = {
									title: 'Remove this passkey?',
									detail:
										'This passkey will stop verifying your developer identity.',
									run: () => api.removePasskey(passkey.id)
								};
							}}
						>
							Remove
						</Button>
					</li>
				{/each}
			</ul>
		{/if}
		{#if !security.passkeys_configured}
			<p class="text-sm text-brand-text-secondary">
				Passkeys require a configured secure console origin.
			</p>
		{:else if !passkeySupported}
			<p class="text-sm text-brand-text-secondary">
				Passkey registration requires a browser with WebAuthn support.
			</p>
		{:else}
			<form class="grid gap-2" onsubmit={addPasskey}>
				<label for={`${id}-key-name`} class="text-sm font-medium">
					Passkey name
				</label>
				<div class="flex flex-wrap items-center gap-2">
					<input
						id={`${id}-key-name`}
						class={field}
						bind:value={label}
						maxlength="80"
						required
					/>
					<Button
						size="sm"
						type="submit"
						disabled={busy || !verified || !label.trim()}
					>
						Register passkey
					</Button>
				</div>
			</form>
		{/if}
	</Card>
</section>

<Modal
	open={action !== undefined}
	title={action?.title ?? 'Confirm change'}
	description={action?.detail}
	variant="danger"
	loading={busy}
	onconfirm={confirmAction}
	oncancel={() => (action = undefined)}
>
	{#if error}<p role="alert" class="text-sm text-brand-danger-text">
			{error}
		</p>{/if}
</Modal>

<Modal
	open={codes.length > 0 && action === undefined}
	title="Save your backup codes"
	description="Store these somewhere safe. This is the only time the full codes are shown."
	confirmText="Done"
	cancelText="Close"
	onconfirm={() => (codes = [])}
	oncancel={() => (codes = [])}
>
	<ul class="mb-3 grid list-none gap-2 font-mono text-sm sm:grid-cols-2">
		{#each codes as code (code)}<li class="text-xs break-all">{code}</li>{/each}
	</ul>
	<Button size="sm" onclick={() => copy(codes.join('\n'))}>Copy codes</Button>
</Modal>
