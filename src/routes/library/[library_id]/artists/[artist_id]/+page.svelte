<script lang="ts">
import { page } from "$app/state";
import ArtistListRow from "$lib/components/library/artist/ArtistListRow.svelte";
import AlbumListRow from "$lib/components/library/album/AlbumListRow.svelte";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { goto } from "$app/navigation";
import { profilSelector } from "$lib/stores/profil/profil.store";
import type { Library } from "$lib/types/db/library/Library";
import type { ArtistDetailView } from "$lib/types/ui/library/artist/ArtistDetailView";
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
import { loadArtist } from "$lib/services/library/library.service";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { settingsStore } from "$lib/stores/settings/settings.store";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import AlbumListItem from "$lib/components/library/album/AlbumListItem.svelte";
import AlbumListTrackItem from "$lib/components/library/album/AlbumListTrackItem.svelte";
import TrackTable from "$lib/components/library/track/TrackTable.svelte";
import { trierPistes, resetTagCache } from "$lib/config/trackColumns";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import ArtistListItem from "$lib/components/library/artist/ArtistListItem.svelte";
import type { ArtistListView } from "$lib/types/ui/library/artist/ArtistListView";
import { t } from "$lib/i18n";
import { toQueueTracks } from "$lib/helper/tools/queueTools";
import { queueState } from "$lib/stores/queue/queueState.store";
import { playerService } from "$lib/services/player/player.service";
import ArtistHeroSkeleton from "$lib/components/library/artist/ArtistHeroSkeleton.svelte";
import ImgZoom from "$lib/components/ui/tools/ImgZoom.svelte";
import LibraryTrackSkeleton from "$lib/components/library/common/skeleton/LibraryTrackSkeleton.svelte";
import LibraryAlbumSkeleton from "$lib/components/library/common/skeleton/LibraryAlbumSkeleton.svelte";
import LibraryArtistSkeleton from "$lib/components/library/common/skeleton/LibraryArtistSkeleton.svelte";

// --- State ---
let artist: ArtistDetailView | null = $state(null);
let artistAlbums: AlbumListView[] = $state([]);
let artistTracks: TrackListView[] = $state([]);
let similarArtists: ArtistListView[] = $state([]);
let artistImageUrl: string | null = $state(null);

// --- Loading states (per section) ---
let loadingHero = $state(true);
let loadingTracks = $state(true);
let loadingAlbums = $state(true);
let loadingSimilar = $state(true);
let error: string | null = $state(null);

// --- UI state ---
let showAllTracks = $state(false);
const TRACKS_PER_PAGE = 10;

// Tri discographie
type TrackSortField = 'album' | 'year' | 'title';
type SortDir = 'asc' | 'desc';
let tracksSort = $state<TrackSortField>('album');
let tracksSortDir = $state<SortDir>('asc');

let sortedTracks = $derived.by(() => {
  const sorted = [...artistTracks];
  sorted.sort((a, b) => {
    let cmp = 0;
    if (tracksSort === 'album') {
      const aa = a.album?.toLowerCase() ?? '';
      const ab = b.album?.toLowerCase() ?? '';
      cmp = aa.localeCompare(ab);
      if (cmp === 0) cmp = (a.disc_number ?? 1) - (b.disc_number ?? 1);
      if (cmp === 0) cmp = (a.track_number ?? 0) - (b.track_number ?? 0);
    } else if (tracksSort === 'year') {
      cmp = (Number(a.year) || 0) - (Number(b.year) || 0);
      if (cmp === 0) cmp = (a.album?.toLowerCase() ?? '').localeCompare(b.album?.toLowerCase() ?? '');
    } else {
      const ta = a.title?.toLowerCase() ?? '';
      const tb = b.title?.toLowerCase() ?? '';
      cmp = ta.localeCompare(tb);
    }
    return tracksSortDir === 'desc' ? -cmp : cmp;
  });
  return sorted;
});

let visibleTracks = $derived(
  showAllTracks ? sortedTracks : sortedTracks.slice(0, TRACKS_PER_PAGE)
);

