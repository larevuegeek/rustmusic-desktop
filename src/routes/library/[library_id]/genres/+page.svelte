<script lang="ts">
import { lireLocal, ecrireLocal } from "$lib/helper/tools/stockage";
import EmptyResult from "$lib/components/ui/feedback/EmptyResult.svelte";
import SearchField from "$lib/components/ui/input/SearchField.svelte";
import TextButton from "$lib/components/ui/button/TextButton.svelte";
import MenuSelect from "$lib/components/ui/menu/MenuSelect.svelte";
import ViewModeSwitch from "$lib/components/ui/input/ViewModeSwitch.svelte";
import SelectionToggle from "$lib/components/ui/selection/SelectionToggle.svelte";
import { page } from "$app/state";
import { onDestroy } from "svelte";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "$lib/i18n";
import { libraryHeader } from "$lib/stores/library/libraryHeader";
import { libraryStore } from "$lib/stores/library/library.store";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import { handleRandomMix } from "$lib/actions/queue/QueueAction";
import { cleTri, lettreTri } from "$lib/helper/library/cleTri";
import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
import GenreCard from "$lib/components/library/genre/GenreCard.svelte";
import GenreListRow from "$lib/components/library/genre/GenreListRow.svelte";
import AlphabetNav from "$lib/components/ui/alphabet/AlphabetNav.svelte";
import FilterChip from "$lib/components/ui/input/FilterChip.svelte";
import Menu from "$lib/components/ui/menu/Menu.svelte";
import MenuItem from "$lib/components/ui/menu/MenuItem.svelte";
import type { GenreView } from "$lib/types/ui/library/genre/GenreView";

const libraryId = $derived(Number(page.params.library_id));
const currentLibrary = $derived($libraryStore.libraries.find((l) => l.id === libraryId));

// ─── Genres de la bibliothèque ───
let genres = $state<GenreView[]>([]);
let chargement = $state(true);
$effect(() => {
  const id = libraryId;
  chargement = true;
  invoke<GenreView[]>("get_genres", { libraryId: id })
    .then((g) => { if (id === libraryId) genres = g; })
    .catch(() => (genres = []))
    .finally(() => (chargement = false));
});

// Artistes phares et dernière écoute : calculés par la base (plus besoin de charger tous les titres).
const artistesDe = (g: GenreView) => (g.top_artists ?? []).join(", ");

// Albums sans genre : un pseudo-genre qui mène à la vue Albums filtrée.
const sansGenre = $derived.by((): GenreView => {
  const albums = $libraryContentStore.albums.filter((a) => !a.genre);
  return {
    name: "",
    total_albums: albums.length,
    total_tracks: albums.reduce((s, a) => s + (a.total_tracks ?? 0), 0),
    covers: albums.map((a) => a.cover_url).filter((c): c is string => !!c).slice(0, 4),
  };
});

// ─── Tri (retenu d'une visite à l'autre) ───
type Tri = "tracks" | "albums" | "name" | "recent";
const TRIS: { cle: Tri; libelle: string; icone: string }[] = [
  { cle: "tracks", libelle: "genres_view.sort_tracks", icone: "material-symbols:library-music-outline-rounded" },
  { cle: "albums", libelle: "genres_view.sort_albums", icone: "material-symbols:album-outline-rounded" },
  { cle: "name", libelle: "genres_view.sort_name", icone: "material-symbols:sort-by-alpha-rounded" },
  { cle: "recent", libelle: "genres_view.sort_recent", icone: "material-symbols:history-rounded" },
];
const CLE_TRI = "genres:tri";
const triLu = lireLocal(CLE_TRI, "tracks");
let tri = $state<Tri>(TRIS.some((x) => x.cle === triLu) ? (triLu as Tri) : "tracks");
$effect(() => ecrireLocal(CLE_TRI, tri));

// ─── Filtres ───
let recherche = $state("");
let principaux = $state(false);
let voirSansGenre = $state(false);
const filtresActifs = $derived(!!recherche.trim() || principaux || voirSansGenre);
function effacerFiltres() {
  recherche = "";
  principaux = voirSansGenre = false;
}

const totalTitres = $derived(currentLibrary?.total_tracks || genres.reduce((s, g) => s + g.total_tracks, 0));
// « Principaux » : au moins 3 % des titres (et 50 au minimum).
const seuil = $derived(Math.max(50, totalTitres * 0.03));
const maxTitres = $derived(Math.max(1, ...genres.map((g) => g.total_tracks)));

