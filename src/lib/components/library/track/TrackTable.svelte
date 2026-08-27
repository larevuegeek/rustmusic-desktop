<script lang="ts">
  import Icon from "@iconify/svelte";
  import TrackListCompact from "$lib/components/library/track/TrackListCompact.svelte";
  import TrackColumnsPopin from "$lib/components/library/common/popin/TrackColumnsPopin.svelte";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import {
    lireColonnes,
    resoudreColonnes,
    lireLargeurs,
    largeurDe,
    estFlexible,
    styleCellule,
    largeurAjustee,
    LARGEUR_MIN,
    LARGEUR_MAX,
    type SortDir,
    type TrackColumn,
  } from "$lib/config/trackColumns";
  import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
  import { selectionStore } from "$lib/stores/ui/selection.store";

  /**
   * La liste de pistes en tableau, colonnes au choix et tri par en-tête.
   *
   * Le composant ne trie rien lui-même : il annonce le tri demandé et affiche
   * ce qu'on lui donne. C'est ce qui lui permet de servir aussi bien une page
   * d'album — où la centaine de pistes tient en mémoire et se range
   * instantanément — que l'onglet Morceaux, où quatorze mille pistes sont
   * rangées par la base et rapportées page par page. Trier sur place dans le
   * second cas ne classerait que ce qui est déjà chargé, ce qui est pire que
   * de ne pas trier : le résultat aurait l'air juste.
   */
  let {
    libraryId,
    tracks,
    sortKey = null,
    sortDir = "asc",
    onsort,
  }: {
    /**
     * Bibliothèque à interroger pour recenser les tags proposables en colonne.
     *
     * `null` est permis : une page de playlist peut s'afficher avant que les
     * bibliothèques ne soient chargées, ou sur un profil qui n'en a aucune. Le
     * tableau se dessine quand même — seule la liste des tags du sélecteur
     * reste vide, ce qui est exactement ce qu'elle vaudrait de toute façon.
     */
    libraryId: number | null;
    tracks: TrackListView[];
    /** Colonne actuellement triée, `null` si l'ordre est celui d'origine. */
    sortKey?: string | null;
    sortDir?: SortDir;
    /** Appelé au clic sur un en-tête. Au parent de réordonner ou de recharger. */
    onsort?: (key: string, dir: SortDir) => void;
  } = $props();

  let showColumns = $state(false);

  // ─── L'ordre affiché, pour la sélection par plage ───
  //
  // « De celui-ci à celui-là » suppose un entre-deux, et seule la vue le
  // connaît : le tri, le filtre et la pagination ne laissent pas deux listes du
  // même contenu dans le même ordre.
  $effect(() => {
    selectionStore.setOrder(tracks.map((t) => ({ id: t.id, track: t })));
  });

  const columnKeys = $derived(lireColonnes($settingsStore.track_columns));
  const columns = $derived(resoudreColonnes(columnKeys));

  // ─── Largeurs ───
  //
  // Réglées par colonne et enregistrées : une largeur qu'on ajuste à chaque
  // ouverture n'est pas un réglage, c'est une corvée.
  const largeurs = $derived(lireLargeurs($settingsStore.track_column_widths));

  /**
   * Largeur en cours de tirage, tant que le bouton n'est pas relâché.
   *
   * Séparée de la valeur enregistrée : écrire dans les réglages à chaque pixel
   * ferait une écriture en base par mouvement de souris.
   */
  let tirage = $state<{ cle: string; largeur: number } | null>(null);

  function largeurAffichee(col: TrackColumn): number {
    if (tirage?.cle === col.key) return tirage.largeur;
    return largeurDe(col, largeurs);
  }

  function enregistrerLargeur(cle: string, largeur: number) {
    const suivant = { ...largeurs, [cle]: Math.round(largeur) };
    settingsStore.set("track_column_widths", JSON.stringify(suivant));
  }

  /** Démarre un tirage. Suivi et relâchement sont capturés sur la fenêtre. */
  function commencerTirage(e: PointerEvent, col: TrackColumn) {
    // Le tirage ne doit pas déclencher le tri de la colonne qu'on saisit.
    e.preventDefault();
    e.stopPropagation();

    const depart = e.clientX;

    // La largeur de départ se mesure à l'écran, pas dans les réglages.
    // Une colonne souple — le titre — occupe la place restante, souvent bien
    // plus que sa largeur de base : partir de celle-ci ferait sauter la colonne
    // au premier pixel de mouvement.
    const cellule = (e.currentTarget as HTMLElement)?.parentElement;
    const initiale = cellule
      ? Math.round(cellule.getBoundingClientRect().width)
      : largeurAffichee(col);
    tirage = { cle: col.key, largeur: initiale };

    function suivre(ev: PointerEvent) {
      const proposee = initiale + (ev.clientX - depart);
      tirage = {
        cle: col.key,
        largeur: Math.min(LARGEUR_MAX, Math.max(LARGEUR_MIN, proposee)),
      };
    }

    function relacher() {
      window.removeEventListener("pointermove", suivre);
      window.removeEventListener("pointerup", relacher);
      if (tirage) enregistrerLargeur(tirage.cle, tirage.largeur);
      tirage = null;
    }

    window.addEventListener("pointermove", suivre);
    window.addEventListener("pointerup", relacher);
  }

  /**
   * Ajuste une colonne à son contenu, au double-clic sur la poignée.
   *
   * C'est le geste qu'on connaît d'un tableur, et il évite d'avoir à viser une
   * largeur à la souris quand on veut simplement « que tout tienne ».
   */
  function ajusterAuContenu(col: TrackColumn) {
    enregistrerLargeur(col.key, largeurAjustee(col, tracks));
  }

  /** Rend toutes les colonnes à leur largeur d'origine. */
  function reinitialiserLargeurs() {
    settingsStore.set("track_column_widths", "{}");
  }

  function handleColumnsChange(cles: string[]) {
    settingsStore.set("track_columns", JSON.stringify(cles));
  }

  /**
   * Premier clic : croissant. Clic suivant sur la même colonne : on inverse.
   *
   * Repartir du croissant en changeant de colonne évite l'effet de sens
   * hérité — cliquer « Durée » après avoir mis « Titre » en décroissant
   * donnerait sinon les morceaux les plus longs d'abord, sans qu'on l'ait
   * demandé.
   */
  function trier(key: string) {
    const suivant: SortDir = sortKey === key && sortDir === "asc" ? "desc" : "asc";
    onsort?.(key, suivant);
  }

  function fleche(key: string): string | null {
    if (sortKey !== key) return null;
    return sortDir === "asc" ? "lucide:arrow-up" : "lucide:arrow-down";
  }
