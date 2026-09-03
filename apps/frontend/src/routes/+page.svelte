<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import { authStore } from '$lib/stores/auth.svelte';
	import { goto } from '$app/navigation';

	$effect(() => {
		if (authStore.initialized && authStore.isAuthenticated) {
			goto('/dashboard');
		}
	});
</script>

<svelte:head>
	<title>AuthApp — Secure Authentication Platform</title>
	<meta name="description" content="Production-grade authentication with Rust and SvelteKit. JWT, OIDC, sessions — all in one." />
</svelte:head>

<!-- Hero Section -->
<main class="landing">
	<header class="landing-header">
		<div class="logo">
			<div class="logo-icon">🔐</div>
			<span class="logo-text">AuthApp</span>
		</div>
		<nav class="landing-nav">
			<Button href="/login" variant="ghost">Sign In</Button>
			<Button href="/register" variant="primary">Get Started</Button>
		</nav>
	</header>

	<section class="hero animate-fade-in-up">
		<div class="hero-badge">
			<span class="badge badge-info">✦ Built with Rust + SvelteKit</span>
		</div>

		<h1 class="hero-title">
			Authentication<br />
			<span class="gradient-text">Without Compromise</span>
		</h1>

		<p class="hero-description">
			Production-grade auth system built on Rust (Axum), SvelteKit, PostgreSQL, and Redis.
			JWT tokens, OIDC social login, session management — all secured and observable.
		</p>

		<div class="hero-actions">
			<Button href="/register" variant="primary" size="lg">
				Start for Free →
			</Button>
			<Button href="/login" variant="outline" size="lg">
				Sign In
			</Button>
		</div>

		<div class="hero-stats">
			<div class="stat-item">
				<span class="stat-number">~2ms</span>
				<span class="stat-label">Auth Latency</span>
			</div>
			<div class="stat-divider"></div>
			<div class="stat-item">
				<span class="stat-number">100%</span>
				<span class="stat-label">Type Safe</span>
			</div>
			<div class="stat-divider"></div>
			<div class="stat-item">
				<span class="stat-number">K8s</span>
				<span class="stat-label">Native Deploy</span>
			</div>
		</div>
	</section>

	<!-- Feature Cards -->
	<section class="features">
		{#each features as feature}
			<div class="feature-card glass-card">
				<div class="feature-icon">{feature.icon}</div>
				<h3>{feature.title}</h3>
				<p>{feature.description}</p>
			</div>
		{/each}
	</section>

	<!-- Tech Stack -->
	<section class="tech-stack">
		<p class="stack-label">Powered by</p>
		<div class="stack-logos">
			{#each techStack as tech}
				<div class="stack-item">
					<span class="stack-icon">{tech.icon}</span>
					<span>{tech.name}</span>
				</div>
			{/each}
		</div>
	</section>
</main>

<script lang="ts" module>
	const features = [
		{
			icon: '⚡',
			title: 'Blazing Fast',
			description: 'Rust-powered auth API with sub-millisecond token validation and async I/O.'
		},
		{
			icon: '🔒',
			title: 'Security First',
			description: 'Argon2 password hashing, short-lived JWTs, refresh token rotation, and rate limiting.'
		},
		{
			icon: '🌐',
			title: 'OIDC / Social Login',
			description: 'Login with Google or GitHub using the OpenID Connect protocol.'
		},
		{
			icon: '📊',
			title: 'Full Observability',
			description: 'Distributed tracing (Tempo), metrics (Prometheus/Grafana), and logs (Loki).'
		},
		{
			icon: '☸️',
			title: 'Kubernetes Native',
			description: 'Kustomize overlays, HPA, PDB, and Argo CD GitOps deployment out of the box.'
		},
		{
			icon: '🔄',
			title: 'Session Management',
			description: 'Redis-backed sessions with refresh token rotation and device tracking.'
		}
	];

	const techStack = [
		{ icon: '🦀', name: 'Rust / Axum' },
		{ icon: '🔥', name: 'SvelteKit' },
		{ icon: '🐘', name: 'PostgreSQL' },
		{ icon: '⚡', name: 'Redis' },
		{ icon: '☸️', name: 'Kubernetes' },
		{ icon: '🔭', name: 'OpenTelemetry' }
	];
</script>

<style>
	.landing {
		max-width: 1200px;
		margin: 0 auto;
		padding: 0 var(--space-6);
	}

	/* Header */
	.landing-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-6) 0;
	}

	.logo {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.logo-icon {
		font-size: 1.75rem;
	}

	.logo-text {
		font-size: 1.25rem;
		font-weight: 700;
		color: var(--color-text);
	}

	.landing-nav {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	/* Hero */
	.hero {
		text-align: center;
		padding: var(--space-16) 0 var(--space-12);
	}

	.hero-badge {
		margin-bottom: var(--space-6);
	}

	.hero-title {
		font-size: clamp(2.5rem, 6vw, 4.5rem);
		line-height: 1.1;
		margin-bottom: var(--space-6);
	}

	.gradient-text {
		background: linear-gradient(135deg, var(--color-primary-light), var(--color-accent));
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.hero-description {
		font-size: 1.125rem;
		max-width: 600px;
		margin: 0 auto var(--space-8);
		color: var(--color-text-muted);
		line-height: 1.7;
	}

	.hero-actions {
		display: flex;
		justify-content: center;
		gap: var(--space-4);
		margin-bottom: var(--space-12);
		flex-wrap: wrap;
	}

	.hero-stats {
		display: inline-flex;
		align-items: center;
		gap: var(--space-8);
		background: var(--color-bg-glass);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-full);
		padding: var(--space-3) var(--space-8);
		backdrop-filter: blur(12px);
	}

	.stat-item {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
	}

	.stat-number {
		font-size: 1.125rem;
		font-weight: 700;
		color: var(--color-primary-light);
		font-family: var(--font-mono);
	}

	.stat-label {
		font-size: 0.75rem;
		color: var(--color-text-subtle);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.stat-divider {
		width: 1px;
		height: 2rem;
		background: var(--color-border);
	}

	/* Features */
	.features {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: var(--space-6);
		margin-bottom: var(--space-16);
	}

	.feature-card {
		padding: var(--space-6);
		transition: transform var(--transition-normal), border-color var(--transition-normal);
	}

	.feature-card:hover {
		transform: translateY(-4px);
		border-color: var(--color-border-hover);
	}

	.feature-icon {
		font-size: 2rem;
		margin-bottom: var(--space-4);
	}

	.feature-card h3 {
		color: var(--color-text);
		margin-bottom: var(--space-2);
	}

	/* Tech Stack */
	.tech-stack {
		text-align: center;
		padding-bottom: var(--space-16);
	}

	.stack-label {
		font-size: 0.875rem;
		color: var(--color-text-subtle);
		text-transform: uppercase;
		letter-spacing: 0.1em;
		margin-bottom: var(--space-6);
	}

	.stack-logos {
		display: flex;
		justify-content: center;
		gap: var(--space-8);
		flex-wrap: wrap;
	}

	.stack-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--color-text-muted);
		font-size: 0.9375rem;
		font-weight: 500;
	}

	.stack-icon { font-size: 1.5rem; }
</style>
