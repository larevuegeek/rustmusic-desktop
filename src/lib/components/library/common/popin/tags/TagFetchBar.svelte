<script lang="ts">
  // Barre de la colonne des champs : ce que Deezer vient de remplir, et l'accès à la recherche.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import { t } from "$lib/i18n";

  let {
    filled,
    disabled,
    onfetch,
  }: {
    /** Clés des champs repris (et `cover`). */
    filled: string[];
    disabled: boolean;
    onfetch: () => void;
  } = $props();
</script>

<div class="shrink-0 flex items-center gap-2 px-4 py-2
            border-b border-neutral-200/70 dark:border-white/8">
  {#if filled.length > 0}
    <span class="flex-1 min-w-0 flex items-center gap-1.5 text-[11px] truncate
                 text-emerald-600 dark:text-emerald-400"
          transition:fade={{ duration: 120 }}>
      <Icon icon="lucide:sparkles" width="11" class="shrink-0" />
      <!-- Les champs sont nommés : « 6 champs » laisserait chercher lesquels. -->
      {filled.map((k) => $t(`tags.${k}`)).join(', ')}
    </span>
  {:else}
    <span class="flex-1"></span>
  {/if}
  <button
    type="button"
    onclick={onfetch}
    {disabled}
    class="shrink-0 flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[11.5px]
           cursor-pointer transition-colors disabled:opacity-40
           text-emerald-600 dark:text-emerald-400
           hover:bg-emerald-500/12"
  >
    <Icon icon="lucide:cloud-download" width="12" />
    {$t('source.fetch')}
  </button>
</div>
