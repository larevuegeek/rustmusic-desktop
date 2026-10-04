<script lang="ts">
// Page d'un genre : mosaïque et chiffres, ses albums, ses artistes, puis tous ses titres.
import { page } from "$app/state";
import Icon from "@iconify/svelte";
import { untrack } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "$lib/i18n";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { handleTracksPlay } from "$lib/actions/queue/QueueAction";
import { clesAffichees, resoudreColonnes, lireLargeurs, resetTagCache, trierPistes, type SortDir } from "$lib/config/trackColumns";
import { dureeEcoute } from "$lib/helper/tools/dateTools";
import { teinte } from "$lib/helper/tools/teinte";
import { iconeGenre } from "$lib/helper/tools/iconeGenre";
import { cleTri, comparerNaturel } from "$lib/helper/library/cleTri";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import DetailPage from "$lib/components/ui/layout/DetailPage.svelte";
import PageState from "$lib/components/ui/layout/PageState.svelte";
import PlayButton from "$lib/components/ui/button/PlayButton.svelte";
import RoundButton from "$lib/components/ui/button/RoundButton.svelte";
import ToolbarButton from "$lib/components/ui/button/ToolbarButton.svelte";
import ChipLink from "$lib/components/ui/link/ChipLink.svelte";
import SegmentedControl from "$lib/components/ui/input/SegmentedControl.svelte";
import ViewModeSwitch from "$lib/components/ui/input/ViewModeSwitch.svelte";
import SelectionToggle from "$lib/components/ui/selection/SelectionToggle.svelte";
import CarouselSection from "$lib/components/ui/carousel/CarouselSection.svelte";
import TrackColumnsButton from "$lib/components/library/track/TrackColumnsButton.svelte";
import CollectionContextMenu from "$lib/components/ui/contextmenu/CollectionContextMenu.svelte";
import AlbumTrackRow from "$lib/components/library/album/AlbumTrackRow.svelte";
import LibraryTrackTable from "$lib/components/library/track/LibraryTrackTable.svelte";
import AlbumListItem from "$lib/components/library/album/AlbumListItem.svelte";
import AlbumListRow from "$lib/components/library/album/AlbumListRow.svelte";
import ArtistListItem from "$lib/components/library/artist/ArtistListItem.svelte";
import ArtistListRow from "$lib/components/library/artist/ArtistListRow.svelte";
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";
import type { ArtistListView } from "$lib/types/ui/library/artist/ArtistListView";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";

const libraryId = $derived(Number(page.params.library_id));
// Déjà décodé par SvelteKit : un second décodage plantait sur « 100% Hits ».
const genre = $derived(String((page.params as Record<string, string>).genre ?? ""));

let tracks = $state<TrackListView[]>([]);
let charge = $state(false);
let erreur = $state(false);
let defilement = $state<HTMLDivElement | null>(null);
let tour = 0;

$effect(() => {
  const id = libraryId;
  const g = genre;
  if (!id || !g) return;
  const n = ++tour;
  untrack(() => charger(id, g, n));
});

async function charger(id: number, g: string, n: number) {
  charge = false;
  erreur = false;
  tracks = [];
  try {
    const r = await invoke<TrackListView[]>("get_tracks_by_genre", { libraryId: id, genre: g });
    if (n !== tour) return;
    resetTagCache();
    tracks = r ?? [];
    defilement?.scrollTo({ top: 0 });
  } catch (e) {
    if (n === tour) erreur = true;
    console.error("[genre] chargement :", e);
  } finally {
    if (n === tour) charge = true;
  }
}

// ─── Albums et artistes du genre (déjà en mémoire) ───
const parTitre = (x: string, y: string) => comparerNaturel.compare(cleTri(x), cleTri(y));
const albums = $derived(
  $libraryContentStore.albums
    .filter((a) => a.genre?.toLowerCase() === genre.toLowerCase())
    .sort((x, y) => (y.year ?? 0) - (x.year ?? 0) || parTitre(x.title, y.title)),
);
// Les plus présents d'abord : le nombre de titres du genre, pas de toute leur carrière.
const artistes = $derived.by((): ArtistListView[] => {
  const n = new Map<string, number>();
  for (const x of tracks) if (x.artist_id) n.set(x.artist_id, (n.get(x.artist_id) ?? 0) + 1);
  const parId = new Map($libraryContentStore.artists.map((a) => [a.id, a]));
  return [...n.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([id]) => parId.get(id))
    .filter((a): a is ArtistListView => !!a);
});

