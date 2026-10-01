<script lang="ts" generics="T">
  // Section titrée : carrousel de cartes (flèches dans l'en-tête) en grille, lignes en liste.
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";
  import { t } from "$lib/i18n";
  import Carousel from "./Carousel.svelte";

  let {
    titre,
    sousTitre = "",
    grille,
    items,
    cle,
    carte,
    ligne,
    extra,
    reinit = null,
    class: classes = "mt-12",
  }: {
    titre: string;
    sousTitre?: string;
    grille: boolean;
    items: T[];
    cle: (x: T) => string;
    carte: Snippet<[T]>;
    ligne: Snippet<[T]>;
    /** Contrôles placés avant les flèches (tri…). */
    extra?: Snippet;
    /** Change → carrousel recréé (sinon le défilement suit la carte accrochée quand l'ordre change). */
    reinit?: unknown;
    class?: string;
  } = $props();

  let carrousel = $state<Carousel | undefined>();
  let avant = $state(false);
  let apres = $state(false);
  const fleche = "w-8.5 h-8.5 rounded-full border flex items-center justify-center cursor-pointer transition-colors border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx) disabled:opacity-30 disabled:cursor-default";
</script>

<section class={classes}>
  <div class="flex flex-wrap items-end gap-3 mb-4">
    <div class="flex flex-col gap-1 min-w-0">
      <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{titre}</h2>
      {#if sousTitre}<span class="text-[13px] text-(--rg-mu)">{sousTitre}</span>{/if}
    </div>
    <span class="flex-1"></span>
    {@render extra?.()}
    <!-- Sans débordement, pas de flèches. -->
    {#if grille && (avant || apres)}
      <button type="button" class={fleche} disabled={!avant} title={$t("common.previous")} aria-label={$t("common.previous")} onclick={() => carrousel?.defiler(-1)}>
        <Icon icon="material-symbols:chevron-left-rounded" width="22" />
      </button>
      <button type="button" class={fleche} disabled={!apres} title={$t("common.next")} aria-label={$t("common.next")} onclick={() => carrousel?.defiler(1)}>
        <Icon icon="material-symbols:chevron-right-rounded" width="22" />
      </button>
    {/if}
  </div>
  {#if grille}
    {#key reinit}
      <Carousel bind:this={carrousel} fleches={false} bind:avant bind:apres class="gap-4.5 pb-1">
        {#each items as x (cle(x))}
          <div class="snap-start shrink-0 w-[clamp(150px,16%,200px)]">{@render carte(x)}</div>
        {/each}
      </Carousel>
    {/key}
  {:else}
    <div class="flex flex-col">
      {#each items as x (cle(x))}{@render ligne(x)}{/each}
    </div>
  {/if}
</section>