// ─── Tri du tableau ───
//
// Toutes les pistes de l'artiste sont en mémoire : le classement se fait sur
// place et porte sur l'ensemble, pas sur la portion affichée. Il s'applique
// donc avant le découpage — trier puis tronquer donne bien les dix premières
// du classement, là que tronquer puis trier ne rangerait que la première page.
let tableSortKey = $state<string | null>(null);
let tableSortDir = $state<SortDir>('asc');

let tableTracks = $derived(
  trierPistes(sortedTracks, tableSortKey, tableSortDir)
    .slice(0, showAllTracks ? undefined : TRACKS_PER_PAGE)
);

function handleTableSort(key: string, dir: SortDir) {
  tableSortKey = key;
  tableSortDir = dir;
}

// Les tags analysés appartiennent aux pistes chargées : changer d'artiste doit
// vider ce qui a été mis en cache pour le précédent.
$effect(() => {
  const _pistes = artistTracks;
  resetTagCache();
});

// ─── L'ordre affiché, pour la sélection par plage ───
//
// Déclaré aussi en vue cartes : Maj + clic doit fonctionner dans les deux modes,
// et l'ordre n'y est pas le même.
$effect(() => {
  selectionStore.setOrder(visibleTracks.map((t) => ({ id: t.id, track: t })));
});

function toggleTracksSort(field: TrackSortField) {
  if (tracksSort === field) {
    tracksSortDir = tracksSortDir === 'desc' ? 'asc' : 'desc';
  } else {
    tracksSort = field;
    tracksSortDir = field === 'year' ? 'desc' : 'asc';
  }
}

// Tri albums
type AlbumSortField = 'year' | 'title';
let albumsSort = $state<AlbumSortField>('year');
let albumsSortDir = $state<SortDir>('desc');

function comparerAlbums(a: AlbumListView, b: AlbumListView) {
  if (albumsSort === 'year') {
    const ya = a.year ?? 0;
    const yb = b.year ?? 0;
    return albumsSortDir === 'desc' ? yb - ya : ya - yb;
  }
  const ta = a.title?.toLowerCase() ?? '';
  const tb = b.title?.toLowerCase() ?? '';
  return albumsSortDir === 'desc' ? tb.localeCompare(ta) : ta.localeCompare(tb);
}

// Ses albums d'un côté, ceux où il n'est qu'invité de l'autre.
let sortedAlbums = $derived(
  artistAlbums.filter(a => !a.participation).sort(comparerAlbums)
);
let albumsInvite = $derived(
  artistAlbums.filter(a => a.participation).sort(comparerAlbums)
);

function toggleAlbumsSort(field: AlbumSortField) {
  if (albumsSort === field) {
    albumsSortDir = albumsSortDir === 'desc' ? 'asc' : 'desc';
  } else {
    albumsSort = field;
    albumsSortDir = field === 'year' ? 'desc' : 'asc';
  }
}

let heroBgSrc = $derived.by(() => {
  if (artistImageUrl) return artistImageUrl;
  if (artistAlbums[0]?.cover_url) return artistAlbums[0].cover_url;
  return null;
});

async function playAllTracks() {
  if (artistTracks.length === 0) return;
  const queueTracks = toQueueTracks(artistTracks);
  await queueState.loadTracks(queueTracks);
  playerService.playFile(queueTracks[0]);
}

const libraryId = $derived(Number(page.params.library_id));
const artistId = $derived(String(page.params.artist_id));
const profil = $derived($profilSelector.profilSelected);

let currentTag = 0;

$effect(() => {
  const id = libraryId;
  const aId = artistId;
  const p = profil;
  error = null;

  if (!$profilSelector.initialized) return;

  if (!p) { error = "Aucun profil sélectionné"; loadingHero = false; return; }
  if (!id || !aId) { error = "Paramètres invalides"; loadingHero = false; return; }

  const tag = ++currentTag;
  loadData(id, aId, p.id, tag);
});