// ─── Chiffres et teinte ───
const h = $derived(teinte(genre));
const couleur = $derived(`oklch(0.7 0.17 ${h})`);
const pochettes = $derived(albums.map((a) => a.cover_url).filter((c): c is string => !!c).slice(0, 4));
const nombre = (n: number) => n.toLocaleString($currentLocale);
const plusieurs = (n: number, un: string, n_: string) => $t(n === 1 ? un : n_);
const duree = $derived(tracks.reduce((s, x) => s + (x.duration ?? 0), 0));
const periode = $derived.by(() => {
  const ans = albums.map((a) => a.year ?? 0).filter((y) => y > 0);
  if (!ans.length) return null;
  const [min, max] = [Math.min(...ans), Math.max(...ans)];
  return min === max ? String(min) : `${min} – ${max}`;
});

// ─── Tous les titres ───
let tri = $state<string>("title");
let sens = $state<SortDir>("asc");
const tries = $derived(trierPistes(tracks, tri, sens));
// Un genre peut compter des milliers de titres : un lot d'abord, le reste à la demande.
const LOT = 200;
let affiches = $state(LOT);
$effect(() => {
  void genre;
  affiches = LOT;
});
const visibles = $derived(tries.slice(0, affiches));

function trierPar(cle: string) {
  if (tri === cle) sens = sens === "asc" ? "desc" : "asc";
  else {
    tri = cle;
    sens = cle === "year" || cle === "created_at" ? "desc" : "asc";
  }
}
const flecheSens = (cle: string) => (tri === cle ? (sens === "asc" ? "material-symbols:arrow-upward-rounded" : "material-symbols:arrow-downward-rounded") : null);

const grille = $derived($viewMode !== "list");
$effect(() => {
  if (grille) selectionStore.setOrder(visibles.map((x) => ({ id: x.id, track: x })));
});

const colonnes = $derived(resoudreColonnes(clesAffichees($settingsStore.track_columns)).filter((c) => c.key !== "genre"));
const largeurs = $derived(lireLargeurs($settingsStore.track_column_widths));

let menu = $state<{ x: number; y: number } | null>(null);
</script>

