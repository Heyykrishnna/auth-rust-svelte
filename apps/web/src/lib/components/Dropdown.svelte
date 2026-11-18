<script lang="ts" generics="Value extends string">
	import { tick } from 'svelte';

	interface Props {
		label: string;
		options: readonly { value: Value; label: string }[];
		value: Value;
		placeholder?: string;
		disabled?: boolean;
		class?: string;
		onchange?: (value: Value) => void;
	}

	let {
		label,
		options,
		value = $bindable(),
		placeholder = 'Select an option',
		disabled = false,
		class: className = '',
		onchange
	}: Props = $props();

	const id = $props.id();
	let trigger: HTMLButtonElement;
	let list: HTMLDivElement;
	let open = $state(false);
	let active = $state(-1);
	let search = '';
	let lastKeyAt = 0;
	const selection = $derived(
		options.findIndex((option) => option.value === value)
	);
	const unavailable = $derived(disabled || options.length === 0);

	$effect(() => {
		if (unavailable && open) list.hidePopover();
	});

	function activate(index: number) {
		active = index;
		void tick().then(() => {
			list
				?.querySelectorAll('[role="option"]')
				.item(active)
				?.scrollIntoView({ block: 'nearest' });
		});
	}

	function onToggle(event: ToggleEvent) {
		open = event.newState === 'open';
		if (open && active === -1) activate(Math.max(selection, 0));
		if (!open) {
			active = -1;
			search = '';
		}
	}

	function choose(index: number) {
		const option = options[index];
		if (!option || unavailable) return;
		list.hidePopover();
		trigger.focus({ preventScroll: true });
		if (option.value === value) return;
		if (onchange) onchange(option.value);
		else value = option.value;
	}

	function onKeydown(event: KeyboardEvent) {
		if (unavailable) return;
		const key = event.key;
		const expanded = list.matches(':popover-open');
		if (key === 'Escape' && expanded) {
			event.preventDefault();
			event.stopPropagation();
			list.hidePopover();
			return;
		}
		if (key === 'Tab') {
			if (expanded) list.hidePopover();
			return;
		}
		if (expanded && (key === 'Enter' || key === ' ')) {
			event.preventDefault();
			choose(active);
			return;
		}

		const printable =
			key.length === 1 &&
			key !== ' ' &&
			!event.altKey &&
			!event.ctrlKey &&
			!event.metaKey;
		if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(key) && !printable)
			return;
		event.preventDefault();
		if (!expanded) list.showPopover({ source: trigger });
		const current = expanded ? active : selection;
		const last = options.length - 1;

		if (key === 'Home') activate(0);
		else if (key === 'End') activate(last);
		else if (key === 'ArrowDown')
			activate(expanded ? Math.min(current + 1, last) : Math.max(current, 0));
		else if (key === 'ArrowUp')
			activate(
				expanded ? Math.max(current - 1, 0) : current < 0 ? last : current
			);
		else {
			const now = performance.now();
			const letter = key.toLocaleLowerCase();
			const repeated = now - lastKeyAt < 600 && search === letter;
			search =
				now - lastKeyAt < 600 && !repeated ? `${search}${letter}` : letter;
			lastKeyAt = now;
			const start = repeated ? current + 1 : 0;
			for (let offset = 0; offset < options.length; offset++) {
				const index = (start + offset) % options.length;
				if (options[index].label.toLocaleLowerCase().startsWith(search)) {
					activate(index);
					break;
				}
			}
		}
	}
</script>

<div class="inline-grid min-w-0 [anchor-scope:--dropdown] {className}">
	<button
		bind:this={trigger}
		type="button"
		role="combobox"
		aria-label={label}
		aria-controls={id}
		aria-expanded={open}
		aria-activedescendant={open && active >= 0 ? `${id}-${active}` : undefined}
		disabled={unavailable}
		popovertarget={id}
		onkeydown={onKeydown}
		class="inline-flex h-9 w-full min-w-0 items-center justify-between gap-3 rounded-lg border border-brand-border-hover bg-brand-surface-input px-3 text-left text-sm text-brand-text-primary transition-colors [anchor-name:--dropdown] hover:border-brand-border-strong hover:bg-brand-surface-raised focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-lime disabled:cursor-not-allowed disabled:opacity-50"
	>
		<span class="min-w-0 truncate">
			{options[selection]?.label ?? placeholder}
		</span>
		<svg
			class="size-4 shrink-0 text-brand-text-secondary"
			viewBox="0 0 16 16"
			fill="none"
			aria-hidden="true"
		>
			<path
				d="m4 6 4 4 4-4"
				stroke="currentColor"
				stroke-width="1.4"
				stroke-linecap="round"
				stroke-linejoin="round"
			/>
		</svg>
	</button>

	<div
		bind:this={list}
		{id}
		popover="auto"
		role="listbox"
		aria-label={label}
		ontoggle={onToggle}
		class="fixed inset-auto [top:anchor(bottom)] [left:anchor(left)] m-0 mt-1 max-h-60 [width:anchor-size(width)] max-w-[calc(100vw-2rem)] min-w-40 overflow-y-auto overscroll-contain rounded-lg border border-brand-border-hover bg-brand-surface-raised p-1 text-brand-text-primary shadow-xl [position-anchor:--dropdown] [position-try-fallbacks:flip-block,flip-inline]"
	>
		{#each options as option, index (option.value)}
			<button
				type="button"
				role="option"
				id={`${id}-${index}`}
				tabindex="-1"
				aria-selected={option.value === value}
				onclick={() => choose(index)}
				class="flex min-h-9 w-full items-center justify-between gap-3 rounded-md px-2.5 py-1.5 text-left text-sm wrap-anywhere hover:bg-brand-surface-alt {active ===
				index
					? 'bg-brand-surface-alt'
					: ''}"
			>
				<span>{option.label}</span>
				{#if option.value === value}
					<svg
						class="size-4 shrink-0 text-brand-lime"
						viewBox="0 0 16 16"
						fill="none"
						aria-hidden="true"
					>
						<path
							d="m3 8 3 3 7-7"
							stroke="currentColor"
							stroke-width="1.5"
							stroke-linecap="round"
							stroke-linejoin="round"
						/>
					</svg>
				{/if}
			</button>
		{/each}
	</div>
</div>
