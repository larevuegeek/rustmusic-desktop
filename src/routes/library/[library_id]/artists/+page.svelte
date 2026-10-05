<script lang="ts">
import { lireLocal, ecrireLocal } from "#lib/helper/tools/stockage";
import EmptyResult from "#lib/components/ui/feedback/EmptyResult.svelte";
import SearchField from "#lib/components/ui/input/SearchField.svelte";
import TextButton from "#lib/components/ui/button/TextButton.svelte";
import MenuSelect from "#lib/components/ui/menu/MenuSelect.svelte";
import ViewModeSwitch from "#lib/components/ui/input/ViewModeSwitch.svelte";
import SelectionToggle from "#lib/components/ui/selection/SelectionToggle.svelte";
import { page } from "$app/state";
import Icon from "@iconify/svelte";
import { t, currentLocale } from "#lib/i18n";
import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
import { libraryStore } from "#lib/stores/library/library.store";
import { viewMode } from "#lib/stores/ui/viewMode.store";
import { artistImageReadyStore } from "#lib/stores/library/artistImageReady.store";
import { handleAddFiles, handleAddDirectory } from "#lib/actions/library/LibraryAction";
import { cleTri, lettreTri, comparerNaturel } from "#lib/helper/library/cleTri";
import { differe } from "#lib/helper/tools/differe.svelte";
import LibraryArtistSkeleton from "#lib/components/library/common/skeleton/LibraryArtistSkeleton.svelte";
import LibraryImportingLoader from "#lib/components/library/common/loader/LibraryImportingLoader.svelte";
import { importAffiche } from "#lib/stores/library/importProgress.store";
import ArtistListItem from "#lib/components/library/artist/ArtistListItem.svelte";
import ArtistListRow from "#lib/components/library/artist/ArtistListRow.svelte";
import AlphabetNav from "#lib/components/ui/alphabet/AlphabetNav.svelte";
import FilterChip from "#lib/components/ui/input/FilterChip.svelte";
import Menu from "#lib/components/ui/menu/Menu.svelte";
import MenuItem from "#lib/components/ui/menu/MenuItem.svelte";
import type { ArtistListView } from "#lib/types/ui/library/artist/ArtistListView";

const libraryId = $derived(Number(page.params.library_id));
const currentLibrary = $derived($libraryStore.libraries.find((l) => l.id === libraryId));

// ─── Tri (retenu d'une visite à l'autre) ───
type Tri = "name" | "albums" | "tracks" | "duration";
const TRIS: { cle: Tri; libelle: string; icone: string }[] = [
  { cle: "name", libelle: "artists_view.sort_name", icone: "material-symbols:sort-by-alpha-rounded" },
  { cle: "albums", libelle: "artists_view.sort_albums", icone: "material-symbols:album-outline-rounded" },
  { cle: "tracks", libelle: "artists_view.sort_tracks", icone: "material-symbols:music-note-rounded" },
  { cle: "duration", libelle: "artists_view.sort_duration", icone: "material-symbols:schedule-outline-rounded" },
];
const CLE_TRI = "artistes:tri";
const triLu = lireLocal(CLE_TRI, "name");
let tri = $state<Tri>(TRIS.some((x) => x.cle === triLu) ? (triLu as Tri) : "name");
$effect(() => ecrireLocal(CLE_TRI, tri));

// ─── Filtres ───
let recherche = $state("");
const cherche = differe(() => recherche);
let sansPortrait = $state(false);
const filtresActifs = $derived(!!recherche.trim() || sansPortrait);
function effacerFiltres() {
  recherche = "";
  sansPortrait = false;
}

const artistes = $derived($libraryContentStore.artists);
// Un portrait arrivé pendant la visite compte : l'artiste quitte « Sans portrait ».
const aUnPortrait = (a: ArtistListView) => !!(artistImageReadyStore.get(a.id, $artistImageReadyStore) ?? a.thumbnail_path);

// Clés calculées une fois par liste et par tri : la frappe ne fait plus que filtrer.
type Entree = { a: ArtistListView; cle: string; lettre: string; nom: string };
const parNom = (x: Entree, y: Entree) => comparerNaturel.compare(x.cle, y.cle);
const TRI_FN: Record<Tri, (x: Entree, y: Entree) => number> = {
  name: parNom,
  albums: (x, y) => y.a.total_albums - x.a.total_albums || parNom(x, y),
  tracks: (x, y) => y.a.total_tracks - x.a.total_tracks || parNom(x, y),
  duration: (x, y) => (y.a.total_duration ?? 0) - (x.a.total_duration ?? 0) || parNom(x, y),
};
const tries = $derived(
  artistes.map((a) => ({ a, cle: cleTri(a.name), lettre: lettreTri(a.name), nom: (a.name ?? "").toLowerCase() })).sort(TRI_FN[tri]),
);

const filtres = $derived.by(() => {
  const q = cherche.valeur.trim().toLowerCase();
  return tries.filter((e) => (!q || e.nom.includes(q)) && (!sansPortrait || !aUnPortrait(e.a)));
});
const visibles = $derived(filtres.map((e) => e.a));


