<script lang="ts">
	let { children }: { children: import('svelte').Snippet } = $props();
</script>

<div class="min-h-screen w-full bg-black text-[#f4f4f5]">
	{@render children()}
</div>
