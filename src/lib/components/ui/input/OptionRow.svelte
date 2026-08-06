<script lang="ts">
  // Ligne d'option générique : icône + titre + description à gauche,
  // contrôle (toggle, select, bouton…) passé en children à droite.
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";

  let {
    icon,
    title,
    desc,
    value,
    iconClass = 'text-neutral-400',
    children,
  }: {
    icon: string;
    title: string;
    desc: string;
    /** Valeur technique optionnelle (chemin, port, device…) affichée en
     *  pastille monospace sous la description. Tronquée si trop longue,
     *  valeur complète au survol. */
    value?: string;
    iconClass?: string;
    children: Snippet;
  } = $props();
</script>

<div class="flex items-center justify-between gap-4 px-4 py-3 rounded-xl
            hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
  <div class="flex items-center gap-3 min-w-0">
    <Icon {icon} width="18" class="shrink-0 {iconClass}" />
    <div class="min-w-0">
      <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{title}</p>
      <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{desc}</p>
      {#if value}
        <p
          class="mt-1 inline-block max-w-full truncate rounded-md px-1.5 py-0.5 font-mono text-[10px]
                 bg-neutral-100 text-neutral-500
                 dark:bg-white/5 dark:text-neutral-400"
          title={value}
        >{value}</p>
      {/if}
    </div>
  </div>
  {@render children()}
</div>