const comparer = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });
const parNom = (x: GenreView, y: GenreView) => comparer.compare(cleTri(x.name), cleTri(y.name));
const TRI_FN: Record<Tri, (x: GenreView, y: GenreView) => number> = {
  tracks: (x, y) => y.total_tracks - x.total_tracks || parNom(x, y),
  albums: (x, y) => y.total_albums - x.total_albums || parNom(x, y),
  name: parNom,
  recent: (x, y) => (y.last_played_at ?? "").localeCompare(x.last_played_at ?? "") || parNom(x, y),
};

const visibles = $derived.by(() => {
  if (voirSansGenre) return [];
  const q = recherche.trim().toLowerCase();
  return genres
    .filter((g) => (!q || `${g.name} ${artistesDe(g)}`.toLowerCase().includes(q)) && (!principaux || g.total_tracks >= seuil))
    .sort(TRI_FN[tri]);
});

// ─── En-tête : chiffres des genres, et « Mix aléatoire » ───
$effect(() => {
  const n = genres.length;
  const albums = currentLibrary?.total_albums ?? 0;
  const sans = sansGenre.total_albums;
  libraryHeader.update(() => ({
    action: { cle: "genres_view.random_mix", icone: "material-symbols:shuffle-rounded", lancer: () => handleRandomMix(libraryId) },
    chiffres: [
      { n, un: "library_head.genres_one", plusieurs: "library_head.genres_n" },
      { n: albums, un: "library_head.albums_one", plusieurs: "library_head.albums_n" },
      { n: totalTitres, un: "library_head.tracks_one", plusieurs: "library_head.tracks_n" },
      { n: sans, un: "library_head.untagged_one", plusieurs: "library_head.untagged_n" },
    ],
  }));
});
onDestroy(() => libraryHeader.update((h) => ({ ...h, action: null, chiffres: null })));

// ─── Navigation A–Z (tri par nom seulement) ───
let defilement = $state<HTMLDivElement | null>(null);
const lettres = $derived(new Set(visibles.map((g) => lettreTri(g.name))));
const premiere = (i: number) => tri === "name" && (i === 0 || lettreTri(visibles[i - 1].name) !== lettreTri(visibles[i].name));
function allerLettre(l: string) {
  defilement?.querySelector(`[data-letter="${l}"]`)?.scrollIntoView({ behavior: "smooth", block: "start" });
}

const nombre = (n: number) => n.toLocaleString($currentLocale);
const lienSansGenre = $derived(`/library/${libraryId}/albums?genre=__sans__`);
</script>

