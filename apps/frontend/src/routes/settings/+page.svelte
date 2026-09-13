<script lang="ts">
	import Card from '$lib/components/Card.svelte';
	import Button from '$lib/components/Button.svelte';
	import Input from '$lib/components/Input.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { authStore, setAuth } from '$lib/stores/auth.svelte';
	import { addToast } from '$lib/components/Toast.svelte';
	import { usersApi } from '$lib/api/users';
	import { sessionsApi } from '$lib/api/sessions';
	import { authApi } from '$lib/api/auth';
	import type { SessionItem } from '$lib/api/types';
	import { ApiError } from '$lib/api/client';
	import { onMount } from 'svelte';

	let displayName = $state('');
	let profileLoading = $state(false);
	let profileError = $state('');

	let sessions = $state<SessionItem[]>([]);
	let sessionsLoading = $state(false);
	let sessionToRevoke = $state<SessionItem | null>(null);
	let revokingSession = $state(false);

	let sendingReset = $state(false);

	onMount(async () => {
		if (authStore.user) {
			displayName = authStore.user.display_name;
		}
		await loadSessions();
	});

	$effect(() => {
		if (authStore.user && !displayName) {
			displayName = authStore.user.display_name;
		}
	});

	async function handleUpdateProfile(e: SubmitEvent) {
		e.preventDefault();
		if (!displayName.trim() || displayName.trim().length < 2) {
			profileError = 'Display name must be at least 2 characters';
			return;
		}

		profileError = '';
		profileLoading = true;
		try {
			const updated = await usersApi.updateProfile({ display_name: displayName.trim() });
			setAuth(updated);
			addToast('Profile updated successfully', 'success');
		} catch (err) {
			if (err instanceof ApiError) {
				profileError = err.message;
			} else {
				profileError = 'Failed to update profile';
			}
		} finally {
			profileLoading = false;
		}
	}

	async function loadSessions() {
		sessionsLoading = true;
		try {
			sessions = await sessionsApi.listSessions();
		} catch (err) {
			if (err instanceof ApiError) {
				addToast(err.message, 'error');
			}
		} finally {
			sessionsLoading = false;
		}
	}

	function confirmRevokeSession(session: SessionItem) {
		sessionToRevoke = session;
	}

	async function executeRevokeSession() {
		if (!sessionToRevoke) return;
		revokingSession = true;
		try {
			await sessionsApi.revokeSession(sessionToRevoke.id);
			sessions = sessions.filter((s) => s.id !== sessionToRevoke?.id);
			addToast('Session revoked successfully', 'success');
			sessionToRevoke = null;
		} catch (err) {
			if (err instanceof ApiError) {
				addToast(err.message, 'error');
			} else {
				addToast('Failed to revoke session', 'error');
			}
		} finally {
			revokingSession = false;
		}
	}

	async function handleSendPasswordReset() {
		if (!authStore.user?.email) return;
		sendingReset = true;
		try {
			await authApi.forgotPassword({ email: authStore.user.email });
			addToast('Password reset link sent to your email', 'success');
		} catch (err) {
			if (err instanceof ApiError) {
				addToast(err.message, 'error');
			} else {
				addToast('Failed to request password reset', 'error');
			}
		} finally {
			sendingReset = false;
		}
	}

	function formatTime(iso: string): string {
		try {
			return new Date(iso).toLocaleString();
		} catch {
			return iso;
		}
	}

	function parseDevice(ua: string | null): string {
		if (!ua) return 'Unknown Device';
		if (ua.includes('Macintosh')) return 'macOS Device';
		if (ua.includes('Windows')) return 'Windows PC';
		if (ua.includes('Linux')) return 'Linux Device';
		if (ua.includes('iPhone') || ua.includes('iPad')) return 'iOS Device';
		if (ua.includes('Android')) return 'Android Device';
		return 'Web Browser';
	}
</script>

<svelte:head>
	<title>Settings — AuthPlatform</title>
	<meta name="description" content="Manage your account profile, active sessions, and security preferences." />
</svelte:head>

