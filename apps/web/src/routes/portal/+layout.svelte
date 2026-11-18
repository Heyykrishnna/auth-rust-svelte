<script lang="ts">
	let { children }: { children: import('svelte').Snippet } = $props();
</script>

<div class="portal-root">
	{@render children()}
</div>

<style>
	.portal-root {
		min-height: 100vh;
		width: 100%;
		background-color: #000000;
		color: #f4f4f5;
	}
</style>
