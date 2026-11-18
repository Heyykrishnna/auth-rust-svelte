<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import Navbar from '$lib/components/Navbar.svelte';
	import Toast from '$lib/components/Toast.svelte';
	import { initAuth, syncServerUser } from '$lib/stores/auth.svelte';
	import { onMount } from 'svelte';
	import type { PageData } from './$types';

	let { data, children }: { data: PageData; children: import('svelte').Snippet } = $props();

	const isFullBleedRoute = $derived(
		page.url.pathname.startsWith('/control') ||
			page.url.pathname.startsWith('/portal') ||
			page.url.pathname === '/login' ||
			page.url.pathname === '/register'
	);

	$effect(() => {
		if (data?.user !== undefined) {
			syncServerUser(data.user);
		}
	});

	onMount(() => {
		initAuth();
	});
</script>

<div class="app-bg" aria-hidden="true"></div>

{#if isFullBleedRoute}
	<div class="full-bleed-shell">
		{@render children()}
	</div>
{:else}
	<div class="app-shell">
		<Navbar />

		<main class="page-container">
			{@render children()}
		</main>

		<footer class="app-footer">
			<div class="footer-content">
				<p>Secure Auth Platform — Multi-Tenant Identity & Developer Control Plane</p>
			</div>
		</footer>
	</div>
{/if}

<Toast />

<style>
	.full-bleed-shell {
		min-height: 100vh;
		width: 100%;
	}

	.app-shell {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
		position: relative;
		z-index: 1;
	}

	.page-container {
		flex: 1;
		display: flex;
		flex-direction: column;
	}

	.app-footer {
		border-top: 1px solid var(--color-border);
		padding: 1.25rem 1.5rem;
		text-align: center;
		color: var(--color-text-subtle);
		font-size: 0.8rem;
		background-color: var(--color-bg-sidebar);
	}

	.footer-content {
		max-width: 1240px;
		margin: 0 auto;
	}

	.footer-content p {
		margin: 0;
		color: var(--color-text-subtle);
	}
</style>