async function loadData(
  libId: number,
  artId: string,
  profilId: number,
  tag: number
) {
  // Reset all states
  loadingHero = true;
  loadingTracks = true;
  loadingAlbums = true;
  loadingSimilar = true;
  artist = null;
  artistAlbums = [];
  artistTracks = [];
  similarArtists = [];
  showAllTracks = false;
  error = null;

  try {
    // Phase 1: Fast — get library + artist info for the hero header
    const [lib, artistData] = await Promise.all([
      invoke<Library>('get_library', { libraryId: libId }),
      loadArtist(artId),
    ]);

    if (tag !== currentTag) return;

    if (!lib || lib.profil_id !== profilId) {
      goto("/");
      return;
    }

    // Show hero immediately
    artist = artistData;
    artistImageUrl = artistData?.thumbnail_path ?? null;
    loadingHero = false;

    // ─── Portrait de l'artiste, depuis Deezer ───
    //
    // C'est le seul appel réseau que l'application déclenche d'elle-même :
    // ouvrir une fiche artiste sans portrait interroge Deezer. Le résultat est
    // mis en cache en base — y compris l'absence de résultat — donc une fiche
    // déjà vue ne redemande rien.
    //
    // Le réglage ne touche qu'à ce déclenchement automatique : le bouton de
    // récupération groupée reste disponible, un clic étant une demande
    // explicite.
    const autoArtistImages =
      settingsStore.get('auto_download_artist_images') !== 'false';

    if (autoArtistImages && artistData?.name && artistData?.id) {
      invoke<string | null>('fetch_artist_image', {
        artistId: artistData.id,
        artistName: artistData.name
      }).then(url => {
        if (tag === currentTag && url) artistImageUrl = url;
      }).catch(() => {});
    }

    // Phase 2: Load tracks + albums in parallel
    invoke<TrackListView[]>('get_tracks_by_artist', { libraryId: libId, artistId: artId })
      .then(tracks => {
        if (tag !== currentTag) return;
        artistTracks = tracks;
      })
      .catch(e => console.error('Failed to load tracks:', e))
      .finally(() => { if (tag === currentTag) loadingTracks = false; });

    invoke<AlbumListView[]>('get_albums_by_artist', { libraryId: libId, artistId: artId })
      .then(albums => {
        if (tag !== currentTag) return;
        artistAlbums = albums;
      })
      .catch(e => console.error('Failed to load albums:', e))
      .finally(() => { if (tag === currentTag) loadingAlbums = false; });

    // Phase 3: Similar artists — loaded after main content
    invoke<ArtistListView[]>('get_similar_artists', { libraryId: libId, artistId: artId, limit: 10 })
      .then(artists => {
        if (tag !== currentTag) return;
        similarArtists = artists;
      })
      .catch(e => console.error('Failed to load similar artists:', e))
      .finally(() => { if (tag === currentTag) loadingSimilar = false; });

  } catch (e) {
    if (tag !== currentTag) return;
    error = "Impossible de charger l'artiste";
    loadingHero = false;
  }
}
</script>

