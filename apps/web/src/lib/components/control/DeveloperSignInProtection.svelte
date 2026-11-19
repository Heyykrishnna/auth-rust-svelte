<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import Card from '$lib/components/Card.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import SelectionRow from '$lib/components/SelectionRow.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import {
		developerSecurityApi as api,
		type DeveloperProtection,
		type DeveloperTrustedDevice,
		type SecurityNotifications
	} from '$lib/api/developer-security';

	let {
		security,
		verified,
		bruteForce,
		refresh,
		handleError
	}: {
		security: DeveloperProtection;
		verified: boolean;
		bruteForce: boolean;
		refresh: () => Promise<void>;
		handleError: (error: unknown) => Promise<boolean>;
	} = $props();

	const id = $props.id();
	const date = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' });
	let label = $state('');
	let busy = $state(false);
	let error = $state('');
	let remove = $state<DeveloperTrustedDevice>();
	const currentBrowserTrusted = $derived(
		security.trusted_devices.some((device) => device.current)
	);

	async function perform(operation: () => Promise<unknown>, message: string) {
		if (busy) return;
		busy = true;
		error = '';
		try {
			await operation();
			addToast(message, 'success');
			await refresh();
		} catch (err) {
			if (!(await handleError(err))) {
				error =
					err instanceof Error
						? err.message
						: 'Could not update sign-in protection.';
			}
		} finally {
			busy = false;
		}
	}

	function trustBrowser(event: SubmitEvent) {
		event.preventDefault();
		void perform(async () => {
			await api.trustBrowser(label.trim());
			label = '';
		}, 'Browser trusted.');
	}

	function forgetBrowser() {
		const device = remove;
		if (!device) return;
		void perform(async () => {
			await api.forgetBrowser(device.id);
			remove = undefined;
		}, 'Browser removed and its other sessions ended.');
	}

	function changePreference(
		name: keyof SecurityNotifications,
		enabled: boolean
	) {
		return perform(
			() =>
				api.setNotifications({ ...security.notifications, [name]: enabled }),
			'Notification preferences saved.'
		);
	}
</script>

<section
	class="grid min-w-0 gap-4"
	aria-label="Sign-in protection and recovery"
>
	<Card
		title="Trusted browsers"
		subtitle="Recognize your browsers in sign-in activity. Trusted browsers still require 2FA when it’s enabled."
	>
		{#if security.trusted_devices.length > 0}
			<ul class="mb-3 list-none divide-y divide-brand-border p-0">
				{#each security.trusted_devices as device (device.id)}
					<li class="flex flex-wrap items-center justify-between gap-3 py-2">
						<div class="min-w-0 flex-1 basis-40">
							<p class="text-sm font-medium break-words">
								{device.label}{device.current ? ' · This browser' : ''}
							</p>
							<p class="mt-1 text-xs text-brand-text-secondary">
								Last seen {date.format(new Date(device.last_seen_at))} · Trust expires
								{date.format(new Date(device.expires_at))}
							</p>
						</div>
						<Button
							size="sm"
							variant="danger"
							disabled={busy || !verified}
							onclick={() => {
								error = '';
								remove = device;
							}}
						>
							Forget
						</Button>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="mb-3 text-sm text-brand-text-secondary">No trusted browsers.</p>
		{/if}

		{#if !currentBrowserTrusted}
			<form class="grid gap-2" onsubmit={trustBrowser}>
				<label class="text-sm font-medium" for={`${id}-browser-name`}>
					Name this browser
				</label>
				<div class="flex flex-wrap items-center gap-2">
					<input
						id={`${id}-browser-name`}
						class="min-h-10 w-full min-w-0 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 py-2 text-sm outline-none focus:border-action focus:ring-1 focus:ring-action"
						bind:value={label}
						maxlength="80"
						required
					/>
					<Button
						size="sm"
						type="submit"
						disabled={busy || !verified || !label.trim()}
					>
						Trust this browser
					</Button>
				</div>
			</form>
		{/if}
		{#if error && !remove}<p
				role="alert"
				class="mt-3 text-sm text-brand-danger-text"
			>
				{error}
			</p>{/if}
	</Card>

	<Card
		title="Security notifications"
		subtitle="Suspicious sign-ins appear in Login history. Choose which events also send email."
	>
		<div class="grid gap-2">
			<SelectionRow
				label="Successful sign-ins"
				checked={security.notifications.notify_new_sign_in}
				disabled={busy || !verified}
				onchange={(enabled) => changePreference('notify_new_sign_in', enabled)}
			/>
			<SelectionRow
				label="Suspicious sign-ins"
				checked={security.notifications.notify_suspicious_sign_in}
				disabled={busy || !verified}
				onchange={(enabled) =>
					changePreference('notify_suspicious_sign_in', enabled)}
			/>
		</div>
		{#if !security.delivery.email_configured}
			<p class="mt-3 text-sm text-brand-text-secondary">
				Email delivery requires SMTP configuration.
			</p>
		{/if}
	</Card>

	<Card
		title="Sign-in protection"
		subtitle="Signet checks for repeated failures, unusual networks, and changes in sign-in patterns."
	>
		<ul class="m-0 list-none divide-y divide-brand-border p-0 text-sm">
			<li class="grid gap-1 py-2 first:pt-0">
				<span class="font-medium">Brute force protection</span>
				<span class="text-xs text-brand-text-secondary">
					{bruteForce
						? 'Repeated failures temporarily limit password attempts.'
						: 'Password attempt limits are unavailable.'}
				</span>
			</li>
			<li class="grid gap-1 py-2">
				<span class="font-medium">Credential stuffing detection</span>
				<span class="text-xs text-brand-text-secondary">
					{security.credential_stuffing_protection
						? 'Networks that fail sign-in across many accounts are temporarily limited.'
						: 'Network attempt limits are unavailable.'}
				</span>
			</li>
			<li class="grid gap-1 py-2">
				<span class="font-medium">IP reputation and impossible travel</span>
				<span class="text-xs text-brand-text-secondary">
					{security.risk_coverage.ip_reputation_configured
						? 'IP reputation data is configured.'
						: 'IP reputation needs a network reputation feed.'}
					{security.risk_coverage.location_data_configured
						? 'Location data is configured for travel checks.'
						: 'Travel checks need IP location data.'}
				</span>
			</li>
			<li class="grid gap-1 py-2 last:pb-0">
				<span class="font-medium">Account takeover detection</span>
				<span class="text-xs text-brand-text-secondary">
					Multiple risk signals in a sign-in are flagged together in Login
					history.
				</span>
			</li>
		</ul>
	</Card>

	<Card
		title="Account recovery"
		subtitle="Reset your developer password using a one-time link sent to your sign-in email."
	>
		{#if security.delivery.recovery_configured}
			<Button size="sm" href="/control/login/recovery">
				Request a reset link
			</Button>
		{:else}
			<p class="text-sm text-brand-text-secondary">
				Password recovery requires SMTP and a configured console origin.
			</p>
		{/if}
	</Card>
</section>

<Modal
	open={remove !== undefined}
	title="Forget this browser?"
	description="This also ends its other developer sessions."
	confirmText="Forget browser"
	variant="danger"
	loading={busy}
	onconfirm={forgetBrowser}
	oncancel={() => (remove = undefined)}
>
	{#if error}<p role="alert" class="text-sm text-brand-danger-text">
			{error}
		</p>{/if}
</Modal>
