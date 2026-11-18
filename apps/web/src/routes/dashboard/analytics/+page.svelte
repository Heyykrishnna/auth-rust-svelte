<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(() => {
		goto('/control?section=analytics', { replaceState: true });
	});
</script>

<svelte:head>
	<title>Analytics — Secure Auth Control Plane</title>
</svelte:head>

<div class="redirect-shell">
	<p>Opening environment analytics...</p>
</div>

<style>
	.redirect-shell {
		min-height: 60vh;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-secondary);
		font-size: 0.9rem;
	}
</style>

