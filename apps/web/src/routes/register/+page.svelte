<script lang="ts">
	import AuthSplitShell from '$lib/components/AuthSplitShell.svelte';
	import { authApi } from '$lib/api/auth';
	import { setAuth, authStore } from '$lib/stores/auth.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { ApiError } from '$lib/api/client';
	import { goto } from '$app/navigation';
	import { onDestroy } from 'svelte';

	$effect(() => {
		if (authStore.initialized && authStore.isAuthenticated) {
			goto('/dashboard');
		}
	});

	let step = $state<'form' | 'otp'>('form');
	let email = $state('');
	let password = $state('');
	let displayName = $state('');
	let confirmPassword = $state('');
	let showPassword = $state(false);
	let loading = $state(false);
	let verifyLoading = $state(false);
	let resending = $state(false);
	let errors = $state<Record<string, string>>({});

	let otpDigits = $state<string[]>(['', '', '', '', '', '']);
	let otpInputRefs: (HTMLInputElement | null)[] = [];
	let resendCooldown = $state(60);
	let timerInterval: ReturnType<typeof setInterval> | null = null;

	function startResendTimer() {
		if (timerInterval) clearInterval(timerInterval);
		resendCooldown = 60;
		timerInterval = setInterval(() => {
			if (resendCooldown > 0) {
				resendCooldown--;
			} else if (timerInterval) {
				clearInterval(timerInterval);
				timerInterval = null;
			}
		}, 1000);
	}

	onDestroy(() => {
		if (timerInterval) clearInterval(timerInterval);
	});

	function validate(): boolean {
		errors = {};
		if (!displayName.trim()) errors.displayName = 'Display name is required';
		else if (displayName.trim().length < 2) errors.displayName = 'Minimum 2 characters';

		if (!email.trim()) errors.email = 'Email is required';
		else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) errors.email = 'Invalid email address';

		if (!password) errors.password = 'Password is required';
		else if (password.length < 8) errors.password = 'Minimum 8 characters';
		else if (!/[A-Z]/.test(password)) errors.password = 'Include at least one uppercase letter';
		else if (!/[0-9]/.test(password)) errors.password = 'Include at least one number';

		if (password !== confirmPassword) errors.confirmPassword = 'Passwords do not match';
		return Object.keys(errors).length === 0;
	}

	let fullOtp = $derived(otpDigits.join(''));

	async function handleInitiateRegister(e: SubmitEvent) {
		e.preventDefault();
		if (!validate()) return;

		loading = true;
		errors = {};
		try {
			await authApi.register({
				email: email.trim().toLowerCase(),
				password,
				display_name: displayName.trim()
			});
			step = 'otp';
			startResendTimer();
			setTimeout(() => otpInputRefs[0]?.focus(), 80);
		} catch (err) {
			if (err instanceof ApiError) {
				if (err.status === 409) errors.email = 'An account with this email already exists';
				else errors.form = err.message;
			} else {
				errors.form = 'An unexpected error occurred. Please try again.';
			}
		} finally {
			loading = false;
		}
	}

	async function handleVerifyOtp(e?: SubmitEvent) {
		if (e) e.preventDefault();
		if (fullOtp.length !== 6) {
			errors.otp = 'Enter all 6 verification digits';
			return;
		}

		verifyLoading = true;
		errors = {};
		try {
			const session = await authApi.verifyRegisterOtp({
				email: email.trim().toLowerCase(),
				code: fullOtp
			});
			setAuth(session.user);
			addToast('Account verified and created', 'success');
			goto('/dashboard');
		} catch (err) {
			errors.otp = err instanceof ApiError ? err.message : 'Verification failed';
		} finally {
			verifyLoading = false;
		}
	}

	async function handleResendOtp() {
		if (resendCooldown > 0 || resending) return;
		resending = true;
		errors = {};
		try {
			await authApi.resendRegisterOtp({ email: email.trim().toLowerCase() });
			addToast('Verification code resent', 'info');
			otpDigits = ['', '', '', '', '', ''];
			startResendTimer();
			otpInputRefs[0]?.focus();
		} catch (err) {
			errors.otp = err instanceof ApiError ? err.message : 'Failed to resend code';
		} finally {
			resending = false;
		}
	}

	function handleOtpInput(index: number, e: Event) {
		const target = e.target as HTMLInputElement;
		const val = target.value.replace(/\D/g, '');
		otpDigits[index] = val ? val.slice(-1) : '';
		if (val && index < 5) otpInputRefs[index + 1]?.focus();
		if (otpDigits.every((d) => d.length === 1)) handleVerifyOtp();
	}

	function handleOtpKeyDown(index: number, e: KeyboardEvent) {
		if (e.key === 'Backspace') {
			if (!otpDigits[index] && index > 0) {
				otpDigits[index - 1] = '';
				otpInputRefs[index - 1]?.focus();
			} else {
				otpDigits[index] = '';
			}
		} else if (e.key === 'ArrowLeft' && index > 0) {
			otpInputRefs[index - 1]?.focus();
		} else if (e.key === 'ArrowRight' && index < 5) {
			otpInputRefs[index + 1]?.focus();
		}
	}

	function handleOtpPaste(e: ClipboardEvent) {
		e.preventDefault();
		const digits = (e.clipboardData?.getData('text') || '').replace(/\D/g, '').slice(0, 6).split('');
		if (digits.length === 0) return;
		for (let i = 0; i < 6; i++) otpDigits[i] = digits[i] || '';
		otpInputRefs[Math.min(digits.length, 5)]?.focus();
		if (digits.length === 6) handleVerifyOtp();
	}
