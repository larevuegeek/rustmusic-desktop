<script lang="ts">
  // Bouton secondaire à contour, icône facultative (roue de chargement pendant `busy`) ; `danger` le passe en rouge.
  import Icon from "@iconify/svelte";

  let {
    label,
    icon,
    onclick,
    busy = false,
    disabled = false,
    danger = false,
    title,
  }: { label: string; icon?: string; onclick: () => void; busy?: boolean; disabled?: boolean; danger?: boolean; title?: string } = $props();
</script>

<button
  type="button"
  class="shrink-0 h-8 px-3 flex items-center gap-1.5 rounded-lg border cursor-pointer transition-colors
         text-[13px] font-semibold whitespace-nowrap
         {danger
           ? 'border-red-500/40 text-red-600 dark:text-red-400 hover:bg-red-500/10 hover:border-red-500/60'
           : 'border-(--rg-off) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx)'}
         disabled:opacity-60 disabled:cursor-wait
         focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--rg-g)"
  disabled={disabled || busy}
  {title}
  {onclick}
>
  {#if icon}<Icon icon={busy ? "material-symbols:progress-activity" : icon} width="17" class={busy ? "animate-spin" : ""} />{/if}
  {label}
</button>