<!-- Cartes et lignes des sections (hors de DetailPage : sinon ce seraient ses props). -->
{#snippet carteAlbum(a: AlbumListView)}<AlbumListItem {libraryId} album={a} />{/snippet}
{#snippet ligneAlbum(a: AlbumListView)}<AlbumListRow {libraryId} album={a} />{/snippet}
{#snippet carteArtiste(a: ArtistListView)}<ArtistListItem {libraryId} artist={a} />{/snippet}
{#snippet ligneArtiste(a: ArtistListView)}<ArtistListRow {libraryId} artist={a} />{/snippet}

<!-- Mosaïque du genre : quatre pochettes, sinon des dégradés dans sa teinte. -->
{#snippet mosaique(taille: string, icone: number)}
  <div class="relative {taille} shrink-0 rounded-[14px] overflow-hidden grid grid-cols-2 grid-rows-2 gap-0.5 isolate bg-(--rg-bg)
              shadow-[0_24px_60px_rgba(0,0,0,0.3)] dark:shadow-[0_24px_60px_rgba(0,0,0,0.55)]">
    {#each [0, 1, 2, 3] as k (k)}
      {#if pochettes[k]}
        <CoverImg path={pochettes[k]} alt="" size="2x" class="w-full h-full object-cover" />
      {:else}
        <span style="background: linear-gradient({135 + k * 40}deg, oklch({0.55 - k * 0.06} 0.1 {(h + k * 23) % 360}), oklch(0.2 0.04 {(h + k * 23) % 360}))"></span>
      {/if}
    {/each}
    {#if pochettes.length === 0}
      <span class="absolute inset-0 flex items-center justify-center text-white/85"><Icon icon={iconeGenre(genre)} width={icone} /></span>
    {/if}
    <span class="absolute inset-0 rounded-[14px] ring-1 ring-inset ring-black/5 dark:ring-white/6 pointer-events-none"></span>
  </div>
{/snippet}

{#if erreur}
  <PageState icon="material-symbols:sell-outline-rounded" message={$t("genre_view.error")} lien={{ href: `/library/${libraryId}/genres`, label: $t("genre_view.back") }} />

{:else if !charge && tracks.length === 0}
  <PageState chargement message={$t("genre_view.loading")} />

{:else}
<DetailPage bind:defilement image={pochettes[0] ?? null} retourHref={`/library/${libraryId}/genres`} retourLabel={$t("genre_view.back")}
            playLabel={$t("artist_view.play_all")} onplay={() => handleTracksPlay(tries)}>
  {#snippet mini()}
    {@render mosaique("w-8.5 h-8.5 rounded-[7px]!", 16)}
    <b class="truncate text-[15px] text-(--rg-tx)">{genre}</b>
  {/snippet}
  {#snippet barre()}<ViewModeSwitch class="bg-(--rg-carte)/80" />{/snippet}

    <!-- ─── En-tête ─── -->
    <div class="relative flex flex-wrap items-end gap-8 pt-3 pb-7" style="--gc: {couleur}">
      {@render mosaique("w-55 h-55 @max-[760px]:w-40 @max-[760px]:h-40", 64)}

      <div class="flex-[1_1_320px] min-w-0 flex flex-col gap-3">
        <div class="flex items-center gap-2 text-xs font-bold tracking-[0.1em] uppercase text-(--rg-tx2)">
          <span class="w-5 h-0.75 rounded-full bg-(--gc)"></span>{$t("genre_view.kicker")}
        </div>
        <h1 class="font-extrabold leading-none tracking-[-0.03em] text-balance line-clamp-2 text-(--rg-tx)
                   {genre.length > 24 ? 'text-[56px] @max-[760px]:text-[34px]' : 'text-[72px] @max-[760px]:text-[44px]'}" title={genre}>{genre}</h1>

        <div class="flex flex-wrap items-center gap-x-3 gap-y-2 text-[15px] text-(--rg-mu)">
          {#if albums.length}
            <span><b class="font-bold text-(--rg-tx)">{nombre(albums.length)}</b> {plusieurs(albums.length, "library_head.albums_one", "library_head.albums_n")}</span>
            <span class="text-(--rg-mu2)">·</span>
          {/if}
          <span><b class="font-bold text-(--rg-tx)">{nombre(tracks.length)}</b> {plusieurs(tracks.length, "library_head.tracks_one", "library_head.tracks_n")}</span>
          {#if artistes.length}
            <span class="text-(--rg-mu2)">·</span>
            <span><b class="font-bold text-(--rg-tx)">{nombre(artistes.length)}</b> {plusieurs(artistes.length, "library_head.artists_one", "library_head.artists_n")}</span>
          {/if}
          {#if duree}
            <span class="text-(--rg-mu2)">·</span>
            <span>{dureeEcoute(duree)}</span>
          {/if}
          {#if periode}
            <span class="text-(--rg-mu2)">·</span>
            <span>{periode}</span>
          {/if}
        </div>

        <div class="flex flex-wrap items-center gap-2.5 mt-2">
          <PlayButton label={$t("artist_view.play_all")} disabled={!tracks.length} onclick={() => handleTracksPlay(tries)} />
          <RoundButton icon="material-symbols:shuffle-rounded" title={$t("album_view.shuffle")} disabled={!tracks.length} onclick={() => handleTracksPlay(tracks, true)} />
          <RoundButton icon="material-symbols:more-horiz" title={$t("album_view.more")}
                       onclick={(e) => { const r = (e.currentTarget as HTMLElement).getBoundingClientRect(); menu = { x: r.left, y: r.bottom + 6 }; }} />
          {#if artistes.length}
            <div class="flex flex-wrap gap-1.5 ml-1.5 @max-[760px]:ml-0">
              {#each artistes.slice(0, 3) as a (a.id)}<ChipLink href={`/library/${libraryId}/artists/${a.id}`} label={a.name} title={$t("genre_view.artist_title")} />{/each}
            </div>
          {/if}
        </div>
      </div>
    </div>

    {#if albums.length > 0}
      <CarouselSection class="mt-3" titre={$t("genre_view.albums")}
                       sousTitre={`${nombre(albums.length)} ${plusieurs(albums.length, "library_head.albums_one", "library_head.albums_n")}`}
                       {grille} items={albums} cle={(a) => a.id} carte={carteAlbum} ligne={ligneAlbum} />
    {/if}
    {#if artistes.length > 0}
      <CarouselSection titre={$t("genre_view.artists")} sousTitre={$t("genre_view.artists_desc")}
                       {grille} items={artistes.slice(0, 30)} cle={(a) => a.id} carte={carteArtiste} ligne={ligneArtiste} />
    {/if}

    <!-- ─── Tous les titres ─── -->
    <section class="mt-12">
      <h2 class="mb-2 text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("artist_view.all_tracks")}</h2>

      <div class="flex flex-wrap items-center gap-2.5 pt-1 pb-3.5">
        <span class="mr-auto text-[13px] text-(--rg-mu)"><b class="font-semibold text-(--rg-tx2)">{nombre(tracks.length)}</b> {plusieurs(tracks.length, "library_head.tracks_one", "library_head.tracks_n")}{#if duree}{" · "}{dureeEcoute(duree)}{/if}</span>

        <SegmentedControl variant="discret" label={$t("common.sort")} value={tri} onchange={trierPar}
                          options={[
                            { value: "title", label: $t("artist_view.sort_title"), after: flecheSens("title") },
                            { value: "artist", label: $t("genre_view.sort_artist"), after: flecheSens("artist") },
                            { value: "year", label: $t("artist_view.sort_year"), after: flecheSens("year") },
                            { value: "created_at", label: $t("genre_view.sort_added"), after: flecheSens("created_at") },
                          ]} />
        {#if !grille}<TrackColumnsButton {libraryId} avecLibelle />{/if}
        <SelectionToggle avecLibelle />
      </div>

      {#if !charge}
        <div class="flex justify-center py-10 text-(--rg-mu)"><Icon icon="material-symbols:progress-activity" width="22" class="animate-spin" /></div>
      {:else if tracks.length === 0}
        <p class="py-10 text-center text-sm text-(--rg-mu)">{$t("genre_view.empty")}</p>
      {:else if grille}
        <!-- Deux colonnes lues de haut en bas, une seule quand la place manque. -->
        <div class="grid grid-cols-2 grid-flow-col gap-x-7 gap-y-0.5 grid-rows-[repeat(var(--rangs),auto)]
                    @max-[900px]:grid-cols-1 @max-[900px]:grid-flow-row @max-[900px]:grid-rows-none"
             style="--rangs: {Math.ceil(visibles.length / 2)}">
          {#each visibles as track, i (track.id)}
            <AlbumTrackRow {libraryId} {track} tracks={tries} jaquette position={i + 1} numero={i + 1}
                           sousTitre={[track.artist, track.album].filter(Boolean).join(" · ")} />
          {/each}
        </div>
      {:else}
        <LibraryTrackTable {libraryId} tracks={visibles} columns={colonnes} {largeurs} barreHorizontale sortKey={tri} sortDir={sens} colle={60}
                           onsort={(cle, dir) => { tri = cle; sens = dir; }} />
      {/if}

      {#if tries.length > affiches}
        <div class="flex justify-center mt-5">
          <ToolbarButton icon="material-symbols:expand-more-rounded" label={$t("genre_view.show_more").replace("{n}", nombre(Math.min(LOT, tries.length - affiches)))}
                         onclick={() => (affiches += LOT)} />
        </div>
      {/if}
    </section>
</DetailPage>

{#if menu}
  <CollectionContextMenu title={genre} type="genre" loadTracks={async () => tries} x={menu.x} y={menu.y} onclose={() => (menu = null)} {libraryId} />
{/if}
{/if}
