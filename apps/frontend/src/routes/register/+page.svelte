<script lang="ts">
	import Input from '$lib/components/Input.svelte';
	import Button from '$lib/components/Button.svelte';
	import { authApi } from '$lib/api/auth';
	import { setAuth } from '$lib/stores/auth.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { ApiError } from '$lib/api/client';
	import { goto } from '$app/navigation';

	let email = $state('');
	let password = $state('');
	let displayName = $state('');
	let confirmPassword = $state('');
	let loading = $state(false);
	let errors = $state<Record<string, string>>({});

	function validate(): boolean {
		errors = {};
		if (!displayName.trim()) errors.displayName = 'Display name is required';
		else if (displayName.length < 2) errors.displayName = 'Must be at least 2 characters';

		if (!email) errors.email = 'Email is required';
		else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) errors.email = 'Invalid email address';

		if (!password) errors.password = 'Password is required';
		else if (password.length < 8) errors.password = 'Must be at least 8 characters';
		else if (!/[A-Z]/.test(password)) errors.password = 'Must contain at least one uppercase letter';
		else if (!/[0-9]/.test(password)) errors.password = 'Must contain at least one number';

		if (password !== confirmPassword) errors.confirmPassword = 'Passwords do not match';

		return Object.keys(errors).length === 0;
	}

	let passwordStrength = $derived(() => {
		let score = 0;
		if (password.length >= 8) score++;
		if (password.length >= 12) score++;
		if (/[A-Z]/.test(password)) score++;
		if (/[0-9]/.test(password)) score++;
		if (/[^A-Za-z0-9]/.test(password)) score++;
		return score;
	});

	let strengthLabel = $derived(() => {
		const s = passwordStrength();
		if (s <= 1) return { text: 'Weak', color: 'var(--color-error)' };
		if (s <= 2) return { text: 'Fair', color: 'var(--color-warning)' };
		if (s <= 3) return { text: 'Good', color: '#84cc16' };
		return { text: 'Strong', color: 'var(--color-success)' };
	});

	async function handleRegister(e: SubmitEvent) {
		e.preventDefault();
		if (!validate()) return;

		loading = true;
		try {
			const session = await authApi.register({
				email,
				password,
				display_name: displayName
			});
			setAuth(session.user, session.tokens);
			addToast('Account created! Welcome aboard 🎉', 'success');
			goto('/dashboard');
		} catch (err) {
			if (err instanceof ApiError) {
				if (err.status === 409) {
					errors.email = 'An account with this email already exists';
				} else {
					errors.form = err.message;
				}
			} else {
				errors.form = 'An unexpected error occurred. Please try again.';
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Create Account — AuthApp</title>
	<meta name="description" content="Create your free AuthApp account." />
</svelte:head>

<div class="auth-layout">
	<div class="auth-card glass-card">
		<div class="auth-logo">
			<div class="auth-logo-icon">🔐</div>
			<span class="auth-logo-text">AuthApp</span>
		</div>

		<h1 class="auth-title">Create an account</h1>
		<p class="auth-subtitle">Get started for free — no credit card required</p>

		<form class="auth-form" onsubmit={handleRegister} novalidate>
			{#if errors.form}
				<div class="form-error" role="alert">⚠ {errors.form}</div>
			{/if}

			<Input
				id="displayName"
				label="Display name"
				type="text"
				placeholder="Jane Doe"
				bind:value={displayName}
				error={errors.displayName}
				autocomplete="name"
				required
			/>

			<Input
				id="email"
				label="Email address"
				type="email"
				placeholder="you@example.com"
				bind:value={email}
				error={errors.email}
				autocomplete="email"
				required
			/>

			<div>
				<Input
					id="password"
					label="Password"
					type="password"
					placeholder="••••••••"
					bind:value={password}
					error={errors.password}
					autocomplete="new-password"
					required
				/>
				{#if password.length > 0}
					<div class="strength-bar">
						<div class="strength-segments">
							{#each [1, 2, 3, 4, 5] as level}
								<div
									class="segment"
									style="background: {level <= passwordStrength()
										? strengthLabel().color
										: 'var(--color-border)'}"
								></div>
							{/each}
						</div>
						<span class="strength-text" style="color: {strengthLabel().color}">
							{strengthLabel().text}
						</span>
					</div>
				{/if}
			</div>

			<Input
				id="confirmPassword"
				label="Confirm password"
				type="password"
				placeholder="••••••••"
				bind:value={confirmPassword}
				error={errors.confirmPassword}
				autocomplete="new-password"
				required
			/>

			<Button type="submit" variant="primary" full {loading}>
				{loading ? 'Creating account…' : 'Create Account'}
			</Button>

			<p class="terms">
				By creating an account, you agree to our
				<a href="/terms">Terms of Service</a> and
				<a href="/privacy">Privacy Policy</a>.
			</p>
		</form>

		<p class="auth-footer">
			Already have an account? <a href="/login">Sign in →</a>
		</p>
	</div>
</div>

<style>
	.form-error {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.25);
		border-radius: var(--radius-md);
		color: #fca5a5;
		font-size: 0.9rem;
	}

	.strength-bar {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		margin-top: var(--space-2);
	}

	.strength-segments {
		display: flex;
		gap: 4px;
		flex: 1;
	}

	.segment {
		height: 4px;
		flex: 1;
		border-radius: 2px;
		transition: background 0.3s ease;
	}

	.strength-text {
		font-size: 0.8125rem;
		font-weight: 600;
		min-width: 3.5rem;
		text-align: right;
	}

	.terms {
		font-size: 0.8125rem;
		color: var(--color-text-subtle);
		text-align: center;
	}

	.terms a { color: var(--color-primary-light); }
</style>
