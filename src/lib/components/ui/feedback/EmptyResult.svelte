<script lang="ts">
  // Liste vide après filtrage : message et, si des filtres sont actifs, un lien pour les effacer.
  import Icon from "@iconify/svelte";

  let { message, effacerLabel, oneffacer = null }: {
    message: string;
    effacerLabel: string;
    /** `null` : aucun filtre actif, pas de lien. */
    oneffacer?: (() => void) | null;
  } = $props();
</script>

<div class="flex flex-col items-center gap-3 py-16 text-center text-(--rg-mu)">
  <Icon icon="material-symbols:search-off-rounded" width="32" />
  <p class="text-sm">{message}</p>
  {#if oneffacer}
    <button type="button" class="text-sm font-semibold cursor-pointer text-(--rg-gtx) hover:underline" onclick={oneffacer}>{effacerLabel}</button>
  {/if}
</div>
