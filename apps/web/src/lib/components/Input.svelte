<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	interface Props {
		id: string;
		label?: string;
		type?: 'text' | 'email' | 'password' | 'number';
		placeholder?: string;
		value?: string;
		error?: string;
		hint?: string;
		disabled?: boolean;
		required?: boolean;
		autocomplete?: HTMLInputAttributes['autocomplete'];
		onchange?: (value: string) => void;
	}

	let {
		id,
		label,
		type = 'text',
		placeholder,
		value = $bindable(''),
		error,
		hint,
		disabled = false,
		required = false,
		autocomplete,
		onchange
	}: Props = $props();

	let showPassword = $state(false);
	let actualType = $derived(type === 'password' ? (showPassword ? 'text' : 'password') : type);
</script>

<div class="flex flex-col gap-0">
	{#if label}
		<label class="block text-xs font-medium text-brand-text-muted mb-1.5" for={id}>
			{label}
			{#if required}<span class="text-rose-500 ml-1" aria-hidden="true">*</span>{/if}
		</label>
	{/if}

	<div class="relative">
		<input
			{id}
			type={actualType}
			{placeholder}
			{disabled}
			{required}
			{autocomplete}
			bind:value
			oninput={() => onchange?.(value)}
			class="w-full px-3.5 py-2.5 bg-brand-surface border rounded-md text-brand-text-primary font-sans text-sm outline-none transition-colors placeholder:text-brand-text-subtle disabled:opacity-50 disabled:cursor-not-allowed focus:border-brand-lime {type === 'password' ? 'pr-10' : ''} {error ? 'border-rose-500 focus:border-rose-500' : 'border-brand-border'}"
			aria-invalid={!!error}
			aria-describedby={error ? `${id}-error` : hint ? `${id}-hint` : undefined}
		/>

		{#if type === 'password'}
			<button
				type="button"
				class="absolute right-2.5 top-1/2 -translate-y-1/2 bg-transparent border-none text-brand-text-muted hover:text-brand-text-primary cursor-pointer inline-flex items-center justify-center p-1 transition-colors"
				onclick={() => (showPassword = !showPassword)}
				aria-label={showPassword ? 'Hide password' : 'Show password'}
			>
				{#if showPassword}
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true">
						<path d="M17.94 17.94A10.94 10.94 0 0 1 12 20C7 20 2.73 16.11 1 12c.74-1.76 1.94-3.36 3.47-4.66" />
						<path d="M9.9 4.24A10.94 10.94 0 0 1 12 4c5 0 9.27 3.89 11 8a11.05 11.05 0 0 1-2.16 3.19" />
						<line x1="1" y1="1" x2="23" y2="23" />
					</svg>
				{:else}
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true">
						<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
						<circle cx="12" cy="12" r="3" />
					</svg>
				{/if}
			</button>
		{/if}
	</div>

	{#if error}
		<p id="{id}-error" class="text-xs text-rose-400 mt-1" role="alert">{error}</p>
	{:else if hint}
		<p id="{id}-hint" class="text-xs text-brand-text-subtle mt-1">{hint}</p>
	{/if}
</div>
