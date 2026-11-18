<script lang="ts">
	let { label, value = $bindable() }: { label: string; value: string } =
		$props();
</script>

<label
	class="flex min-h-10 min-w-0 items-center gap-2.5 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 text-brand-text-secondary focus-within:border-brand-lime"
>
	<svg
		class="size-4 shrink-0"
		viewBox="0 0 24 24"
		fill="none"
		stroke="currentColor"
		stroke-width="1.6"
		aria-hidden="true"
	>
		<circle cx="11" cy="11" r="6.5" />
		<path d="m16 16 4 4" stroke-linecap="round" />
	</svg>
	<input
		type="search"
		class="w-full min-w-0 py-2 text-sm text-brand-text-primary outline-none placeholder:text-brand-text-muted"
		aria-label={label}
		placeholder={label}
		bind:value
	/>
</label>