// ─── Navigation A–Z (tri par nom seulement) ───
let defilement = $state<HTMLDivElement | null>(null);
const lettres = $derived(new Set(filtres.map((e) => e.lettre)));
const premiere = (i: number) => tri === "name" && (i === 0 || filtres[i - 1].lettre !== filtres[i].lettre);
function allerLettre(l: string) {
  // `scrollIntoView` : les cartes hors écran ont une hauteur présumée (`content-visibility`).
  defilement?.querySelector(`[data-letter="${l}"]`)?.scrollIntoView({ behavior: "smooth", block: "start" });
}

const nombre = (n: number) => n.toLocaleString($currentLocale);
</script>

{#if $importAffiche}
  <LibraryImportingLoader />

{:else if currentLibrary?.total_artists === 0}
  <div class="flex flex-col items-center justify-center py-20 px-6 text-center">
    <div class="w-16 h-16 mb-5 rounded-full border flex items-center justify-center bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-g)">
      <Icon icon="material-symbols:mic-external-on-outline-rounded" width="30" />
    </div>
    <h3 class="text-base font-semibold text-(--rg-tx) mb-1.5">{$t("library.no_artist")}</h3>
    <p class="text-sm text-(--rg-mu) max-w-xs leading-relaxed mb-6">{$t("library.no_artist_desc")}</p>
    <div class="flex items-center gap-2">
      <button class="h-10 px-4 flex items-center gap-2 rounded-[10px] text-sm font-bold cursor-pointer bg-(--rg-g) text-(--rg-on-g) hover:bg-[#34d673]"
              onclick={() => libraryId && handleAddFiles(libraryId)}>
        <Icon icon="material-symbols:upload-file-outline-rounded" width="18" />{$t("library.import_files")}
      </button>
      <button class="h-10 px-4 flex items-center gap-2 rounded-[10px] text-sm font-semibold cursor-pointer border border-(--rg-bd2) text-(--rg-tx2) hover:text-(--rg-tx) hover:bg-(--rg-carte)"
              onclick={() => libraryId && handleAddDirectory(libraryId)}>
        <Icon icon="material-symbols:create-new-folder-outline-rounded" width="18" />{$t("library.import_folder")}
      </button>
    </div>
  </div>

{:else if $libraryContentStore.isLoading}
  <LibraryArtistSkeleton />

{:else}
  <div class="flex flex-col h-full">
    <!-- ─── Outils : une ligne si la place le permet ; sinon recherche + vue en haut, filtres dessous ─── -->
    <div class="@container shrink-0 pl-4 md:pl-8 pr-4 md:pr-14 py-3">
      <div class="flex flex-wrap items-center gap-2.5">
        <SearchField bind:value={recherche} class="order-1 w-65 @max-6xl:w-auto @max-6xl:flex-1 min-w-36" placeholder={$t("artists_view.filter").replace("{n}", nombre(artistes.length))} clearLabel={$t("artists_view.clear_filters")} />

        <div class="order-2 @max-6xl:order-4 @max-6xl:basis-full flex flex-wrap items-center gap-2.5">
          <FilterChip icon="material-symbols:no-accounts-outline-rounded" pressed={sansPortrait} title={$t("artists_view.no_portrait")} onclick={() => (sansPortrait = !sansPortrait)}>
            <span class="@max-xl:hidden">{$t("artists_view.no_portrait")}</span>
          </FilterChip>
          {#if filtresActifs}
            <TextButton icon="material-symbols:filter-alt-off-outline-rounded" label={$t("artists_view.clear_filters")} labelClass="@max-3xl:hidden" onclick={effacerFiltres} />
          {/if}
        </div>

        <span class="order-3 flex-1 @max-6xl:hidden"></span>

        <div class="order-3 flex items-center gap-2.5">
          <MenuSelect value={tri} options={TRIS.map((x) => ({ value: x.cle, label: $t(x.libelle), icon: x.icone }))} onchange={(v) => (tri = v as typeof tri)} labelClass="@max-xl:hidden" menuClass="w-52.5" />

          <ViewModeSwitch />

          <SelectionToggle />
        </div>
      </div>
    </div>

    <!-- ─── Artistes ─── -->
    <div class="flex-1 relative min-h-0">
      <div class="absolute inset-0 overflow-y-auto scrollbar-app pl-4 md:pl-8 pr-10 md:pr-14 pb-16" bind:this={defilement}>
        {#if visibles.length === 0}
          <EmptyResult message={$t("artists_view.no_match")} effacerLabel={$t("artists_view.clear_filters")} oneffacer={filtresActifs ? effacerFiltres : null} />
        {:else if $viewMode === "grid"}
          <div class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-x-5 gap-y-7 pt-3">
            {#each visibles as artist, i (artist.id)}
              <div class="relative min-w-0">
                {#if premiere(i)}<span class="absolute -top-3 left-0 h-0 scroll-mt-4" data-letter={lettreTri(artist.name)}></span>{/if}
                <ArtistListItem {artist} {libraryId} />
              </div>
            {/each}
          </div>
        {:else}
          <div class="flex flex-col pt-2">
            {#each visibles as artist, i (artist.id)}
              {#if premiere(i)}<span class="h-0 scroll-mt-4" data-letter={lettreTri(artist.name)}></span>{/if}
              <ArtistListRow {artist} {libraryId} />
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
