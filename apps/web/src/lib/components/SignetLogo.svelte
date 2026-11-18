<script lang="ts">
  import SignetMark from "./SignetMark.svelte";

  interface Props {
    variant?: "light" | "dark";
    height?: number;
    label?: string;
    class?: string;
  }

  let {
    variant = "light",
    height = 24,
    label = "Signet",
    class: className = "",
  }: Props = $props();

  const isDefaultBrand = $derived(label.trim().toLowerCase() === "signet");
  const src = $derived(
    variant === "dark"
      ? "/brand/signet-logo-dark.png"
      : "/brand/signet-logo-light.png",
  );
</script>

{#if isDefaultBrand}
  <img
    {src}
    alt="Signet"
    class="block w-auto object-contain select-none {className}"
    style={`height: ${height}px`}
  />
{:else}
  <span
    class="inline-flex items-center gap-2 text-brand-text-primary {className}"
  >
    <SignetMark size={Math.round(height * 0.9)} />
    <span class="font-heading font-bold text-base tracking-tight leading-none"
      >{label}</span
    >
  </span>
{/if}