{#if $libraryStore.isImporting}
  <LibraryImportingLoader />

{:else if chargement && genres.length === 0}
  <div class="flex items-center justify-center py-20 text-(--rg-mu)">
    <Icon icon="material-symbols:progress-activity" width="26" class="animate-spin" />
  </div>

{:else if genres.length === 0}
  <div class="flex flex-col items-center justify-center py-20 px-6 text-center">
    <div class="w-16 h-16 mb-5 rounded-2xl border flex items-center justify-center bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-g)">
      <Icon icon="material-symbols:sell-outline-rounded" width="30" />
    </div>
    <h3 class="text-base font-semibold text-(--rg-tx) mb-1.5">{$t("library.no_genre")}</h3>
    <p class="text-sm text-(--rg-mu) max-w-xs leading-relaxed">{$t("library.no_genre_desc")}</p>
  </div>

{:else}
  <div class="flex flex-col h-full">
    <!-- ─── Outils : une ligne si la place le permet ; sinon recherche + vue en haut, filtres dessous ─── -->
    <div class="@container shrink-0 pl-4 md:pl-8 pr-4 md:pr-14 py-3">
      <div class="flex flex-wrap items-center gap-2.5">
        <SearchField bind:value={recherche} class="order-1 w-65 @max-6xl:w-auto @max-6xl:flex-1 min-w-36" placeholder={$t("genres_view.filter").replace("{n}", nombre(genres.length))} clearLabel={$t("genres_view.clear_filters")} />

        <div class="order-2 @max-6xl:order-4 @max-6xl:basis-full flex flex-wrap items-center gap-2.5">
          <FilterChip icon="material-symbols:trending-up-rounded" pressed={principaux} title={$t("genres_view.main")} onclick={() => { principaux = !principaux; voirSansGenre = false; }}>
            <span class="@max-xl:hidden">{$t("genres_view.main")}</span>
          </FilterChip>
          {#if sansGenre.total_albums > 0}
            <FilterChip icon="material-symbols:label-off-outline-rounded" pressed={voirSansGenre} title={$t("genres_view.untagged_hint")} onclick={() => { voirSansGenre = !voirSansGenre; principaux = false; }}>
              <span class="@max-xl:hidden">{$t("genres_view.untagged")}</span>
              <span class="text-[11px] font-medium text-(--rg-mu2)">{nombre(sansGenre.total_albums)}</span>
            </FilterChip>
          {/if}
          {#if filtresActifs}
            <TextButton icon="material-symbols:filter-alt-off-outline-rounded" label={$t("genres_view.clear_filters")} labelClass="@max-3xl:hidden" onclick={effacerFiltres} />
          {/if}
        </div>

        <span class="order-3 flex-1 @max-6xl:hidden"></span>

        <div class="order-3 flex items-center gap-2.5">
          <MenuSelect value={tri} options={TRIS.map((x) => ({ value: x.cle, label: $t(x.libelle), icon: x.icone }))} onchange={(v) => (tri = v as typeof tri)} labelClass="@max-xl:hidden" menuClass="w-55" />

          <ViewModeSwitch />

          <SelectionToggle />
        </div>
      </div>
    </div>

    <!-- ─── Genres ─── -->
    <div class="flex-1 relative min-h-0">
      <div class="absolute inset-0 overflow-y-auto scrollbar-app pl-4 md:pl-8 pr-10 md:pr-14 pb-16" bind:this={defilement}>
        {#if voirSansGenre}
          <!-- Le pseudo-genre seul : il mène aux albums sans genre. -->
          {#if $viewMode === "grid"}
            <div class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] max-[900px]:grid-cols-[repeat(auto-fill,minmax(170px,1fr))] gap-x-4.5 gap-y-5.5 pt-3">
              <GenreCard {libraryId} genre={sansGenre} nom={$t("genres_view.untagged")} lien={lienSansGenre} artistes={$t("genres_view.untagged_hint")} special />
            </div>
          {:else}
            <div class="flex flex-col pt-2">
              <GenreListRow {libraryId} genre={sansGenre} nom={$t("genres_view.untagged")} lien={lienSansGenre} artistes={$t("genres_view.untagged_hint")}
                            part={(sansGenre.total_tracks / Math.max(1, totalTitres)) * 100} barre={(sansGenre.total_tracks / maxTitres) * 100} special />
            </div>
          {/if}
        {:else if visibles.length === 0}
          <EmptyResult message={$t("genres_view.no_match")} effacerLabel={$t("genres_view.clear_filters")} oneffacer={filtresActifs ? effacerFiltres : null} />
        {:else if $viewMode === "grid"}
          <div class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] max-[900px]:grid-cols-[repeat(auto-fill,minmax(170px,1fr))] gap-x-4.5 gap-y-5.5 pt-3">
            {#each visibles as genre, i (genre.name)}
              <div class="relative min-w-0">
                {#if premiere(i)}<span class="absolute -top-3 left-0 h-0 scroll-mt-4" data-letter={lettreTri(genre.name)}></span>{/if}
                <GenreCard {libraryId} {genre} artistes={artistesDe(genre)} />
              </div>
            {/each}
          </div>
        {:else}
          <div class="flex flex-col pt-2">
            {#each visibles as genre, i (genre.name)}
              {#if premiere(i)}<span class="h-0 scroll-mt-4" data-letter={lettreTri(genre.name)}></span>{/if}
              <GenreListRow {libraryId} {genre} artistes={artistesDe(genre)}
                            part={(genre.total_tracks / Math.max(1, totalTitres)) * 100} barre={(genre.total_tracks / maxTitres) * 100} />
            {/each}
          </div>
        {/if}
      </div>

      {#if tri === "name" && visibles.length > 0}
        <AlphabetNav availableLetters={lettres} onletter={allerLettre} toujours />
      {/if}
    </div>
  </div>
{/if}
