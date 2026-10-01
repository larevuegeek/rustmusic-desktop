<script lang="ts">
  // Ligne de menu : icône, libellé, fin optionnelle (compteur, coche) ; courante en vert, dangereuse en rouge.
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";

  let {
    icon = null,
    actif = false,
    danger = false,
    onclick,
    children,
    fin,
  }: {
    icon?: string | null;
    actif?: boolean;
    danger?: boolean;
    onclick: () => void;
    children: Snippet;
    fin?: Snippet;
  } = $props();
</script>

<button
  type="button"
  role="menuitem"
  class="w-full flex items-center gap-2.5 px-2.5 py-2.25 rounded-lg text-[13px] text-left cursor-pointer transition-colors
         hover:bg-(--rg-s2) dark:hover:bg-[#252b28]
         {actif ? 'text-(--rg-gtx) font-semibold' : danger ? 'text-red-600 dark:text-red-400' : 'text-(--rg-tx)'}"
  {onclick}
>
  {#if icon}<Icon {icon} width="18" class="shrink-0 {actif || danger ? '' : 'text-(--rg-mu)'}" />{/if}
  <span class="flex-1 min-w-0 truncate">{@render children()}</span>
  {#if fin}{@render fin()}{/if}
</button>