</script>

<svelte:head>
	<title>{step === 'form' ? 'Create Account' : 'Verify Email'} — Secure Auth</title>
</svelte:head>

<AuthSplitShell brandName="Secure Auth" topActionLabel="Sign in" topActionHref="/login">
	{#if step === 'form'}
		<h1 class="editorial-title">Create Account</h1>
		<p class="editorial-subtitle">Register a central identity with email verification.</p>

		{#if errors.form}
			<div class="editorial-alert" role="alert">{errors.form}</div>
		{/if}

		<form class="editorial-stack" onsubmit={handleInitiateRegister} novalidate>
			<div class="editorial-grid-2">
				<div class="editorial-field">
					<label class="editorial-label" for="reg-name">Full Name</label>
					<input
						id="reg-name"
						type="text"
						class="editorial-input"
						class:has-error={Boolean(errors.displayName)}
						bind:value={displayName}
						placeholder="Jane Doe"
						autocomplete="name"
						required
					/>
					{#if errors.displayName}
						<span class="editorial-field-error" role="alert">{errors.displayName}</span>
					{/if}
				</div>

				<div class="editorial-field">
					<label class="editorial-label" for="reg-email">Email</label>
					<input
						id="reg-email"
						type="email"
						class="editorial-input"
						class:has-error={Boolean(errors.email)}
						bind:value={email}
						placeholder="name@company.com"
						autocomplete="email"
						required
					/>
					{#if errors.email}
						<span class="editorial-field-error" role="alert">{errors.email}</span>
					{/if}
				</div>
			</div>

			<div class="editorial-grid-2">
				<div class="editorial-field">
					<label class="editorial-label" for="reg-password">Password</label>
					<div class="editorial-input-wrap">
						<input
							id="reg-password"
							type={showPassword ? 'text' : 'password'}
							class="editorial-input"
							class:has-error={Boolean(errors.password)}
							bind:value={password}
							placeholder="Min 8 chars, A-Z, 0-9"
							autocomplete="new-password"
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
					{#if errors.password}
						<span class="editorial-field-error" role="alert">{errors.password}</span>
					{/if}
				</div>

				<div class="editorial-field">
					<label class="editorial-label" for="reg-confirm">Confirm Password</label>
					<input
						id="reg-confirm"
						type={showPassword ? 'text' : 'password'}
						class="editorial-input"
						class:has-error={Boolean(errors.confirmPassword)}
						bind:value={confirmPassword}
						placeholder="Repeat password"
						autocomplete="new-password"
						required
					/>
					{#if errors.confirmPassword}
						<span class="editorial-field-error" role="alert">{errors.confirmPassword}</span>
					{/if}
				</div>
			</div>

			<div class="editorial-actions-footer">
				<span class="editorial-quiet-link">6-digit email OTP verification required</span>
				<button type="submit" class="editorial-circle-cta" disabled={loading}>
					{loading ? '...' : 'CONTINUE'}
				</button>
			</div>
		</form>
	{:else}
		<h1 class="editorial-title">Verify Email</h1>
		<p class="editorial-subtitle">Enter the 6-digit code sent to {email}</p>

		{#if errors.otp}
			<div class="editorial-alert" role="alert">{errors.otp}</div>
		{/if}

		<form onsubmit={handleVerifyOtp} novalidate>
			<div class="otp-row" onpaste={handleOtpPaste}>
				{#each [0, 1, 2, 3, 4, 5] as idx}
					<input
						bind:this={otpInputRefs[idx]}
						type="text"
						inputmode="numeric"
						maxlength="1"
						class="otp-box"
						value={otpDigits[idx]}
						oninput={(e) => handleOtpInput(idx, e)}
						onkeydown={(e) => handleOtpKeyDown(idx, e)}
						aria-label={`Digit ${idx + 1}`}
					/>
				{/each}
			</div>

			<div class="editorial-actions-footer">
				<div class="otp-links">
					{#if resendCooldown > 0}
						<span class="otp-timer">Resend in 0:{resendCooldown < 10 ? '0' : ''}{resendCooldown}</span>
					{:else}
						<button type="button" class="editorial-quiet-link" onclick={handleResendOtp} disabled={resending}>
							{resending ? 'Sending...' : 'Resend code'}
						</button>
					{/if}
					<button
						type="button"
						class="editorial-quiet-link"
						onclick={() => {
							step = 'form';
							errors = {};
						}}
					>
						Edit details
					</button>
				</div>

				<button type="submit" class="editorial-circle-cta" disabled={verifyLoading}>
					{verifyLoading ? '...' : 'VERIFY'}
				</button>
			</div>
		</form>
	{/if}
</AuthSplitShell>

<style>
	.otp-row {
		display: grid;
		grid-template-columns: repeat(6, 1fr);
		gap: 0.65rem;
		margin-bottom: 1.5rem;
	}

	.otp-box {
		height: 54px;
		text-align: center;
		font-family: var(--font-mono);
		font-size: 1.35rem;
		font-weight: 600;
		color: #ffffff;
		background-color: #161619;
		border: 1px solid #2e2e34;
		border-radius: 6px;
		outline: none;
	}

	.otp-box:focus {
		border-color: #ffffff;
	}

	.otp-links {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
	}

	.otp-timer {
		font-size: 0.78rem;
		color: #71717a;
		font-family: var(--font-mono);
	}
</style>
