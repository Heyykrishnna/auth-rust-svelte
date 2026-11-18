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

<div class="field">
	{#if label}
		<label class="label" for={id}>
			{label}
			{#if required}<span class="required" aria-hidden="true">*</span>{/if}
		</label>
	{/if}

	<div class="input-wrapper">
		<input
			{id}
			type={actualType}
			{placeholder}
			{disabled}
			{required}
			{autocomplete}
			bind:value
			oninput={() => onchange?.(value)}
			class="input"
			class:error={!!error}
			aria-invalid={!!error}
			aria-describedby={error ? `${id}-error` : hint ? `${id}-hint` : undefined}
		/>

		{#if type === 'password'}
			<button
				type="button"
				class="password-toggle"
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
		<p id="{id}-error" class="error-text" role="alert">{error}</p>
	{:else if hint}
		<p id="{id}-hint" class="helper-text">{hint}</p>
	{/if}
</div>

<style>
	.field { display: flex; flex-direction: column; gap: 0; }

	.required {
		color: var(--color-error);
		margin-left: 0.2rem;
	}

	.input-wrapper {
		position: relative;
	}

	.password-toggle {
		position: absolute;
		right: 0.7rem;
		top: 50%;
		transform: translateY(-50%);
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0.25rem;
		transition: color 150ms ease;
	}

	.password-toggle:hover { color: var(--color-text); }

	.input-wrapper .input {
		width: 100%;
	}

	.input-wrapper:has(.password-toggle) .input {
		padding-right: 2.5rem;
	}
</style>
