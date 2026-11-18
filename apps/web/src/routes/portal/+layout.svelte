<script lang="ts">
	let { children }: { children: import('svelte').Snippet } = $props();
</script>

<div class="portal-shell">
	<main class="portal-card">
		{@render children()}
	</main>
	<footer class="portal-footer">
		<span>Secured by Central Identity Provider (OIDC + PKCE)</span>
	</footer>
</div>

<style>
	.portal-shell {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 2rem 1rem;
		background: radial-gradient(circle at top, #181834 0%, #090912 70%);
		color: #f8fafc;
	}

	.portal-card {
		width: 100%;
		max-width: 440px;
		background: rgba(18, 18, 38, 0.92);
		border: 1px solid rgba(148, 163, 184, 0.2);
		border-radius: 16px;
		padding: 2rem;
		box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
	}

	.portal-footer {
		margin-top: 1.5rem;
		font-size: 0.78rem;
		color: #94a3b8;
	}
</style>

