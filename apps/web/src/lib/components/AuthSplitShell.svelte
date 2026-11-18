<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		brandName?: string;
		logoUrl?: string | null;
		footerText?: string;
		topActionLabel?: string;
		topActionHref?: string;
		onTopAction?: () => void;
		compact?: boolean;
		children: Snippet;
	}

	let {
		brandName = 'Secure Auth',
		logoUrl = null,
		footerText = '© 2026 Secure Auth Platform. All rights reserved.',
		topActionLabel,
		topActionHref,
		onTopAction,
		compact = false,
		children
	}: Props = $props();
</script>

<div class="auth-split" class:auth-split-compact={compact}>
	<aside class="brand-pane" aria-label="Brand showcase">
		<div class="pane-top">
			{#if logoUrl}
				<img src={logoUrl} alt={brandName} class="custom-logo" />
			{:else}
				<span class="brand-wordmark">{brandName}</span>
			{/if}
		</div>

		<div class="emblem-stage" aria-hidden="true">
			<div class="grid-axis grid-axis-v"></div>
			<div class="grid-axis grid-axis-h"></div>
			<div class="grid-axis grid-axis-d1"></div>
			<div class="grid-axis grid-axis-d2"></div>
			<svg class="swiss-star" viewBox="0 0 160 160" fill="none">
				<line x1="80" y1="10" x2="80" y2="66" stroke="#ffffff" stroke-width="5" stroke-linecap="square" />
				<line x1="80" y1="94" x2="80" y2="150" stroke="#ffffff" stroke-width="5" stroke-linecap="square" />
				<line x1="10" y1="80" x2="66" y2="80" stroke="#ffffff" stroke-width="5" stroke-linecap="square" />
				<line x1="94" y1="80" x2="150" y2="80" stroke="#ffffff" stroke-width="5" stroke-linecap="square" />
				<line x1="31" y1="31" x2="69" y2="69" stroke="#ffffff" stroke-width="4.5" stroke-linecap="square" />
				<line x1="91" y1="91" x2="129" y2="129" stroke="#ffffff" stroke-width="4.5" stroke-linecap="square" />
				<line x1="129" y1="31" x2="91" y2="69" stroke="#ffffff" stroke-width="4.5" stroke-linecap="square" />
				<line x1="69" y1="91" x2="31" y2="129" stroke="#ffffff" stroke-width="4.5" stroke-linecap="square" />
			</svg>
		</div>

		<div class="pane-footer">
			<span>{footerText}</span>
		</div>
	</aside>

	<section class="form-pane">
		<div class="form-topbar">
			{#if topActionLabel && topActionHref}
				<a href={topActionHref} class="top-action-link">{topActionLabel}</a>
			{:else if topActionLabel && onTopAction}
				<button type="button" class="top-action-link" onclick={onTopAction}>
					{topActionLabel}
				</button>
			{/if}
		</div>

		<div class="form-content">
			{@render children()}
		</div>
	</section>
</div>

<style>
	.auth-split {
		display: grid;
		grid-template-columns: 1fr 1fr;
		min-height: 100vh;
		width: 100%;
		background-color: #000000;
		color: #f4f4f5;
	}

	.auth-split-compact {
		min-height: 460px;
		border: 1px solid #23262e;
		border-radius: 10px;
		overflow: hidden;
	}

	.brand-pane {
		position: relative;
		background-color: #000000;
		border-right: 1px solid #1b1b1d;
		padding: 2.25rem 2.5rem;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		overflow: hidden;
	}

	.auth-split-compact .brand-pane {
		padding: 1.5rem;
	}

	.pane-top {
		position: relative;
		z-index: 2;
	}

	.brand-wordmark {
		font-size: 1.05rem;
		font-weight: 500;
		letter-spacing: -0.02em;
		color: #ffffff;
	}

	.custom-logo {
		max-height: 30px;
		width: auto;
		display: block;
	}

	.emblem-stage {
		position: relative;
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 220px;
	}

	.grid-axis {
		position: absolute;
		background-color: #141416;
		pointer-events: none;
	}

	.grid-axis-v {
		width: 1px;
		height: 200%;
		left: 50%;
		top: -50%;
	}

	.grid-axis-h {
		height: 1px;
		width: 200%;
		top: 50%;
		left: -50%;
	}

	.grid-axis-d1 {
		width: 1px;
		height: 220%;
		left: 50%;
		top: -60%;
		transform: rotate(45deg);
	}

	.grid-axis-d2 {
		width: 1px;
		height: 220%;
		left: 50%;
		top: -60%;
		transform: rotate(-45deg);
	}

	.swiss-star {
		position: relative;
		z-index: 2;
		width: 132px;
		height: 132px;
	}

	.auth-split-compact .swiss-star {
		width: 84px;
		height: 84px;
	}

	.pane-footer {
		position: relative;
		z-index: 2;
		font-size: 0.72rem;
		color: #52525b;
		letter-spacing: 0.01em;
	}

	.form-pane {
		background-color: #111113;
		padding: 2.25rem 3.5rem 2.75rem;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		position: relative;
	}

	.auth-split-compact .form-pane {
		padding: 1.5rem 2rem;
	}

	.form-topbar {
		display: flex;
		justify-content: flex-end;
		min-height: 1.5rem;
	}

	.top-action-link {
		background: transparent;
		border: none;
		color: #a1a1aa;
		font-size: 0.8rem;
		font-weight: 400;
		cursor: pointer;
		text-decoration: none;
		padding: 0;
		transition: color 150ms ease;
	}

	.top-action-link:hover {
		color: #ffffff;
	}

	.form-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		justify-content: center;
		max-width: 520px;
		width: 100%;
		margin: 0 auto;
	}

	:global(.editorial-title) {
		font-size: 2.25rem;
		font-weight: 400;
		letter-spacing: -0.03em;
		color: #ffffff;
		margin: 0 0 0.5rem;
	}

	:global(.editorial-subtitle) {
		font-size: 0.875rem;
		color: #71717a;
		margin: 0 0 2.25rem;
	}

	:global(.editorial-grid-2) {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.75rem;
	}

	:global(.editorial-stack) {
		display: flex;
		flex-direction: column;
		gap: 1.65rem;
	}

	:global(.editorial-field) {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
		position: relative;
	}

	:global(.editorial-label) {
		font-size: 0.72rem;
		font-weight: 500;
		color: #71717a;
		letter-spacing: 0.02em;
	}

	:global(.editorial-input-wrap) {
		position: relative;
		display: flex;
		align-items: center;
	}

	:global(.editorial-input) {
		width: 100%;
		background: transparent;
		border: none;
		border-bottom: 1px solid #2e2e34;
		padding: 0.55rem 0;
		color: #f4f4f5;
		font-family: var(--font-sans);
		font-size: 0.95rem;
		outline: none;
		transition: border-color 150ms ease;
	}

	:global(.editorial-input::placeholder) {
		color: #52525b;
	}

	:global(.editorial-input:focus) {
		border-bottom-color: #ffffff;
	}

	:global(.editorial-input.has-error) {
		border-bottom-color: #ef4444;
	}

	:global(.editorial-eye) {
		position: absolute;
		right: 0;
		background: transparent;
		border: none;
		color: #71717a;
		cursor: pointer;
		padding: 0.2rem;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}

	:global(.editorial-eye:hover) {
		color: #ffffff;
	}

	:global(.editorial-field-error) {
		font-size: 0.74rem;
		color: #f87171;
	}

	:global(.editorial-meta-row) {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: 0.85rem;
		font-size: 0.78rem;
		color: #71717a;
	}

	:global(.editorial-check) {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		color: #a1a1aa;
		user-select: none;
	}

	:global(.editorial-check input) {
		accent-color: #ffffff;
		width: 14px;
		height: 14px;
		cursor: pointer;
	}

	:global(.editorial-quiet-link) {
		color: #a1a1aa;
		font-size: 0.78rem;
		text-decoration: none;
		background: transparent;
		border: none;
		cursor: pointer;
		padding: 0;
	}

	:global(.editorial-quiet-link:hover) {
		color: #ffffff;
	}

	:global(.editorial-alert) {
		padding: 0.75rem 0.9rem;
		border: 1px solid #572327;
		background-color: #1f1112;
		color: #fca5a5;
		border-radius: 6px;
		font-size: 0.82rem;
		margin-bottom: 1.5rem;
	}

	:global(.editorial-actions-footer) {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 1.5rem;
		margin-top: 2.75rem;
	}

	:global(.editorial-oauth-group) {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		flex-wrap: wrap;
	}

	:global(.editorial-oauth-btn) {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		background-color: transparent;
		border: 1px solid #2e2e34;
		border-radius: 6px;
		padding: 0.5rem 0.85rem;
		color: #d4d4d8;
		font-size: 0.78rem;
		font-weight: 500;
		cursor: pointer;
		transition: border-color 150ms ease, color 150ms ease, background-color 150ms ease;
	}

	:global(.editorial-oauth-btn:hover) {
		border-color: #ffffff;
		color: #ffffff;
		background-color: #18181b;
	}

	:global(.editorial-circle-cta) {
		width: 96px;
		height: 96px;
		border-radius: 9999px;
		border: none;
		background-color: #ffffff;
		color: #000000;
		font-size: 0.72rem;
		font-weight: 600;
		letter-spacing: 0.09em;
		text-transform: uppercase;
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		text-align: center;
		padding: 0.5rem;
		flex-shrink: 0;
		transition: background-color 150ms ease, transform 150ms ease;
	}

	:global(.editorial-circle-cta:hover:not(:disabled)) {
		background-color: #e4e4e7;
	}

	:global(.editorial-circle-cta:disabled) {
		opacity: 0.5;
		cursor: not-allowed;
	}

	@media (max-width: 900px) {
		.auth-split {
			grid-template-columns: 1fr;
		}

		.brand-pane {
			min-height: 200px;
			border-right: none;
			border-bottom: 1px solid #1b1b1d;
			padding: 1.5rem;
		}

		.emblem-stage {
			min-height: 110px;
		}

		.swiss-star {
			width: 72px;
			height: 72px;
		}

		.form-pane {
			padding: 1.75rem 1.5rem 2.5rem;
		}

		:global(.editorial-grid-2) {
			grid-template-columns: 1fr;
			gap: 1.25rem;
		}
	}
</style>

