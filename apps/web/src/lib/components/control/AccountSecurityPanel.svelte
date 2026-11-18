<script lang="ts">
	import SectionHeader from './SectionHeader.svelte';
	import Button from '$lib/components/Button.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { controlApi } from '$lib/api/control';

	let currentPassword = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let changingPassword = $state(false);
	let passwordError = $state('');
	const fieldClass =
		'min-h-10 min-w-0 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 py-2 outline-none focus:border-brand-lime focus:ring-1 focus:ring-brand-lime';

	async function handlePasswordChange(e: SubmitEvent) {
		e.preventDefault();
		passwordError = '';
		if (newPassword.length < 12) {
			passwordError = 'New password must be at least 12 characters.';
			return;
		}
		if (newPassword !== confirmPassword) {
			passwordError = 'Password confirmation does not match.';
			return;
		}
		changingPassword = true;
		try {
			await controlApi.changePassword({
				current_password: currentPassword,
				new_password: newPassword
			});
			currentPassword = '';
			newPassword = '';
			confirmPassword = '';
			addToast('Developer account password updated', 'success');
		} catch (err) {
			passwordError =
				err instanceof Error ? err.message : 'Password update failed';
		} finally {
			changingPassword = false;
		}
	}
</script>

<SectionHeader
	title="Account security"
	description="Update the password for your Signet developer account."
/>

<section
	class="grid max-w-lg gap-4 rounded-xl border border-brand-border bg-brand-surface-card p-5 sm:p-6"
	aria-label="Change password"
>
	<h2 class="font-heading text-lg font-bold">Change password</h2>
	{#if passwordError}
		<p class="text-sm text-brand-danger-text" role="alert">{passwordError}</p>
	{/if}
	<form class="grid gap-4 text-sm" onsubmit={handlePasswordChange}>
		<label class="grid gap-1.5 font-medium">
			Current password
			<input
				class={fieldClass}
				type="password"
				bind:value={currentPassword}
				autocomplete="current-password"
				required
			/>
		</label>
		<label class="grid gap-1.5 font-medium">
			New password
			<input
				class={fieldClass}
				type="password"
				bind:value={newPassword}
				autocomplete="new-password"
				minlength="12"
				aria-describedby="password-length"
				required
			/>
			<span
				id="password-length"
				class="text-xs font-normal text-brand-text-secondary"
			>
				At least 12 characters
			</span>
		</label>
		<label class="grid gap-1.5 font-medium">
			Confirm new password
			<input
				class={fieldClass}
				type="password"
				bind:value={confirmPassword}
				autocomplete="new-password"
				minlength="12"
				required
			/>
		</label>
		<div class="justify-self-end">
			<Button
				type="submit"
				loading={changingPassword}
				disabled={changingPassword}
			>
				Update password
			</Button>
		</div>
	</form>
</section>