{#if error}
  <div class="flex items-center justify-center h-full text-red-500">
    {error}
  </div>

{:else}

<div class="flex flex-col scrollbar-app overflow-y-auto h-full">

  <!-- HERO -->
  {#if loadingHero}
    <ArtistHeroSkeleton />
  {:else if artist}
    <div class="sticky left-0 px-8 pt-10 pb-14 shrink-0">

      <!-- BG Dual Layer (style Apple Music) -->
      {#if heroBgSrc}
        <div class="absolute inset-0 overflow-hidden pointer-events-none" aria-hidden="true" style="z-index: 0;">
          <CoverImg path={heroBgSrc} alt=""
               class="absolute inset-0 w-full h-full object-cover scale-110"
               style="filter: blur(30px) saturate(1.4); opacity: 0.5;" />
          <div class="absolute inset-0"
               style="background: radial-gradient(ellipse at 25% 50%, transparent 0%, rgba(var(--hero-overlay-rgb),0.6) 50%, rgba(var(--hero-overlay-rgb),0.95) 100%);"></div>
          <div class="absolute inset-0 dark:bg-black/30 bg-white/30"></div>
        </div>
        <div class="absolute left-0 right-0 bottom-0 h-40 pointer-events-none" aria-hidden="true"
             style="z-index: 1; background: linear-gradient(to bottom, transparent 0%, var(--hero-bg-hex) 100%);"></div>
      {/if}

      <!-- Retour -->
      <button
        type="button"
        onclick={() => goto(`/library/${libraryId}/artists`)}
        class="relative z-10 mb-6 inline-flex items-center gap-2 text-sm text-neutral-400
               hover:text-neutral-900 dark:hover:text-white transition-colors cursor-pointer"
      >
        <Icon icon="lucide:arrow-left" width={16} />
        {$t('library.back_artists')}
      </button>

      <div class="relative z-10 flex items-center gap-6">
        <!-- Photo -->
        <div class="w-44 h-44 rounded-full overflow-hidden
                    bg-neutral-200 dark:bg-neutral-700 shadow-xl flex items-center justify-center">
          {#if artistImageUrl}
            <ImgZoom path={artistImageUrl} alt={artist.name}>
              <CoverImg
                path={artistImageUrl}
                alt={artist.name}
                class="w-full h-full object-cover"
              />
            </ImgZoom>
          {:else}
            <Icon icon="lucide:user" width={64} class="text-neutral-400" />
          {/if}
        </div>

        <!-- Info -->
        <div class="flex flex-col">
          <span class="text-sm uppercase text-neutral-500 dark:text-neutral-400 tracking-wide">{$t('library.artist_label')}</span>
          <h1 class="text-4xl font-bold text-neutral-900 dark:text-white mt-1">{artist.name}</h1>
          <span class="mt-3 text-sm text-neutral-600 dark:text-neutral-300">
            {#if loadingAlbums || loadingTracks}
              <span class="inline-block h-4 w-40 bg-neutral-200 dark:bg-neutral-700 rounded animate-pulse"></span>
            {:else}
              {artistAlbums.length} album{artistAlbums.length !== 1 ? 's' : ''} · {artistTracks.length} titre{artistTracks.length !== 1 ? 's' : ''}
            {/if}
          </span>
          <span class="text-xs text-neutral-400 mt-1">
            Durée totale : {Math.round(artist.total_duration / 60)} min
          </span>

          {#if !loadingTracks && artistTracks.length > 0}
            <button
              type="button"
              class="mt-5 w-fit flex items-center gap-2 px-6 py-2 rounded-full
                     bg-green-600 text-white font-medium
                     hover:bg-green-700 transition-all duration-150 cursor-pointer"
              onclick={playAllTracks}
            >
              <Icon icon="lucide:play" width={16} />
              {$t('library.play_all')}
            </button>
          {:else if loadingTracks}
            <div class="mt-5 h-10 w-36 rounded-full bg-neutral-200 dark:bg-neutral-800 animate-pulse"></div>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- CONTENT -->
  <div class="px-8 py-10 space-y-12">

    <!-- DISCOGRAPHIE / TITRES -->
    <div>
      <div class="flex items-end justify-between mb-5">
        <div>
          <h2 class="text-xl font-semibold text-neutral-800 dark:text-neutral-200">
            {$t('library.discography')}
          </h2>
          {#if !loadingTracks}
            <p class="text-xs text-neutral-400 dark:text-neutral-500 mt-0.5">
              {artistTracks.length} titre{artistTracks.length !== 1 ? 's' : ''}
            </p>
          {/if}
        </div>

        <!-- Masquées en vue tableau : l'en-tête y trie déjà, sur bien plus
             de colonnes. Deux commandes pour la même chose, dont l'une
             couvre un sous-ensemble de l'autre, laissent surtout se
             demander laquelle fait foi. -->
        {#if !loadingTracks && artistTracks.length > 0 && $viewMode !== 'list'}
          <div class="flex items-center rounded-full bg-neutral-100 dark:bg-white/4 border border-neutral-200 dark:border-white/6 p-0.5">
            <button
              type="button"
              class="flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium cursor-pointer
                     transition-all duration-200
                     {tracksSort === 'album'
                       ? 'text-neutral-900 dark:text-white bg-white dark:bg-white/10 shadow-sm'
                       : 'text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-300'}"
              onclick={() => toggleTracksSort('album')}
            >
              Album
              {#if tracksSort === 'album'}
                <svg class="w-3 h-3 transition-transform duration-200 {tracksSortDir === 'desc' ? 'rotate-180' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="m6 9 6 6 6-6"/></svg>
              {/if}
            </button>
            <button
              type="button"
              class="flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium cursor-pointer
                     transition-all duration-200
                     {tracksSort === 'year'
                       ? 'text-neutral-900 dark:text-white bg-white dark:bg-white/10 shadow-sm'
                       : 'text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-300'}"
              onclick={() => toggleTracksSort('year')}
            >
              Année
              {#if tracksSort === 'year'}
                <svg class="w-3 h-3 transition-transform duration-200 {tracksSortDir === 'desc' ? '' : 'rotate-180'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="m6 9 6 6 6-6"/></svg>
              {/if}
            </button>
            <button
              type="button"
              class="flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium cursor-pointer
                     transition-all duration-200
                     {tracksSort === 'title'
                       ? 'text-neutral-900 dark:text-white bg-white dark:bg-white/10 shadow-sm'
                       : 'text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-300'}"
              onclick={() => toggleTracksSort('title')}
            >
              Titre
              {#if tracksSort === 'title'}
                <svg class="w-3 h-3 transition-transform duration-200 {tracksSortDir === 'desc' ? 'rotate-180' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="m6 9 6 6 6-6"/></svg>
              {/if}
            </button>
          </div>
        {/if}
      </div>

      {#if loadingTracks}
        <LibraryTrackSkeleton rows={8} />
      {:else if artistTracks.length > 0}
        {#if $viewMode === 'list'}
          <TrackTable
            {libraryId}
            tracks={tableTracks}
            sortKey={tableSortKey}
            sortDir={tableSortDir}
            onsort={handleTableSort}
          />
        {:else}
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-x-8 gap-y-1">
            {#each visibleTracks as track (track.id)}
              <AlbumListTrackItem libraryId={libraryId} {track} tracks={visibleTracks} showAlbum={true} />
            {/each}
          </div>
        {/if}

        {#if artistTracks.length > TRACKS_PER_PAGE}
          <button
            type="button"
            class="mt-4 flex items-center gap-1.5 text-xs font-medium cursor-pointer
                   text-neutral-500 dark:text-neutral-400
                   hover:text-neutral-700 dark:hover:text-neutral-200
                   transition-colors"
            onclick={() => showAllTracks = !showAllTracks}
          >
            <Icon icon={showAllTracks ? "lucide:chevron-up" : "lucide:chevron-down"} width={14} />
            {showAllTracks
              ? $t('library.show_less')
              : `${$t('library.show_more')} (${artistTracks.length - TRACKS_PER_PAGE})`}
          </button>
        {/if}
      {:else}
        <p class="text-sm text-neutral-400 dark:text-neutral-500 mt-2">{$t('library.no_track_artist')}</p>
      {/if}
    </div>

    <!-- ALBUMS -->
    <div class="sticky left-0">
      {#if loadingAlbums}
        <h2 class="text-xl font-semibold mb-1 text-neutral-800 dark:text-neutral-200">
          {$t('library.albums')}
        </h2>
        <LibraryAlbumSkeleton />
      {:else if sortedAlbums.length > 0}
        <div class="flex items-end justify-between mb-5">
          <div>
            <h2 class="text-xl font-semibold mb-1 text-neutral-800 dark:text-neutral-200">
              {$t('library.albums')}
            </h2>
            <p class="text-xs text-neutral-400 dark:text-neutral-500">
              {sortedAlbums.length} album{sortedAlbums.length !== 1 ? 's' : ''}
            </p>
          </div>

          <div class="flex items-center rounded-full bg-neutral-100 dark:bg-white/4 border border-neutral-200 dark:border-white/6 p-0.5">
            <button
              type="button"
              class="flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium cursor-pointer
                     transition-all duration-200
                     {albumsSort === 'year'
                       ? 'text-neutral-900 dark:text-white bg-white dark:bg-white/10 shadow-sm'
                       : 'text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-300'}"
              onclick={() => toggleAlbumsSort('year')}
            >
              Année
              {#if albumsSort === 'year'}
                <svg class="w-3 h-3 transition-transform duration-200 {albumsSortDir === 'asc' ? 'rotate-180' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="m6 9 6 6 6-6"/></svg>
              {/if}
            </button>
            <button
              type="button"
              class="flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium cursor-pointer
                     transition-all duration-200
                     {albumsSort === 'title'
                       ? 'text-neutral-900 dark:text-white bg-white dark:bg-white/10 shadow-sm'
                       : 'text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-300'}"
              onclick={() => toggleAlbumsSort('title')}
            >
              Titre
              {#if albumsSort === 'title'}
                <svg class="w-3 h-3 transition-transform duration-200 {albumsSortDir === 'asc' ? 'rotate-180' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="m6 9 6 6 6-6"/></svg>
              {/if}
            </button>
          </div>
        </div>

        {#if $viewMode === 'list'}
        <!-- La sous-section suit le mode d'affichage de la liste principale. -->
          <div class="flex flex-col">
            {#each sortedAlbums as album (album.id)}
              <AlbumListRow {libraryId} album={album} />
            {/each}
          </div>
        {:else}
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-6 min-[1800px]:grid-cols-7 min-[2200px]:grid-cols-8 gap-6">
          {#each sortedAlbums as album (album.id)}
            <AlbumListItem album={album} libraryId={libraryId} />
          {/each}
        </div>
        {/if}
      {:else if albumsInvite.length === 0}
        <p class="text-sm text-neutral-400 dark:text-neutral-500 mt-2">{$t('library.no_album_artist')}</p>
      {/if}
    </div>

    <!-- APPARAÎT DANS : compilations, bandes originales, featurings -->
    {#if !loadingAlbums && albumsInvite.length > 0}
      <div class="sticky left-0">
        <h2 class="text-xl font-semibold mb-1 text-neutral-800 dark:text-neutral-200">
          {$t('library.appears_on')}
        </h2>
        <p class="text-xs text-neutral-400 dark:text-neutral-500 mb-5">
          {$t('library.appears_on_desc')}
        </p>

        {#if $viewMode === 'list'}
        <!-- La sous-section suit le mode d'affichage de la liste principale. -->
          <div class="flex flex-col">
            {#each albumsInvite as album (album.id)}
              <AlbumListRow {libraryId} album={album} />
            {/each}
          </div>
        {:else}
          <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-6 min-[1800px]:grid-cols-7 min-[2200px]:grid-cols-8 gap-6">
            {#each albumsInvite as album (album.id)}
              <AlbumListItem album={album} libraryId={libraryId} />
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- ARTISTES SIMILAIRES -->
    {#if loadingSimilar}
      <div>
        <h2 class="text-xl font-semibold mb-1 text-neutral-800 dark:text-neutral-200">
          {$t('library.similar_artists')}
        </h2>
        <LibraryArtistSkeleton />
      </div>
    {:else if similarArtists.length > 0}
      <div class="sticky left-0">
        <h2 class="text-xl font-semibold mb-1 text-neutral-800 dark:text-neutral-200">
          {$t('library.similar_artists')}
        </h2>
        <p class="text-xs text-neutral-400 dark:text-neutral-500 mb-5">
          {$t('library.same_genre')}
        </p>

        {#if $viewMode === 'list'}
        <!-- La sous-section suit le mode d'affichage de la liste principale. -->
          <div class="flex flex-col">
            {#each similarArtists as similarArtist (similarArtist.id)}
              <ArtistListRow {libraryId} artist={similarArtist} />
            {/each}
          </div>
        {:else}
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-6 min-[1800px]:grid-cols-7 min-[2200px]:grid-cols-8 gap-6">
          {#each similarArtists as similarArtist (similarArtist.id)}
            <ArtistListItem libraryId={libraryId} artist={similarArtist} />
          {/each}
        </div>
        {/if}
      </div>
    {/if}

  </div>
</div>

{/if}
