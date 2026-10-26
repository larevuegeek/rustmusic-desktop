<script lang="ts">
  // Groupe d'options : intitulé en capitales, puis une carte de lignes `OptionItem`.
  // Avec `foldKey`, l'intitulé replie la carte (état retenu ; ouverte pendant une recherche).
  import { untrack, type Snippet } from "svelte";
  import { slide } from "svelte/transition";
  import Icon from "@iconify/svelte";
  import { readLocal, writeLocal } from "#lib/helper/tools/storage";
  import { rechercheReglages } from "#lib/stores/ui/settingsSearch.store";

  let {
    title,
    hint,
    action,
    foldKey,
    children,
  }: { title?: string; hint?: string; action?: Snippet; foldKey?: string; children: Snippet } = $props();

  // La clé ne change pas : lue une fois, au montage.
  let replie = $state(untrack(() => (foldKey ? readLocal(`replie:${foldKey}`, "0") === "1" : false)));
  const ferme = $derived(replie && !$rechercheReglages.trim());

  function basculer() {
    replie = !replie;
    if (foldKey) writeLocal(`replie:${foldKey}`, replie ? "1" : "0");
  }
</script>

<section class="rg-groupe">
  {#if title}
    <div class="flex items-baseline gap-2.5 px-1 pb-2.5 min-w-0 leading-[1.2]">
      {#if foldKey}
        <button type="button" class="group/pli flex-1 min-w-0 flex items-baseline gap-2.5 text-left cursor-pointer" aria-expanded={!ferme} onclick={basculer}>
          <Icon icon="material-symbols:keyboard-arrow-down-rounded" width="18"
                class="self-center shrink-0 -mx-0.5 text-(--rg-mu2) transition-transform duration-200 group-hover/pli:text-(--rg-tx2) {ferme ? '-rotate-90' : ''}" />
          <h2 class="shrink-0 text-[13px] font-bold uppercase tracking-[0.08em] text-(--rg-tx2)">{title}</h2>
          {#if hint}<p class="flex-1 min-w-0 text-[13px] text-(--rg-mu2) truncate">{hint}</p>{/if}
        </button>
      {:else}
        <h2 class="shrink-0 text-[13px] font-bold uppercase tracking-[0.08em] text-(--rg-tx2)">{title}</h2>
        {#if hint}<p class="flex-1 min-w-0 text-[13px] text-(--rg-mu2) truncate">{hint}</p>{/if}
      {/if}
      {#if action}<div class="self-center shrink-0 ml-auto">{@render action()}</div>{/if}
    </div>
  {/if}
  {#if !ferme}
    <div transition:slide={{ duration: foldKey ? 200 : 0 }} class="rg-carte rounded-[14px] bg-(--rg-carte) border border-(--rg-bd) overflow-hidden">
      {@render children()}
    </div>
  {/if}
</section>
