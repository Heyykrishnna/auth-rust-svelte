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
	const slots = [0, 1, 2, 3, 4, 5];
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

<div class="relative grid w-full max-w-84 min-w-0 grid-cols-6 gap-1.5 sm:gap-2.5">
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
		class="absolute inset-0 z-10 size-full rounded-xl text-transparent caret-transparent opacity-0 disabled:cursor-not-allowed"
	/>
	{#each slots as index (index)}
		<span
			aria-hidden="true"
			class="grid h-12 min-w-0 place-items-center rounded-xl border bg-brand-surface-card font-mono text-xl font-semibold text-white transition-colors sm:h-14 {invalid ? 'border-brand-danger' : 'border-brand-border-hover'} {focused && cursor === index ? 'ring-2 ring-action-focus ring-offset-2 ring-offset-brand-surface' : ''} {disabled ? 'opacity-60' : ''}"
		>
			{#if value[index]}
				{value[index]}
			{:else if focused && cursor === index}
				<span class="h-4 w-px bg-white"></span>
			{/if}
		</span>
	{/each}
</div>
