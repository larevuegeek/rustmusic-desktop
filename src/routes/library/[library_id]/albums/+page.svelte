<script lang="ts">
import { lireLocal, ecrireLocal } from "$lib/helper/tools/stockage";
import EmptyResult from "$lib/components/ui/feedback/EmptyResult.svelte";
import SearchField from "$lib/components/ui/input/SearchField.svelte";
import TextButton from "$lib/components/ui/button/TextButton.svelte";
import MenuSelect from "$lib/components/ui/menu/MenuSelect.svelte";
import ViewModeSwitch from "$lib/components/ui/input/ViewModeSwitch.svelte";
import SelectionToggle from "$lib/components/ui/selection/SelectionToggle.svelte";
import { page } from "$app/state";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { t, currentLocale } from "$lib/i18n";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { ordreAlbums } from "$lib/stores/library/albumOrder.store";
import { libraryStore } from "$lib/stores/library/library.store";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import { handleAddFiles, handleAddDirectory } from "$lib/actions/library/LibraryAction";
import { etiquetteHiRes } from "$lib/helper/tools/audioFormatTools";
import { cleTri, lettreTri, comparerNaturel } from "$lib/helper/library/cleTri";
import { differe } from "$lib/helper/tools/differe.svelte";
import LibraryAlbumSkeleton from "$lib/components/library/common/skeleton/LibraryAlbumSkeleton.svelte";
import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
import AlbumListItem from "$lib/components/library/album/AlbumListItem.svelte";
import AlbumListRow from "$lib/components/library/album/AlbumListRow.svelte";
import AlphabetNav from "$lib/components/ui/alphabet/AlphabetNav.svelte";
import FilterChip from "$lib/components/ui/input/FilterChip.svelte";
import Menu from "$lib/components/ui/menu/Menu.svelte";
import MenuItem from "$lib/components/ui/menu/MenuItem.svelte";
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";

const libraryId = $derived(Number(page.params.library_id));
const currentLibrary = $derived($libraryStore.libraries.find((l) => l.id === libraryId));

// ─── Tri (retenu d'une visite à l'autre) ───
type Tri = "title" | "artist" | "year" | "added";
const TRIS: { cle: Tri; libelle: string; icone: string }[] = [
  { cle: "title", libelle: "albums_view.sort_title", icone: "material-symbols:sort-by-alpha-rounded" },
  { cle: "artist", libelle: "albums_view.sort_artist", icone: "material-symbols:person-outline-rounded" },
  { cle: "year", libelle: "albums_view.sort_year", icone: "material-symbols:calendar-month-outline-rounded" },
  { cle: "added", libelle: "albums_view.sort_added", icone: "material-symbols:schedule-outline-rounded" },
];
const CLE_TRI = "albums:tri";
const triLu = lireLocal(CLE_TRI, "title");
let tri = $state<Tri>(TRIS.some((x) => x.cle === triLu) ? (triLu as Tri) : "title");
$effect(() => ecrireLocal(CLE_TRI, tri));

// ─── Filtres ───
let recherche = $state("");
const cherche = differe(() => recherche);
let hiRes = $state(false);
let compilations = $state(false);
let sansPochette = $state(false);
// Depuis la vue Genres : `?genre=Rock`, ou `__sans__` pour les albums sans genre.
const SANS_GENRE = "__sans__";
let genre = $state<string | null>(page.url.searchParams.get("genre"));
// L'année vient de l'adresse (?annee=1991, depuis le lecteur) : elle se partage et survit au retour.
const annee = $derived(Number(page.url.searchParams.get("annee")) || null);

function choisirAnnee(y: number | null) {
  menuAnnee = false;
  const url = new URL(page.url);
  if (y) url.searchParams.set("annee", String(y));
  else url.searchParams.delete("annee");
  goto(url.pathname + url.search, { replaceState: true, keepFocus: true, noScroll: true });
}

let menuGenre = $state(false);
let menuAnnee = $state(false);

const cleTitre = cleTri;
const lettre = lettreTri;

