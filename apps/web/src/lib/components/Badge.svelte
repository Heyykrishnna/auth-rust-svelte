<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    variant?:
      | "primary"
      | "success"
      | "warning"
      | "danger"
      | "neutral"
      | "purple";
    size?: "sm" | "md";
    dot?: boolean;
    class?: string;
    children: Snippet;
  }

  let {
    variant = "neutral",
    size = "md",
    dot = false,
    class: className = "",
    children,
  }: Props = $props();

  const baseClasses =
    "inline-flex items-center gap-1.5 font-medium rounded-md tracking-wide whitespace-nowrap leading-none font-sans";

  const sizeClasses: Record<string, string> = {
    sm: "px-2 py-1 text-[11px]",
    md: "px-2.5 py-1.5 text-xs",
  };

  const variantClasses: Record<string, string> = {
    neutral:
      "bg-brand-surface-raised text-brand-text-primary border border-brand-border-hover",
    primary:
      "bg-brand-lime text-brand-lime-ink border border-brand-lime font-semibold",
    purple:
      "bg-brand-lime/10 text-brand-lime border border-brand-lime/30",
    success:
      "bg-brand-success-soft text-brand-success-text border border-brand-success-border",
    warning:
      "bg-brand-warning-soft text-brand-warning-text border border-brand-warning-border",
    danger:
      "bg-brand-danger-soft text-brand-danger-text border border-brand-danger-border",
  };

  const classes = $derived(
    `${baseClasses} ${sizeClasses[size] ?? sizeClasses.md} ${variantClasses[variant] ?? variantClasses.neutral} ${className}`
  );
</script>

<span class={classes}>
  {#if dot}
    <span class="size-1.5 rounded-full bg-current" aria-hidden="true"></span>
  {/if}
  {@render children()}
</span>
