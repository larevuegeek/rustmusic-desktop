<script lang="ts">
  // Tableau de l'onglet Morceaux : les colonnes au choix et redimensionnables de `TrackTable`, habillées comme la maquette.
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";
  import { t } from "#lib/i18n";
  import LibraryTrackRow from "./LibraryTrackRow.svelte";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { selectionStore } from "#lib/stores/ui/selection.store";
  import { largeurDe, largeurAjustee, libelleColonne, LARGEUR_MIN, LARGEUR_MAX, LARGEUR_MAX_SOUPLE, type SortDir, type TrackColumn } from "#lib/config/trackColumns";
  import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";

  let {
    libraryId,
    tracks,
    columns,
    largeurs,
    detaille = false,
    artisteSousTitre = true,
    sortKey = null,
    sortDir = "asc",
    onsort,
    colle = 0,
    positions = false,
    groupes = null,
    enteteGroupe,
    barreHorizontale = false,
  }: {
    libraryId: number;
    tracks: TrackListView[];
    columns: TrackColumn[];
    largeurs: Record<string, number>;
    detaille?: boolean;
    artisteSousTitre?: boolean;
    sortKey?: string | null;
    sortDir?: SortDir;
    /** Au parent de recharger : le tri se fait en base. */
    onsort?: (key: string, dir: SortDir) => void;
    /** Décalage de l'en-tête collant (px) ; `null` : en-tête non collant. */
    colle?: number | null;
    /** Sans numéro dans les tags, le rang de la ligne (une page d'album dans l'ordre du disque). */
    positions?: boolean;
    /** Lignes regroupées (par album) sous un seul en-tête de colonnes ; `tracks` reste la file de lecture. */
    groupes?: { cle: string; pistes: TrackListView[]; ouvert: boolean }[] | null;
    enteteGroupe?: Snippet<[string]>;
    /** Dans une page qui défile : la largeur en trop défile seule, par une barre collée en bas, sans emporter la page. */
    barreHorizontale?: boolean;
  } = $props();

  // ─── Défilement horizontal propre au tableau ───
  // `overflow-x: clip` ne crée pas de zone de défilement : les en-têtes collants suivent toujours la page.
  let largeurVue = $state(0);
  let largeurContenu = $state(0);
  let decalage = $state(0);
  let barre = $state<HTMLDivElement | null>(null);
  let cadre = $state<HTMLDivElement | null>(null);
  const deborde = $derived(barreHorizontale && largeurContenu > largeurVue + 1);
  const decalageEffectif = $derived(deborde ? Math.min(decalage, largeurContenu - largeurVue) : 0);

  // Molette horizontale ou Maj+molette : la barre défile, la page non.
  $effect(() => {
    if (!cadre || !deborde) return;
    const roue = (e: WheelEvent) => {
      const dx = e.shiftKey ? e.deltaY || e.deltaX : Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : 0;
      if (!dx || !barre) return;
      e.preventDefault();
      barre.scrollLeft += dx;
    };
    cadre.addEventListener("wheel", roue, { passive: false });
    return () => cadre?.removeEventListener("wheel", roue);
  });

  const visibles = $derived(groupes ? groupes.flatMap((g) => (g.ouvert ? g.pistes : [])) : tracks);

  // L'ordre affiché, pour la sélection par plage (Maj + clic).
  $effect(() => {
    selectionStore.setOrder(visibles.map((x) => ({ id: x.id, track: x })));
  });

  // ─── Largeurs ───
  // Posées en variables CSS sur le tableau (`--cw-i`) : pendant un tirage, une seule valeur
  // change et les lignes ne se redessinent pas. Le titre absorbe la place tant qu'on ne l'a pas réglé.
  let tirage = $state<{ cle: string; largeur: number } | null>(null);
  const effectives = $derived(tirage ? { ...largeurs, [tirage.cle]: tirage.largeur } : largeurs);
  const souple = $derived(columns.findIndex((c) => c.flexible && effectives[c.key] === undefined));

  function largeurColonne(col: TrackColumn, reglees: Record<string, number>): number {
    if (col.widget === "cover") return detaille ? 68 : 40;
    const w = largeurDe(col, reglees);
    if (col.widget === "index") return Math.max(w, 28);
    if (col.key === "duration") return Math.max(w, 44);
    return w;
  }
  const variables = $derived(columns.map((c, i) => `--cw-${i}: ${largeurColonne(c, effectives)}px`).join("; "));
  const styleCol = (i: number) =>
    i === souple ? `flex: 1 1 var(--cw-${i}); min-width: ${columns[i].minWidth ?? 0}px` : `width: var(--cw-${i})`;

  let ligneEntete = $state<HTMLDivElement | null>(null);

  function enregistrer(cle: string, largeur: number | null) {
    const suivant = { ...largeurs };
    if (largeur === null) delete suivant[cle];
    else suivant[cle] = Math.round(largeur);
    settingsStore.set("track_column_widths", JSON.stringify(suivant));
  }

  // Chaque poignée règle sa propre colonne ; le titre part de sa largeur à l'écran, sans à-coup.
  function tirer(e: PointerEvent, i: number) {
    e.preventDefault();
    e.stopPropagation();
    const col = columns[i];
    const depart = e.clientX;
    const initiale = Math.round(ligneEntete?.querySelector(`[data-col="${i}"]`)?.getBoundingClientRect().width ?? largeurDe(col, largeurs));
    const max = col.flexible ? LARGEUR_MAX_SOUPLE : LARGEUR_MAX;
    const min = Math.max(LARGEUR_MIN, col.flexible ? (col.minWidth ?? 0) : 0);
    tirage = { cle: col.key, largeur: initiale };
    const suivre = (ev: PointerEvent) => {
      tirage = { cle: col.key, largeur: Math.min(max, Math.max(min, initiale + ev.clientX - depart)) };
    };
    const relacher = () => {
      window.removeEventListener("pointermove", suivre);
      window.removeEventListener("pointerup", relacher);
      if (tirage) enregistrer(tirage.cle, tirage.largeur);
      tirage = null;
    };
    window.addEventListener("pointermove", suivre);
    window.addEventListener("pointerup", relacher);
  }

  // Double-clic : ajuste au contenu ; sur le titre, le rend à la largeur automatique.
  function ajuster(i: number) {
    const col = columns[i];
    if (col.flexible) enregistrer(col.key, null);
    else enregistrer(col.key, largeurAjustee(col, visibles));
  }

  // Premier clic croissant, le suivant sur la même colonne inverse.
  function trier(cle: string) {
    onsort?.(cle, sortKey === cle && sortDir === "asc" ? "desc" : "asc");
  }
