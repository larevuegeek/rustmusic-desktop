<script lang="ts">
  // Charpente des pages de détail : lueur teintée par l'image, barre collante (retour, mini-fiche passé l'en-tête, commandes, lecture).
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";
  import PlayButton from "$lib/components/ui/button/PlayButton.svelte";
  import { resolveCoverSrc } from "$lib/helper/tools/coverHelper";
  import { couleurPochette } from "$lib/helper/tools/couleurPochette";

  let {
    image = null,
    retourHref,
    retourLabel,
    playLabel,
    onplay,
    mini,
    barre,
    children,
    defilement = $bindable(null),
  }: {
    /** Pochette ou portrait : donne la teinte de la lueur. */
    image?: string | null;
    retourHref: string;
    retourLabel: string;
    playLabel: string;
    onplay?: () => void;
    /** Mini-fiche affichée dans la barre une fois l'en-tête dépassé. */
    mini: Snippet;
    /** Commandes à droite de la barre (vue, voisins…). */
    barre?: Snippet;
    children: Snippet;
    defilement?: HTMLDivElement | null;
  } = $props();

  let colle = $state(false);
  let teinte = $state({ h: 150, s: 0 });

  $effect(() => {
    const c = image;
    if (!c) {
      teinte = { h: 150, s: 0 };
      return;
    }
    let vivant = true;
    resolveCoverSrc(c, "1x")
      .then((src) => (src ? couleurPochette(src) : null))
      .then((r) => { if (vivant) teinte = r ?? { h: 150, s: 0 }; });
    return () => { vivant = false; };
  });
</script>

<div bind:this={defilement} onscroll={() => (colle = (defilement?.scrollTop ?? 0) > 260)} class="relative h-full overflow-y-auto scrollbar-app" style="--ah: {teinte.h}; --as: {teinte.s}">
  <div class="album-lueur absolute inset-x-0 top-0 h-115 pointer-events-none" aria-hidden="true"></div>

  <div class="@container relative max-w-350 px-4 md:px-8 pb-24">
    <div class="sticky top-0 z-20 h-15 -mx-4 md:-mx-8 px-4 md:px-8 flex items-center gap-3 border-b transition-colors duration-200
                {colle ? 'bg-(--c-fond)/90 dark:bg-[rgba(11,13,12,0.88)] backdrop-blur-md border-(--rg-bd)' : 'border-transparent'}">
      <a href={retourHref}
         class="h-8.5 pl-2 pr-3 shrink-0 flex items-center gap-1.5 rounded-[9px] text-sm font-semibold transition-colors text-(--rg-tx2) hover:bg-(--rg-s2) hover:text-(--rg-tx)">
        <Icon icon="material-symbols:arrow-back-rounded" width="20" />
        {retourLabel}
      </a>
      <div class="flex items-center gap-3 min-w-0 transition-all duration-200 {colle ? 'opacity-100' : 'opacity-0 translate-y-1.5 pointer-events-none'}" aria-hidden={!colle}>
        {@render mini()}
      </div>
      <span class="flex-1"></span>
      {@render barre?.()}
      {#if colle && onplay}<PlayButton size="md" title={playLabel} onclick={onplay} />{/if}
    </div>

    {@render children()}
  </div>
</div>
