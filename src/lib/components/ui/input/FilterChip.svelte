<script lang="ts">
  // Puce de filtre : bascule (verte une fois active) ou ouvre un menu (chevron).
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";

  let {
    icon = null,
    pressed = false,
    menu = false,
    title,
    onclick,
    children,
  }: {
    icon?: string | null;
    pressed?: boolean;
    /** Ouvre une liste de choix : chevron à droite. */
    menu?: boolean;
    /** Info-bulle, utile quand le libellé est masqué faute de place. */
    title?: string;
    onclick: () => void;
    children: Snippet;
  } = $props();
</script>

<button
  type="button"
  aria-pressed={pressed}
  {title}
  class="h-8.5 px-3 rounded-full border flex items-center gap-1.5 text-[13px] font-semibold whitespace-nowrap cursor-pointer transition-colors
         {pressed
           ? 'bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-gtx)'
           : 'bg-(--rg-carte) border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx)'}"
  {onclick}
>
  {#if icon}<Icon {icon} width="17" class="shrink-0" />{/if}
  {@render children()}
  {#if menu}<Icon icon="material-symbols:expand-more-rounded" width="17" class="shrink-0 -mr-1" />{/if}
</button>
