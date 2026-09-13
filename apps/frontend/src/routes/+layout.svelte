<script lang="ts">
	import '../app.css';
	import Navbar from '$lib/components/Navbar.svelte';
	import Toast from '$lib/components/Toast.svelte';
	import { initAuth, syncServerUser } from '$lib/stores/auth.svelte';
	import { onMount } from 'svelte';
	import type { PageData } from './$types';

	let { data, children }: { data: PageData; children: import('svelte').Snippet } = $props();

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

<div class="app-shell">
	<Navbar />

	<main class="page-container">
		{@render children()}
	</main>

	<footer class="app-footer">
		<div class="footer-content">
			<p>
				Production Authentication Platform — Rust (Axum) + SvelteKit. Clean decoupled architecture.
			</p>
		</div>
	</footer>
</div>

<Toast />

<style>
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
		border-top: 1px solid rgba(255, 255, 255, 0.05);
		padding: 1.5rem;
		text-align: center;
		color: #64748b;
		font-size: 0.825rem;
		background: rgba(10, 14, 23, 0.5);
	}

	.footer-content {
		max-width: 1200px;
		margin: 0 auto;
	}

	.footer-content p {
		margin: 0;
	}
</style>