const albums = $derived($libraryContentStore.albums);
const genres = $derived.by(() => {
  const n = new Map<string, number>();
  for (const a of albums) if (a.genre) n.set(a.genre, (n.get(a.genre) ?? 0) + 1);
  return [...n.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
});
const annees = $derived.by(() => {
  const n = new Map<number, number>();
  for (const a of albums) if (a.year) n.set(a.year, (n.get(a.year) ?? 0) + 1);
  return [...n.entries()].sort((a, b) => b[0] - a[0]);
});

// Clés calculées une fois par liste et par tri : la frappe ne fait plus que filtrer.
type Entree = { a: AlbumListView; cle: string; lettre: string; texte: string };
const comparer = comparerNaturel;
const TRI_FN: Record<Tri, (x: Entree, y: Entree) => number> = {
  title: (x, y) => comparer.compare(x.cle, y.cle),
  artist: (x, y) => comparer.compare(x.a.artist ?? "", y.a.artist ?? "") || comparer.compare(x.cle, y.cle),
  year: (x, y) => (y.a.year ?? 0) - (x.a.year ?? 0) || comparer.compare(x.cle, y.cle),
  added: (x, y) => String(y.a.created_at).localeCompare(String(x.a.created_at)),
};
const tries = $derived(
  albums.map((a) => ({ a, cle: cleTitre(a.title), lettre: lettre(a.title), texte: `${a.title} ${a.artist ?? ""} ${a.genre ?? ""}`.toLowerCase() })).sort(TRI_FN[tri]),
);

const filtres = $derived.by(() => {
  const q = cherche.valeur.trim().toLowerCase();
  return tries.filter(({ a, texte }) =>
    (!q || texte.includes(q)) &&
    (!hiRes || !!etiquetteHiRes(a.max_bits, a.max_sample_rate)) &&
    (!compilations || a.album_type === "compilation") &&
    (!sansPochette || !a.cover_url) &&
    (!genre || (genre === SANS_GENRE ? !a.genre : a.genre === genre)) &&
    (!annee || a.year === annee));
});
const visibles = $derived(filtres.map((e) => e.a));

const filtresActifs = $derived(!!recherche.trim() || hiRes || compilations || sansPochette || !!genre || !!annee);
function effacerFiltres() {
  recherche = "";
  hiRes = compilations = sansPochette = false;
  genre = null;
  if (annee) choisirAnnee(null);
}


// La page d'un album parcourt ses voisins dans cet ordre.
$effect(() => {
  ordreAlbums.set({ libraryId, ids: visibles.map((a) => a.id) });
});

// ─── Navigation A–Z (tri par titre seulement) ───
let defilement = $state<HTMLDivElement | null>(null);
const lettres = $derived(new Set(filtres.map((e) => e.lettre)));
const premiere = (i: number) => tri === "title" && (i === 0 || filtres[i - 1].lettre !== filtres[i].lettre);
function allerLettre(l: string) {
  // `scrollIntoView` : les cartes hors écran ont une hauteur présumée (`content-visibility`).
  defilement?.querySelector(`[data-letter="${l}"]`)?.scrollIntoView({ behavior: "smooth", block: "start" });
}

const nombre = (n: number) => n.toLocaleString($currentLocale);
</script>

{#if $libraryStore.isImporting}
  <LibraryImportingLoader />

{:else if currentLibrary?.total_albums === 0}
  <div class="flex flex-col items-center justify-center py-20 px-6 text-center">
    <div class="w-16 h-16 mb-5 rounded-2xl border flex items-center justify-center bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-g)">
      <Icon icon="material-symbols:album-outline-rounded" width="30" />
    </div>
    <h3 class="text-base font-semibold text-(--rg-tx) mb-1.5">{$t("library.no_album")}</h3>
    <p class="text-sm text-(--rg-mu) max-w-xs leading-relaxed mb-6">{$t("library.no_album_desc")}</p>
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
  <LibraryAlbumSkeleton />

{:else}
  <div class="flex flex-col h-full">
    <!-- ─── Outils : une ligne si la place le permet ; sinon recherche + vue en haut, filtres dessous ─── -->
    <div class="@container shrink-0 pl-4 md:pl-8 pr-4 md:pr-14 py-3">
    <div class="flex flex-wrap items-center gap-2.5">
      <SearchField bind:value={recherche} class="order-1 w-65 @max-6xl:w-auto @max-6xl:flex-1 min-w-36" placeholder={$t("albums_view.filter").replace("{n}", nombre(albums.length))} clearLabel={$t("albums_view.clear_filters")} />

      <div class="order-2 @max-6xl:order-4 @max-6xl:basis-full flex flex-wrap items-center gap-2.5">
      <!-- Très étroit : l'icône seule, le libellé reste en info-bulle. -->
      <FilterChip icon="material-symbols:high-quality-outline-rounded" pressed={hiRes} title={$t("albums_view.hires")} onclick={() => (hiRes = !hiRes)}><span class="@max-xl:hidden">{$t("albums_view.hires")}</span></FilterChip>
      <FilterChip icon="material-symbols:library-music-outline-rounded" pressed={compilations} title={$t("albums_view.compilations")} onclick={() => (compilations = !compilations)}><span class="@max-xl:hidden">{$t("albums_view.compilations")}</span></FilterChip>
      <FilterChip icon="material-symbols:hide-image-outline-rounded" pressed={sansPochette} title={$t("albums_view.no_cover")} onclick={() => (sansPochette = !sansPochette)}><span class="@max-xl:hidden">{$t("albums_view.no_cover")}</span></FilterChip>

      {#if genres.length > 0}
        <div class="relative">
          <FilterChip icon="material-symbols:sell-outline-rounded" pressed={!!genre} menu onclick={() => (menuGenre = !menuGenre)}>
            <span class="max-w-40 truncate">{genre === SANS_GENRE ? $t("albums_view.untagged") : (genre ?? $t("albums_view.genre"))}</span>
          </FilterChip>
          <Menu bind:open={menuGenre} align="left" class="w-60 max-h-80 overflow-y-auto scrollbar-app">
            <MenuItem actif={!genre} onclick={() => { genre = null; menuGenre = false; }}>{$t("albums_view.all_genres")}</MenuItem>
            <MenuItem actif={genre === SANS_GENRE} onclick={() => { genre = SANS_GENRE; menuGenre = false; }}>{$t("albums_view.untagged")}</MenuItem>
            {#each genres as [g, n] (g)}
              <MenuItem actif={genre === g} onclick={() => { genre = g; menuGenre = false; }}>
                {g}
                {#snippet fin()}<span class="font-mono text-[11px] text-(--rg-mu2)">{n}</span>{/snippet}
              </MenuItem>
            {/each}
          </Menu>
        </div>
      {/if}

      {#if annees.length > 0 || annee}
        <div class="relative">
          <FilterChip icon="material-symbols:calendar-month-outline-rounded" pressed={!!annee} menu onclick={() => (menuAnnee = !menuAnnee)}>
            {annee ?? $t("albums_view.year")}
          </FilterChip>
          <Menu bind:open={menuAnnee} align="left" class="w-48 max-h-80 overflow-y-auto scrollbar-app">
            <MenuItem actif={!annee} onclick={() => choisirAnnee(null)}>{$t("albums_view.all_years")}</MenuItem>
            {#each annees as [y, n] (y)}
              <MenuItem actif={annee === y} onclick={() => choisirAnnee(y)}>
                <span class="tabular-nums">{y}</span>
                {#snippet fin()}<span class="font-mono text-[11px] text-(--rg-mu2)">{n}</span>{/snippet}
              </MenuItem>
            {/each}
          </Menu>
        </div>
      {/if}

      {#if filtresActifs}
        <TextButton icon="material-symbols:filter-alt-off-outline-rounded" label={$t("albums_view.clear_filters")} labelClass="@max-3xl:hidden" onclick={effacerFiltres} />
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

    <!-- ─── Albums ─── -->
    <div class="flex-1 relative min-h-0">
      <div class="absolute inset-0 overflow-y-auto scrollbar-app pl-4 md:pl-8 pr-10 md:pr-14 pb-16" bind:this={defilement}>
        {#if visibles.length === 0}
          <EmptyResult message={annee && !recherche && !hiRes && !compilations && !sansPochette && !genre ? $t("library.year_none").replace("{y}", String(annee)) : $t("albums_view.no_match")} effacerLabel={$t("albums_view.clear_filters")} oneffacer={filtresActifs ? effacerFiltres : null} />
        {:else if $viewMode === "grid"}
          <div class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-x-5 gap-y-6.5 pt-3">
            {#each visibles as album, i (album.id)}
              <div class="relative min-w-0">
                {#if premiere(i)}<span class="absolute -top-3 left-0 h-0 scroll-mt-4" data-letter={lettre(album.title)}></span>{/if}
                <AlbumListItem {album} {libraryId} />
              </div>
            {/each}
          </div>
        {:else}
          <div class="flex flex-col pt-2">
            {#each visibles as album, i (album.id)}
              {#if premiere(i)}<span class="h-0 scroll-mt-4" data-letter={lettre(album.title)}></span>{/if}
              <AlbumListRow {album} {libraryId} />
            {/each}
          </div>
        {/if}
      </div>

      {#if tri === "title" && visibles.length > 0}
        <AlphabetNav availableLetters={lettres} onletter={allerLettre} toujours />
      {/if}
    </div>
  </div>
{/if}
