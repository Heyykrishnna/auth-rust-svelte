<script lang="ts">
	import '../app.css';
	import Toast from '$lib/components/Toast.svelte';
	import { initAuth } from '$lib/stores/auth.svelte';
	import { onMount } from 'svelte';

	let { children } = $props();

	onMount(() => {
		initAuth();
	});
</script>

<div class="app-bg" aria-hidden="true"></div>

{@render children()}

<Toast />
