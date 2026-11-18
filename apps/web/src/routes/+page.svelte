<script lang="ts" module>
  const features = [
    {
      icon: "01",
      title: "Low-Latency Core",
      description:
        "Rust-powered auth API with sub-millisecond token validation and async I/O.",
    },
    {
      icon: "02",
      title: "Security First",
      description:
        "Argon2id password hashing, ES256 JWKS signing, refresh rotation, and rate limiting.",
    },
    {
      icon: "03",
      title: "OIDC & Social SSO",
      description:
        "Centralized OIDC provider with PKCE S256 and upstream Google and GitHub login.",
    },
    {
      icon: "04",
      title: "Service Sync",
      description:
        "Transactional outbox, HMAC-signed webhooks, JIT claims, and cursor reconciliation.",
    },
    {
      icon: "05",
      title: "Kubernetes Native",
      description:
        "Kustomize overlays, HPA, PDB, and Argo CD GitOps deployment out of the box.",
    },
    {
      icon: "06",
      title: "Session Management",
      description:
        "Environment-isolated sessions with refresh token rotation and instant revocation.",
    },
  ];

  const techStack = [
    { icon: "RS", name: "Rust / Axum" },
    { icon: "SK", name: "SvelteKit 5" },
    { icon: "PG", name: "PostgreSQL" },
    { icon: "RD", name: "Redis" },
    { icon: "K8", name: "Kubernetes" },
    { icon: "OT", name: "OpenTelemetry" },
  ];
</script>

<script lang="ts">
  import Badge from "$lib/components/Badge.svelte";
  import Button from "$lib/components/Button.svelte";
  import { authStore } from "$lib/stores/auth.svelte";
  import { goto } from "$app/navigation";

  $effect(() => {
    if (authStore.initialized && authStore.isAuthenticated) {
      goto("/dashboard");
    }
  });
</script>

<svelte:head>
  <title>Signet — Authentication Infrastructure for Every Application</title>
  <meta
    name="description"
    content="Signet is high-performance authentication infrastructure built with Rust and SvelteKit. Cryptographic ES256 tokens, OIDC social SSO, and multi-tenant developer control plane."
  />
</svelte:head>

<div class="max-w-7xl mx-auto px-6">
  <section class="text-center pt-16 pb-12">
    <div class="mb-6 flex justify-center">
      <Badge variant="purple" dot>Signet Identity Platform</Badge>
    </div>

    <h1 class="text-4xl sm:text-6xl lg:text-7xl font-bold font-heading tracking-tight leading-[1.1] mb-6 text-brand-text-primary">
      Authentication<br />
      <span class="text-brand-lime">infrastructure for every application.</span>
    </h1>

    <p class="text-lg text-brand-text-secondary max-w-2xl mx-auto mb-8 leading-relaxed font-sans">
      Signet delivers high-performance, developer-first authentication
      infrastructure built on Rust (Axum), SvelteKit, PostgreSQL, and Redis.
      Centralized OIDC, cryptographic ES256 JWKS signing, transactional outbox
      webhooks, and developer control plane.
    </p>

    <div class="flex flex-wrap justify-center gap-4 mb-12">
      <Button href="/register" variant="primary" size="lg">
        Get Started with Signet →
      </Button>
      <Button href="/login" variant="outline" size="lg">Sign In</Button>
    </div>

    <div class="inline-flex items-center gap-6 sm:gap-8 bg-brand-surface-card/80 border border-brand-border rounded-full px-6 sm:px-8 py-3 backdrop-blur-md">
      <div class="flex flex-col items-center gap-0.5">
        <span class="text-lg font-bold font-mono text-brand-lime-soft">~2ms</span>
        <span class="text-[11px] uppercase tracking-wider text-brand-text-subtle font-medium">Token Verification</span>
      </div>
      <div class="w-px h-8 bg-brand-border"></div>
      <div class="flex flex-col items-center gap-0.5">
        <span class="text-lg font-bold font-mono text-brand-lime-soft">ES256</span>
        <span class="text-[11px] uppercase tracking-wider text-brand-text-subtle font-medium">JWKS Cryptography</span>
      </div>
      <div class="w-px h-8 bg-brand-border"></div>
      <div class="flex flex-col items-center gap-0.5">
        <span class="text-lg font-bold font-mono text-brand-lime-soft">100%</span>
        <span class="text-[11px] uppercase tracking-wider text-brand-text-subtle font-medium">Isolated Tenancy</span>
      </div>
    </div>

    <div class="max-w-5xl mx-auto mt-12 p-2 rounded-xl border border-brand-border bg-brand-surface-card shadow-2xl overflow-hidden">
      <img
        src="/brand/signet-platform.webp"
        alt="Signet authentication infrastructure platform overview"
        class="w-full h-auto block rounded-lg select-none"
        loading="eager"
      />
    </div>
  </section>

  <section class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-16">
    {#each features as feature}
      <div class="p-6 rounded-xl border border-brand-border bg-brand-surface-card hover:border-brand-border-hover hover:-translate-y-1 transition-all duration-200">
        <div class="text-2xl font-mono font-bold text-brand-lime mb-4">{feature.icon}</div>
        <h3 class="text-lg font-semibold font-heading text-brand-text-primary mb-2">{feature.title}</h3>
        <p class="text-sm text-brand-text-secondary leading-relaxed m-0">{feature.description}</p>
      </div>
    {/each}
  </section>

  <section class="text-center pb-16">
    <p class="text-xs uppercase tracking-widest text-brand-text-subtle mb-6 font-medium">Built with precision for production</p>
    <div class="flex justify-center flex-wrap gap-8">
      {#each techStack as tech}
        <div class="flex items-center gap-2 text-brand-text-muted text-sm font-medium">
          <span class="font-mono font-bold text-brand-lime text-base">{tech.icon}</span>
          <span>{tech.name}</span>
        </div>
      {/each}
    </div>
  </section>
</div>
