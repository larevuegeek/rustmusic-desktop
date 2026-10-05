<script lang="ts">
import Icon from "@iconify/svelte";
import CoverImg from "#lib/components/ui/image/CoverImg.svelte";

/** Mosaïque dès quatre pochettes, une seule sinon, et à défaut un carré de la couleur. */
let { couvertures = [], couleur, icone = null }: {
  couvertures?: string[];
  couleur: string;
  icone?: string | null;
} = $props();
</script>

<span class="sb-vignette relative" style="--c: {couleur};">
  {#if couvertures.length >= 4}
    <span class="absolute inset-0 grid grid-cols-2 grid-rows-2">
      {#each couvertures.slice(0, 4) as chemin (chemin)}
        <CoverImg path={chemin} alt="" size="1x" class="w-full h-full object-cover" />
      {/each}
    </span>
  {:else if couvertures.length > 0}
    <CoverImg path={couvertures[0]} alt="" size="1x" class="absolute inset-0 w-full h-full object-cover" />
  {:else if icone}
    <Icon icon={icone} width="18" />
  {/if}
</span>
