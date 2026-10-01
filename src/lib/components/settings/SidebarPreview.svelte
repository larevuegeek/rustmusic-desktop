<script lang="ts">
  // Barre latérale en miniature : ses sections suivent les interrupteurs voisins.
  import { t } from "$lib/i18n";

  let { ouvrir, albums, artistes }: { ouvrir: boolean; albums: boolean; artistes: boolean } = $props();

  const case_ = "bg-[#ebe7df] dark:bg-[#1b211e]";
  const barre = "h-1.25 flex-1 rounded-[3px] bg-[#d8d3c8] dark:bg-[#2a312d]";
  const titre = "text-[9px] font-bold tracking-[0.08em] uppercase leading-[1.2] text-(--rg-mu2)";
</script>

{#snippet ligne(teinte: number, vive: boolean, largeur = "100%")}
  <div class="flex items-center gap-1.5">
    <span class="w-4 h-4 shrink-0 rounded-[3px] {vive ? 'rg-teinte-vive' : 'rg-teinte'}" style:--h={teinte}></span>
    <span class={barre} style:max-width={largeur}></span>
  </div>
{/snippet}

<div class="flex flex-col gap-1.5 p-4 bg-(--rg-creux)" aria-hidden="true">
  <p class="mb-1 text-[11px] font-bold tracking-[0.08em] uppercase leading-[1.2] text-(--rg-mu2)">{$t("settings.preview")}</p>
  <div class="grid grid-cols-3 gap-1">
    {#each [0, 1, 2, 3, 4, 5] as i (i)}
      <span class="h-5.5 rounded-[5px] {i === 0 ? 'bg-(--rg-gbg) border border-(--rg-gbd)' : case_}"></span>
    {/each}
  </div>
  {#if ouvrir}
    <div class="flex gap-1 pt-1.5">
      <span class="flex-1 h-4 rounded {case_} border border-[#e2ddd3] dark:border-[#262d29]"></span>
      <span class="flex-1 h-4 rounded {case_} border border-[#e2ddd3] dark:border-[#262d29]"></span>
    </div>
  {/if}
  <div class="flex flex-col gap-1 pt-1.5">
    <em class="not-italic {titre}">{$t("nav.playlists")}</em>
    {@render ligne(55, false)}
    {@render ligne(290, false, "60%")}
  </div>
  {#if albums}
    <div class="flex flex-col gap-1 pt-1.5">
      <em class="not-italic {titre}">{$t("library.albums")}</em>
      {@render ligne(25, true)}
      {@render ligne(290, true, "70%")}
    </div>
  {/if}
  {#if artistes}
    <div class="flex flex-col gap-1 pt-1.5">
      <em class="not-italic {titre}">{$t("library.artists")}</em>
      <div class="flex gap-1.5">
        <span class="w-5 h-5 rounded-full rg-teinte-vive" style:--h={230}></span>
        <span class="w-5 h-5 rounded-full rg-teinte-vive" style:--h={300}></span>
      </div>
    </div>
  {/if}
</div>