<div class="settings-container container">
	<div class="settings-header">
		<h1 class="settings-title">Account Settings</h1>
		<p class="settings-subtitle">
			Manage your profile credentials and connected sessions.
		</p>
	</div>

	<div class="settings-grid">
		<Card title="Profile Information" subtitle="Update your public display identity">
			<form onsubmit={handleUpdateProfile} class="settings-form">
				{#if profileError}
					<div class="form-error" role="alert">
						<span>⚠</span> {profileError}
					</div>
				{/if}

				<div class="form-group">
					<label class="form-label" for="email">Account Email</label>
					<input
						id="email"
						type="email"
						value={authStore.user?.email || ''}
						disabled
						class="form-input input-disabled"
					/>
					<span class="form-hint">Email address cannot be changed directly.</span>
				</div>

				<Input
					id="displayName"
					label="Display Name"
					bind:value={displayName}
					required
					placeholder="Jane Doe"
					hint="This name appears in greetings and team interfaces."
				/>

				<div class="form-actions">
					<Button type="submit" variant="primary" loading={profileLoading}>
						Save Changes
					</Button>
				</div>
			</form>
		</Card>

		<Card title="Security & Authentication" subtitle="Cryptographic credential management">
			<div class="security-items">
				<div class="security-item">
					<div class="security-info">
						<span class="security-name">Password Reset</span>
						<span class="security-desc">Receive a secure one-time link to update your password.</span>
					</div>
					<Button
						variant="outline"
						size="sm"
						onclick={handleSendPasswordReset}
						loading={sendingReset}
					>
						Send Reset Email
					</Button>
				</div>

				<div class="security-item">
					<div class="security-info">
						<span class="security-name">Email Verification</span>
						<span class="security-desc">
							{authStore.user?.email_verified
								? 'Your email address has been verified.'
								: 'Verification required to access all platform features.'}
						</span>
					</div>
					{#if authStore.user?.email_verified}
						<Badge variant="success">Verified</Badge>
					{:else}
						<Badge variant="warning">Pending</Badge>
					{/if}
				</div>

				<div class="security-item">
					<div class="security-info">
						<span class="security-name">Session Tokens</span>
						<span class="security-desc">Tokens are isolated in HttpOnly, SameSite=Lax cookies.</span>
					</div>
					<Badge variant="primary">HttpOnly Secure</Badge>
				</div>
			</div>
		</Card>

		<div class="sessions-section">
			<Card
				title="Active Login Sessions"
				subtitle="Devices currently authenticated to your account via the Rust Auth API"
			>
				{#snippet headerActions()}
					<Button variant="ghost" size="sm" onclick={loadSessions} loading={sessionsLoading}>
						Refresh
					</Button>
				{/snippet}

				{#if sessionsLoading && sessions.length === 0}
					<div class="loading-state">Loading active sessions...</div>
				{:else if sessions.length === 0}
					<div class="empty-state">No other active sessions detected.</div>
				{:else}
					<div class="sessions-list">
						{#each sessions as session (session.id)}
							<div class="session-card" class:session-current={session.is_current}>
								<div class="session-icon">
									💻
								</div>
								<div class="session-details">
									<div class="session-title-line">
										<span class="session-device">{parseDevice(session.user_agent)}</span>
										{#if session.is_current}
											<Badge variant="success" size="sm" dot>Current Session</Badge>
										{/if}
									</div>
									<div class="session-meta-line">
										<span>IP: {session.ip_address || '127.0.0.1'}</span>
										<span>•</span>
										<span>Last active: {formatTime(session.last_used_at)}</span>
									</div>
									{#if session.user_agent}
										<div class="session-ua">{session.user_agent}</div>
									{/if}
								</div>
								<div class="session-actions">
									{#if !session.is_current}
										<Button
											variant="danger"
											size="sm"
											onclick={() => confirmRevokeSession(session)}
										>
											Revoke
										</Button>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</Card>
		</div>
	</div>
</div>

<Modal
	open={sessionToRevoke !== null}
	title="Revoke Active Session?"
	description="This action will terminate the session on that device immediately. The device will be signed out from the Rust API."
	confirmText="Revoke Session"
	variant="danger"
	loading={revokingSession}
	onconfirm={executeRevokeSession}
	oncancel={() => (sessionToRevoke = null)}
/>

<style>
	.container {
		max-width: 1000px;
		margin: 0 auto;
		padding: 2rem 1.5rem 4rem;
	}

	.settings-header {
		margin-bottom: 2rem;
	}

	.settings-title {
		font-size: 2rem;
		font-weight: 700;
		color: #ffffff;
		margin: 0 0 0.4rem;
		letter-spacing: -0.02em;
	}

	.settings-subtitle {
		margin: 0;
		color: #94a3b8;
		font-size: 1rem;
	}

	.settings-grid {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	.settings-form {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.form-label {
		font-size: 0.875rem;
		font-weight: 500;
		color: #cbd5e1;
	}

	.form-input {
		background: rgba(15, 23, 42, 0.6);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		padding: 0.65rem 0.85rem;
		color: #f1f5f9;
		font-size: 0.95rem;
	}

	.input-disabled {
		opacity: 0.6;
		cursor: not-allowed;
		background: rgba(0, 0, 0, 0.2);
	}

	.form-hint {
		font-size: 0.8rem;
		color: #64748b;
	}

	.form-error {
		padding: 0.75rem 1rem;
		background: rgba(239, 68, 68, 0.12);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		color: #f87171;
		font-size: 0.9rem;
	}

	.form-actions {
		margin-top: 0.5rem;
	}

	.security-items {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.security-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding-bottom: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	.security-item:last-child {
		border-bottom: none;
		padding-bottom: 0;
	}

	.security-info {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.security-name {
		font-weight: 600;
		color: #f1f5f9;
		font-size: 0.95rem;
	}

	.security-desc {
		color: #94a3b8;
		font-size: 0.85rem;
	}

	.sessions-list {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.session-card {
		display: flex;
		align-items: center;
		gap: 1.25rem;
		padding: 1rem 1.25rem;
		border-radius: 10px;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.05);
	}

	.session-card.session-current {
		border-color: rgba(34, 197, 94, 0.3);
		background: rgba(34, 197, 94, 0.03);
	}

	.session-icon {
		font-size: 1.5rem;
		flex-shrink: 0;
	}

	.session-details {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		overflow: hidden;
	}

	.session-title-line {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.session-device {
		font-weight: 600;
		color: #f8fafc;
		font-size: 0.95rem;
	}

	.session-meta-line {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.8rem;
		color: #94a3b8;
	}

	.session-ua {
		font-size: 0.75rem;
		color: #64748b;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.loading-state,
	.empty-state {
		text-align: center;
		padding: 2rem;
		color: #94a3b8;
	}
</style>
