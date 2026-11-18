<script lang="ts">
	import Button from './Button.svelte';

	let { application, account, scopes, pending = false, onDecision }: {
		application: string;
		account?: string;
		scopes: { title: string; description: string }[];
		pending?: boolean;
		onDecision: (approved: boolean) => void;
	} = $props();
</script>

<section aria-label="Access request" aria-busy={pending} class="task-surface grid min-w-0 gap-4 p-4 sm:p-5">
	<div class="flex min-w-0 items-center gap-3">
		<span class="grid size-10 shrink-0 place-items-center rounded-xl border border-action/35 bg-action/15 text-action-focus" aria-hidden="true">
			<svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="10" width="14" height="11" rx="2" /><path d="M8 10V7a4 4 0 1 1 8 0v3m-4 4v3" /></svg>
		</span>
		<div class="min-w-0">
			<h2 class="font-heading text-base font-bold break-words">{application}</h2>
			{#if account}
				<p class="mt-0.5 text-xs break-words text-brand-text-secondary">Signed in as {account}</p>
			{/if}
		</div>
	</div>

	<details open class="group rounded-lg border border-brand-border bg-brand-surface-input">
		<summary class="flex min-h-10 cursor-pointer list-none items-center justify-between gap-3 rounded-lg px-3 text-sm font-semibold focus-visible:outline-2 focus-visible:outline-action-focus [&::-webkit-details-marker]:hidden">
			<span>Requested permissions</span>
			<svg class="size-4 shrink-0 text-brand-text-secondary transition-transform group-open:rotate-180" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
		</summary>
		{#if scopes.length > 0}
			<ul class="m-0 list-none divide-y divide-brand-border border-t border-brand-border p-0">
				{#each scopes as scope}
					<li class="grid gap-1 px-3 py-2.5">
						<strong class="text-sm font-medium">{scope.title}</strong>
						<span class="text-xs leading-relaxed text-brand-text-secondary">{scope.description}</span>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="border-t border-brand-border px-3 py-2.5 text-sm text-brand-text-secondary">No additional permissions requested.</p>
		{/if}
	</details>

	<div class="flex flex-wrap justify-end gap-3">
		<Button size="sm" disabled={pending} onclick={() => onDecision(false)}>Decline</Button>
		<Button size="sm" disabled={pending} onclick={() => onDecision(true)}>Approve access</Button>
	</div>
	{#if pending}<p role="status" class="text-right text-xs text-brand-text-secondary">Saving your decision…</p>{/if}
</section>
