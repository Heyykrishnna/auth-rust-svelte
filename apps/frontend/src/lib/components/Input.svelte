<script lang="ts">
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
		autocomplete?: string;
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
				{showPassword ? '🙈' : '👁️'}
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
		margin-left: 0.25rem;
	}

	.input-wrapper {
		position: relative;
	}

	.password-toggle {
		position: absolute;
		right: 0.75rem;
		top: 50%;
		transform: translateY(-50%);
		background: none;
		border: none;
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
		opacity: 0.6;
		transition: opacity 150ms ease;
		padding: 0.25rem;
	}

	.password-toggle:hover { opacity: 1; }

	.input-wrapper .input {
		width: 100%;
	}

	.input-wrapper:has(.password-toggle) .input {
		padding-right: 2.75rem;
	}
</style>
