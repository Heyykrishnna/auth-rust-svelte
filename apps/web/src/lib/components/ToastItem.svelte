<script lang="ts">
	import { fly } from 'svelte/transition';
	import { onMount } from 'svelte';

	let { message, type, duration, onDismiss }: {
		message: string;
		type: 'success' | 'error' | 'warning' | 'info';
		duration: number;
		onDismiss: () => void;
	} = $props();

	const styles = {
		success: { label: 'Success', icon: 'm5 12 4 4L19 6', color: 'text-brand-success-text', fill: 'bg-brand-success-soft', track: 'text-brand-success' },
		error: { label: 'Error', icon: 'm7 7 10 10M17 7 7 17', color: 'text-brand-danger-text', fill: 'bg-brand-danger-soft', track: 'text-brand-danger' },
		warning: { label: 'Warning', icon: 'M12 7v6m0 4h.01', color: 'text-brand-warning-text', fill: 'bg-brand-warning-soft', track: 'text-brand-warning' },
		info: { label: 'Notice', icon: 'M12 11v6m0-10h.01', color: 'text-action-focus', fill: 'bg-action/15', track: 'text-action-focus' }
	};
	const style = $derived(styles[type]);
	let remaining = $state(0);
	let hovered = $state(false);
	let focused = $state(false);
	let motion = $state(false);

	onMount(() => {
		motion = !matchMedia('(prefers-reduced-motion: reduce)').matches;
		remaining = duration;
		if (duration <= 0) return;

		let previous = performance.now();
		const timer = setInterval(() => {
			const now = performance.now();
			if (!hovered && !focused) {
				remaining = Math.max(0, remaining - (now - previous));
				if (remaining === 0) {
					clearInterval(timer);
					onDismiss();
				}
			}
			previous = now;
		}, 80);
		return () => clearInterval(timer);
	});
</script>

<li
	class="task-surface pointer-events-auto relative isolate min-w-0 overflow-hidden text-brand-text-primary"
	onpointerenter={() => (hovered = true)}
	onpointerleave={() => (hovered = false)}
	onfocusin={() => (focused = true)}
	onfocusout={(event) => (focused = event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget))}
	transition:fly={{ y: motion ? 8 : 0, duration: motion ? 150 : 0 }}
>
	<div class="flex min-w-0 items-start gap-3 p-3.5">
		<span class="grid size-8 shrink-0 place-items-center rounded-lg {style.color} {style.fill}" aria-hidden="true">
			<svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d={style.icon} /></svg>
		</span>
		<div class="min-w-0 flex-1" role={type === 'error' ? 'alert' : 'status'}>
			<p class="text-xs font-semibold {style.color}">{style.label}</p>
			<p class="mt-0.5 text-sm leading-snug break-words">{message}</p>
		</div>
		<button type="button" class="action-button grid size-7 shrink-0 place-items-center rounded-md" onclick={onDismiss} aria-label="Dismiss notification">
			<svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
		</button>
	</div>
	{#if duration > 0}
		<progress class="absolute inset-x-0 bottom-0 h-0.5 w-full appearance-none {style.track} [&::-moz-progress-bar]:bg-current [&::-webkit-progress-bar]:bg-transparent [&::-webkit-progress-value]:bg-current" value={remaining} max={duration} aria-hidden="true"></progress>
	{/if}
</li>