</script>

<!-- ─── En-tête ───

     Collant en haut de la zone de défilement : sans lui, on perd le nom des
     colonnes dès la deuxième page, et une valeur sans intitulé ne veut rien
     dire — surtout quand ce sont des tags qu'on a soi-même choisis.

     Le fond doit être opaque : les lignes passent dessous. Une transparence,
     même floutée, laisserait les titres défiler à travers les intitulés, et le
     mode contraste élevé neutralise de toute façon les flous d'arrière-plan.

     `z-10` le place au-dessus des lignes, dont certaines portent des éléments
     positionnés. -->
<div class="sticky top-0 z-10 flex items-center gap-3 py-2 px-2 mb-1
            text-[10px] uppercase tracking-wider text-neutral-400
            bg-neutral-50 dark:bg-zinc-950
            border-b border-neutral-200/60 dark:border-white/5">
  {#each columns as col (col.key)}
    {@const w = largeurAffichee(col)}
    <!-- Chaque colonne est un conteneur de largeur fixe, doublé d'une poignée
         posée sur son bord droit. La poignée ne peut pas vivre dans le bouton
         de tri : un appui dessus déclencherait le tri de la colonne qu'on
         voulait simplement élargir. -->
    <div
      class="relative shrink-0 group/col"
      style={styleCellule(col, largeurs)}
    >
      {#if col.widget === 'cover'}
        <!-- La pochette ne se trie pas, et n'a pas d'intitulé qui tiendrait
             dans huit pixels : simple réserve de place. -->
        <div class="h-4"></div>
      {:else}
        <button
          type="button"
          onclick={() => trier(col.key)}
          class="w-full flex items-center gap-1 uppercase cursor-pointer
                 hover:text-neutral-700 dark:hover:text-neutral-200
                 {col.align === 'right' ? 'justify-end' : ''}"
          title={col.label}
        >
          <span class="truncate">{col.label}</span>
          {#if fleche(col.key)}
            <Icon icon={fleche(col.key)!} width="11" class="shrink-0 text-emerald-500" />
          {/if}
        </button>
      {/if}

      <!-- Poignée. Large de six pixels et débordant à droite, pour rester
           saisissable sans mordre sur l'intitulé. Elle ne s'éclaire qu'au
           survol de sa colonne : une rangée de traits verticaux en permanence
           alourdirait un en-tête qu'on lit bien plus souvent qu'on ne le
           redimensionne. -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="absolute top-0 -right-1.5 w-3 h-full cursor-col-resize z-20
               flex items-center justify-center"
        onpointerdown={(e) => commencerTirage(e, col)}
        ondblclick={(e) => { e.stopPropagation(); ajusterAuContenu(col); }}
        title="Tirer pour redimensionner · double-clic pour ajuster au contenu"
      >
        <div class="w-px h-3 rounded-full transition-colors
                    {tirage?.cle === col.key
                      ? 'bg-emerald-500'
                      : 'bg-transparent group-hover/col:bg-neutral-300 dark:group-hover/col:bg-white/20'}">
        </div>
      </div>
    </div>
  {/each}

  <!-- Remplissage : dès qu'aucune colonne n'absorbe la place restante — titre
       masqué, ou titre dont on a fixé la largeur — il faut bien que quelque
       chose la prenne, sinon les commandes de fin de ligne remonteraient se
       coller aux colonnes. -->
  {#if !columns.some(c => estFlexible(c, largeurs))}
    <div class="flex-1 min-w-0"></div>
  {/if}

  <div class="w-[21px] shrink-0"></div>
  <button
    type="button"
    onclick={() => showColumns = true}
    class="w-[22px] shrink-0 flex items-center justify-center cursor-pointer
           text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200"
    aria-label="Choisir les colonnes"
    title="Choisir les colonnes"
  >
    <Icon icon="lucide:columns-3" width="13" />
  </button>
</div>

{#each tracks as track (track.id)}
  <TrackListCompact {libraryId} {track} {columns} {largeurs} />
{/each}

{#if showColumns}
  <TrackColumnsPopin
    bind:open={showColumns}
    {libraryId}
    colonnes={columnKeys}
    onchange={handleColumnsChange}
    onresetwidths={reinitialiserLargeurs}
  />
{/if}
