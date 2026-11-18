<script lang="ts">
	let {
		value = $bindable(''),
		disabled = false,
		invalid = false,
		oncomplete
	}: {
		value?: string;
		disabled?: boolean;
		invalid?: boolean;
		oncomplete?: (code: string) => void;
	} = $props();

	const count = 6;
	let input: HTMLInputElement;
	let focused = $state(false);
	let cursor = $state(0);

	export function focus() {
		input.focus();
	}

	function syncSelection() {
		cursor = Math.min(input.selectionStart ?? value.length, count - 1);
	}

	function update(raw: string, position: number) {
		const next = raw.replace(/\D/g, '').slice(0, count);
		const nextPosition = Math.min(raw.slice(0, position).replace(/\D/g, '').length, next.length);
		if (input.value !== next) input.value = next;
		input.setSelectionRange(nextPosition, nextPosition);
		const changed = next !== value;
		value = next;
		syncSelection();
		if (changed && next.length === count) oncomplete?.(next);
	}

	function paste(event: ClipboardEvent) {
		const digits = event.clipboardData?.getData('text').replace(/\D/g, '').slice(0, count);
		if (!digits) return;
		event.preventDefault();
		if (digits.length === count) {
			update(digits, count);
			return;
		}
		const start = input.selectionStart ?? value.length;
		const end = input.selectionEnd ?? start;
		update(value.slice(0, start) + digits + value.slice(end), start + digits.length);
	}
</script>

<div class="relative flex w-full max-w-sm items-center justify-center gap-2 sm:gap-3">
	<input
		bind:this={input}
		type="text"
		inputmode="numeric"
		autocomplete="one-time-code"
		aria-label="Six-digit verification code"
		aria-invalid={invalid}
		{disabled}
		{value}
		pattern="[0-9]{6}"
		required
		oninput={() => update(input.value, input.selectionStart ?? input.value.length)}
		onpaste={paste}
		onselect={syncSelection}
		onkeyup={syncSelection}
		onclick={syncSelection}
		onfocus={() => { focused = true; syncSelection(); }}
		onblur={() => (focused = false)}
		class="absolute inset-0 z-10 size-full cursor-pointer rounded-xl text-transparent caret-transparent opacity-0 disabled:cursor-not-allowed"
	/>
	<div class="flex items-center gap-1.5 sm:gap-2">
		{#each [0, 1, 2] as index (index)}
			{@const isCurrent = focused && cursor === index}
			<span
				aria-hidden="true"
				class="grid h-13 w-11 place-items-center rounded-xl border bg-brand-surface-card font-mono text-xl font-bold text-white transition-all sm:h-15 sm:w-13 sm:text-2xl {invalid ? 'border-brand-danger ring-2 ring-brand-danger/30' : isCurrent ? 'border-[#296FF0] ring-2 ring-[#296FF0]/40' : 'border-brand-border-hover'} {disabled ? 'opacity-50' : ''}"
			>
				{#if value[index]}
					{value[index]}
				{:else if isCurrent}
					<span class="h-5 w-0.5 rounded-full bg-[#296FF0] animate-pulse"></span>
				{/if}
			</span>
		{/each}
	</div>

	<span class="h-0.5 w-2 rounded-full bg-zinc-600" aria-hidden="true"></span>

	<div class="flex items-center gap-1.5 sm:gap-2">
		{#each [3, 4, 5] as index (index)}
			{@const isCurrent = focused && cursor === index}
			<span
				aria-hidden="true"
				class="grid h-13 w-11 place-items-center rounded-xl border bg-brand-surface-card font-mono text-xl font-bold text-white transition-all sm:h-15 sm:w-13 sm:text-2xl {invalid ? 'border-brand-danger ring-2 ring-brand-danger/30' : isCurrent ? 'border-[#296FF0] ring-2 ring-[#296FF0]/40' : 'border-brand-border-hover'} {disabled ? 'opacity-50' : ''}"
			>
				{#if value[index]}
					{value[index]}
				{:else if isCurrent}
					<span class="h-5 w-0.5 rounded-full bg-[#296FF0] animate-pulse"></span>
				{/if}
			</span>
		{/each}
	</div>
</div>