</script>

<div bind:this={cadre} bind:clientWidth={largeurVue} class={barreHorizontale ? "overflow-x-clip" : ""}>
<!-- Aussi large que ses colonnes : une ligne plus étroite rognerait celles de droite (content-visibility). -->
<!-- --tx / --vue : décalage et largeur visible, pour qu'un en-tête de groupe reste en place. -->
<div class="w-max min-w-full" bind:offsetWidth={largeurContenu} style="--eq: var(--rg-g); {variables}; {deborde ? `--tx: ${decalageEffectif}px; --vue: ${largeurVue}px;` : ''}"
     style:transform={decalageEffectif ? `translateX(${-decalageEffectif}px)` : undefined}>
  <!-- En-tête collant, fond opaque : les lignes passent dessous. -->
  <div bind:this={ligneEntete} style:top={colle === null ? undefined : `${colle}px`} class="{colle === null ? '' : 'sticky'} z-10 flex items-center gap-3.5 h-9.5 px-2.5 border-b border-(--rg-bd)
              bg-(--c-fond) dark:bg-zinc-950 text-[11px] font-bold uppercase tracking-[0.08em] text-(--rg-mu2)">
    {#each columns as col, i (col.key)}
      <div data-col={i} class="relative shrink-0 group/col {i === souple ? 'min-w-0' : ''}" style={styleCol(i)}>
        {#if col.widget === "cover"}
          <span></span>
        {:else if col.widget === "index"}
          <span class="block text-center">#</span>
        {:else}
          <button type="button" class="w-full flex items-center gap-1 cursor-pointer whitespace-nowrap uppercase transition-colors hover:text-(--rg-tx2)
                                      {sortKey === col.key ? 'text-(--rg-g)!' : ''} {col.align === 'right' || col.key === 'duration' ? 'justify-end' : ''}"
                  title={libelleColonne(col, $t)} onclick={() => trier(col.key)}>
            {#if col.key === "duration"}
              <Icon icon="material-symbols:schedule-outline-rounded" width="16" />
            {:else}
              <span class="truncate">{libelleColonne(col, $t)}</span>
            {/if}
            {#if sortKey === col.key}
              <Icon icon={sortDir === "asc" ? "material-symbols:arrow-upward-rounded" : "material-symbols:arrow-downward-rounded"} width="14" class="shrink-0" />
            {/if}
          </button>
        {/if}
        {#if col.widget !== "cover" && col.widget !== "index"}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="absolute top-0 -right-2 w-3 h-full cursor-col-resize z-20 flex items-center justify-center"
               onpointerdown={(e) => tirer(e, i)}
               ondblclick={(e) => { e.stopPropagation(); ajuster(i); }}
               title={col.flexible ? $t("tracks_view.resize_title") : $t("tracks_view.resize")}>
            <div class="w-px h-3.5 rounded-full transition-colors {tirage?.cle === col.key ? 'bg-(--rg-g)' : 'bg-transparent group-hover/col:bg-(--rg-bd2)'}"></div>
          </div>
        {/if}
      </div>
    {/each}
    {#if souple < 0}
      <div class="flex-1 min-w-0"></div>
    {/if}
    <!-- Réserve des boutons de fin de ligne (j'aime, options). -->
    <div class="w-[78px] shrink-0"></div>
  </div>

  <div class="pt-1.5">
    {#if groupes}
      {#each groupes as g (g.cle)}
        {@render enteteGroupe?.(g.cle)}
        {#if g.ouvert}
          {#each g.pistes as track, i (track.id)}
            <LibraryTrackRow {libraryId} {track} {tracks} {columns} {souple} {detaille} {artisteSousTitre} rang={positions ? i + 1 : null} />
          {/each}
        {/if}
      {/each}
    {:else}
      {#each tracks as track, i (track.id)}
        <LibraryTrackRow {libraryId} {track} {tracks} {columns} {souple} {detaille} {artisteSousTitre} rang={positions ? i + 1 : null} />
      {/each}
    {/if}
  </div>
</div>
{#if deborde}
  <div bind:this={barre} onscroll={() => (decalage = barre?.scrollLeft ?? 0)} class="sticky bottom-0 z-10 overflow-x-auto scrollbar-app" aria-hidden="true">
    <div class="h-px" style:width="{largeurContenu}px"></div>
  </div>
{/if}
</div>
