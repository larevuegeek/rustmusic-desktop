<script lang="ts">
  import Icon from "@iconify/svelte";
  import TrackListCompact from "$lib/components/library/track/TrackListCompact.svelte";
  import TrackColumnsPopin from "$lib/components/library/common/popin/TrackColumnsPopin.svelte";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import {
    lireColonnes,
    resoudreColonnes,
    type SortDir,
  } from "$lib/config/trackColumns";
  import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";

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

  const columnKeys = $derived(lireColonnes($settingsStore.track_columns));
  const columns = $derived(resoudreColonnes(columnKeys));

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

<!-- En-tête. Les largeurs viennent de la même définition que les lignes, et
     les deux colonnes muettes de fin reproduisent le cœur et le menu. -->
<div class="flex items-center gap-3 py-1 px-2 mb-1 text-[10px] uppercase tracking-wider text-neutral-400
            border-b border-neutral-200/60 dark:border-white/5">
  <div class="w-5 text-right shrink-0">#</div>
  <div class="w-8 shrink-0"></div>

  <button
    type="button"
    onclick={() => trier('title')}
    class="flex-1 min-w-0 flex items-center gap-1 text-left uppercase cursor-pointer
           hover:text-neutral-700 dark:hover:text-neutral-200"
  >
    <span class="truncate">Titre</span>
    {#if fleche('title')}
      <Icon icon={fleche('title')!} width="11" class="shrink-0 text-emerald-500" />
    {/if}
  </button>

  {#each columns as col (col.key)}
    <button
      type="button"
      onclick={() => trier(col.key)}
      class="shrink-0 flex items-center gap-1 uppercase cursor-pointer
             hover:text-neutral-700 dark:hover:text-neutral-200
             {col.width} {col.align === 'right' ? 'justify-end' : ''}"
      title={col.label}
    >
      <span class="truncate">{col.label}</span>
      {#if fleche(col.key)}
        <Icon icon={fleche(col.key)!} width="11" class="shrink-0 text-emerald-500" />
      {/if}
    </button>
  {/each}

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
  <TrackListCompact {libraryId} {track} {columns} />
{/each}

{#if showColumns}
  <TrackColumnsPopin
    bind:open={showColumns}
    {libraryId}
    colonnes={columnKeys}
    onchange={handleColumnsChange}
  />
{/if}
