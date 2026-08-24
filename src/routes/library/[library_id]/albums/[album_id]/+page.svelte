<script lang="ts">
import { page } from "$app/state";
import { goto } from "$app/navigation";
import { profilSelector } from "$lib/stores/profil/profil.store";
import type { Library } from "$lib/types/db/library/Library";
import type { AlbumDetailView } from "$lib/types/ui/library/album/AlbumDetailView";
import { loadAlbum, loadTracksByAlbum } from "$lib/services/library/library.service";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { handleAlbumEnqueue } from "$lib/actions/queue/QueueAction";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";
import AlbumListTrackItem from "$lib/components/library/album/AlbumListTrackItem.svelte";
import TrackTable from "$lib/components/library/track/TrackTable.svelte";
import { trierPistes, resetTagCache } from "$lib/config/trackColumns";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import AlbumListItem from "$lib/components/library/album/AlbumListItem.svelte";
import ArtistListItem from "$lib/components/library/artist/ArtistListItem.svelte";
import { loadAlbums, loadArtists } from "$lib/services/library/library.service";
import type { ArtistListView } from "$lib/types/ui/library/artist/ArtistListView";
import { t } from "$lib/i18n";
import { dataCache } from "$lib/stores/cache/dataCache.store";
import CollectionContextMenu from "$lib/components/ui/contextmenu/CollectionContextMenu.svelte";
import ImgZoom from "$lib/components/ui/tools/ImgZoom.svelte";

// ==========================
// STATE
// ==========================

let library: Library | null = $state(null);
let album: AlbumDetailView | null = $state(null);
let tracks: TrackListView[] = $state([]);
let otherAlbums: AlbumListView[] = $state([]);
let sameGenreAlbums: AlbumListView[] = $state([]);
let sameGenreArtists: ArtistListView[] = $state([]);

let isLoading = $state(true);
let albumContextMenu = $state<{ x: number; y: number } | null>(null);

async function loadAlbumTracksForMenu() {
  if (!album) return [];
  const result = await loadTracksByAlbum(libraryId, album.id);
  return result ?? [];
}
let error: string | null = $state(null);

// Tri des tracks de l'album
type TrackSortField = 'default' | 'title' | 'duration';
let trackSort = $state<TrackSortField>('default');
let trackSortDir = $state<SortDir>('asc');

let sortedTracks = $derived.by(() => {
  if (trackSort === 'default') return tracks;
  const sorted = [...tracks];
  sorted.sort((a, b) => {
    let cmp = 0;
    if (trackSort === 'title') {
      const ta = a.title?.toLowerCase() ?? '';
      const tb = b.title?.toLowerCase() ?? '';
      cmp = ta.localeCompare(tb);
    } else {
      cmp = (a.duration ?? 0) - (b.duration ?? 0);
    }
    return trackSortDir === 'desc' ? -cmp : cmp;
  });
  return sorted;
});

// ─── Tri du tableau ───
//
// Toutes les pistes de l'album sont en mémoire : le classement se fait donc
// sur place, sans aller-retour, et il est exact — contrairement à l'onglet
// Morceaux où seule une page est chargée et où la base doit trancher.
//
// Il s'applique par-dessus `sortedTracks` : sans colonne triée, l'ordre du
// disque est conservé, et c'est bien celui qu'on veut par défaut sur un album.
let tableSortKey = $state<string | null>(null);
let tableSortDir = $state<SortDir>('asc');

let tableTracks = $derived(trierPistes(sortedTracks, tableSortKey, tableSortDir));

function handleTableSort(key: string, dir: SortDir) {
  tableSortKey = key;
  tableSortDir = dir;
}

// Les tags analysés appartiennent aux pistes chargées : changer d'album doit
// vider ce qui a été mis en cache pour le précédent.
$effect(() => {
  const _pistes = tracks;
  resetTagCache();
});

function toggleTrackSort(field: TrackSortField) {
  if (field === 'default') {
    trackSort = 'default';
    trackSortDir = 'asc';
  } else if (trackSort === field) {
    trackSortDir = trackSortDir === 'desc' ? 'asc' : 'desc';
  } else {
    trackSort = field;
    trackSortDir = 'asc';
  }
}

// Tri des "autres albums"
type SortField = 'year' | 'title';
type SortDir = 'asc' | 'desc';
let otherAlbumsSort = $state<SortField>('year');
let otherAlbumsSortDir = $state<SortDir>('desc');

let sortedOtherAlbums = $derived.by(() => {
  const sorted = [...otherAlbums];
  sorted.sort((a, b) => {
    if (otherAlbumsSort === 'year') {
      const ya = a.year ?? 0;
      const yb = b.year ?? 0;
      return otherAlbumsSortDir === 'desc' ? yb - ya : ya - yb;
    }
    const ta = a.title?.toLowerCase() ?? '';
    const tb = b.title?.toLowerCase() ?? '';
    return otherAlbumsSortDir === 'desc' ? tb.localeCompare(ta) : ta.localeCompare(tb);
  });
  return sorted;
});

function toggleOtherAlbumsSort(field: SortField) {
  if (otherAlbumsSort === field) {
    otherAlbumsSortDir = otherAlbumsSortDir === 'desc' ? 'asc' : 'desc';
  } else {
    otherAlbumsSort = field;
    otherAlbumsSortDir = field === 'year' ? 'desc' : 'asc';
  }
}

// ==========================
// DERIVED
// ==========================

const libraryId = $derived(Number(page.params.library_id));
const libraryAlbumId = $derived(page.params.album_id as string);
const profil = $derived($profilSelector.profilSelected);


let currentTag = 0;

// ==========================
// EFFECT
// ==========================

$effect(() => {
  const id = libraryId;
  const albumId = libraryAlbumId;
  const p = profil;
  error = null;

  if (!$profilSelector.initialized) {
      return; // on attend
  }

  if (!p || !id || !albumId) {
    error = "Paramètres invalides";
    isLoading = false;
    return;
  }

  const tag = ++currentTag;
  loadAlbumPage(id, albumId, p.id, tag);
  loadAlbumTracks(id, albumId);
});

// ==========================
// LOAD
// ==========================

async function loadAlbumPage(
  libId: number,
  albumId: string,
  profilId: number,
  tag: number
) {
  isLoading = true;
  error = null;

  try {

    const lib = await invoke<Library>('get_library', { 
        libraryId: libId,
    });

    if (tag !== currentTag) return;

    if (!lib || lib.profil_id !== profilId) {
      goto("/");
      return;
    }

    library = lib;

    // Essayer le cache d'abord (rempli par le preload au survol)
    const cachedAlbum = dataCache.get<AlbumDetailView>(`album:${albumId}`);
    const result = cachedAlbum ? cachedAlbum.data : await loadAlbum(albumId);

    // Stocker en cache si pas déjà fait
    if (!cachedAlbum && result) dataCache.set(`album:${albumId}`, result);

    if (tag !== currentTag) return;

    album = result;

    // Charger tous les albums + artistes de la bibliothèque
    const [allAlbums, allArtists] = await Promise.all([
      loadAlbums(libId, false),
      loadArtists(libId),
    ]);

    // Autres albums du même artiste (exclure l'album en cours)
    if (result?.artist_id) {
      otherAlbums = allAlbums.filter(a => a.artist_id === result.artist_id && a.id !== albumId);
    }

    // Albums du même genre (exclure l'album en cours et ceux du même artiste)
    if (result?.genre) {
      const genre = result.genre.toLowerCase();
      sameGenreAlbums = allAlbums
        .filter(a => a.id !== albumId && a.artist_id !== result.artist_id && a.genre?.toLowerCase() === genre)
        .slice(0, 10);
    }

    // Artistes du même genre (exclure l'artiste de l'album)
    if (result?.genre) {
      const genre = result.genre.toLowerCase();
      // On récupère les artist_id des albums du même genre
      const genreArtistIds = new Set(
        allAlbums.filter(a => a.genre?.toLowerCase() === genre && a.artist_id !== result.artist_id)
          .map(a => a.artist_id)
          .filter(Boolean)
      );
      sameGenreArtists = allArtists
        .filter(a => genreArtistIds.has(a.id))
        .slice(0, 10);
    }

  } catch (e) {
    if (tag !== currentTag) return;
    error = "Impossible de charger l’album";
  } finally {
    if (tag !== currentTag) return;
    isLoading = false;
  }
}

async function loadAlbumTracks(
  libId: number,
  albumId: string,
) {
    try {
      // Cache d'abord (rempli par preload au survol)
      const cached = dataCache.get<TrackListView[]>(`album-tracks:${albumId}`);
      const result = cached ? cached.data : await loadTracksByAlbum(libId, albumId);

      if (!cached && result) dataCache.set(`album-tracks:${albumId}`, result);

      if (!result || result.length === 0) return;
      tracks = result;
    }
    catch(e) {
      error = "Impossible de charger les morceaux de l'album";
      console.log(e);
    }
}

</script>

{#if isLoading}
<div class="flex items-center justify-center h-full text-neutral-400">
  Chargement de l’album…
</div>

{:else if error}
<div class="flex items-center justify-center h-full text-red-500">
  {error}
</div>

{:else if library && album}
<div class="flex flex-col scrollbar-app overflow-y-auto h-full">

  <!-- ================= HEADER ================= -->
  <div class="relative px-8 pt-10 pb-8 shrink-0">

    <!-- BG Dual Layer (style Apple Music) -->
    {#if album.cover_url}
      <div class="absolute inset-0 overflow-hidden pointer-events-none" aria-hidden="true" style="z-index: 0;">
        <!-- Layer 1 : image floutée légèrement — on devine les formes -->
        <CoverImg
          path={album.cover_url}
          alt=""
          size="2x"
          class="absolute inset-0 w-full h-full object-cover scale-110"
          style="filter: blur(30px) saturate(1.4); opacity: 0.5;"
        />
        <!-- Layer 2 : gradient radial — assombrit/éclaircit les bords selon le thème -->
        <div class="absolute inset-0"
             style="background: radial-gradient(ellipse at 30% 50%, transparent 0%, rgba(var(--hero-overlay-rgb),0.6) 50%, rgba(var(--hero-overlay-rgb),0.95) 100%);"></div>
        <!-- Layer 3 : ajustement de lisibilité -->
        <div class="absolute inset-0 dark:bg-black/30 bg-white/30"></div>
      </div>
      <!-- Fondu bas vers le fond de page -->
      <div class="absolute left-0 right-0 bottom-0 h-40 pointer-events-none" aria-hidden="true"
           style="z-index: 1; background: linear-gradient(to bottom, transparent 0%, var(--hero-bg-hex) 100%);"></div>
    {/if}

    <!-- RETOUR -->
    <button
      type="button"
      onclick={() => goto(`/library/${libraryId}/albums`)}
      class="relative z-10 mb-6 inline-flex items-center gap-2 text-sm text-neutral-400
             hover:text-neutral-900 dark:hover:text-white transition-colors"
    >
      <Icon icon="lucide:arrow-left" width={16} />
      {$t('library.back_albums')}
    </button>

    <div class="relative z-10 flex gap-8 items-start">

      <!-- COVER -->
      <div class="w-56 h-56 rounded-2xl overflow-hidden shadow-xl bg-neutral-300 dark:bg-neutral-700 flex items-center justify-center">

        {#if album.cover_url}
          <ImgZoom path={album.cover_url} alt={album.title}>
            <CoverImg
              path={album.cover_url}
              alt={album.title}
              class="w-full h-full object-cover"
            />
          </ImgZoom>
        {:else}
          <Icon icon="lucide:disc-album" width={64} class="text-neutral-500" />
        {/if}

      </div>

      <!-- INFOS -->
      <div class="flex flex-col gap-3">

        <span class="uppercase text-xs tracking-widest text-neutral-500 dark:text-neutral-400">
          {album.album_type}
        </span>

        <h1 class="text-4xl font-bold text-neutral-900 dark:text-white leading-tight">
          {album.title}
        </h1>

        <div class="flex items-center gap-2 text-sm text-neutral-600 dark:text-neutral-400">

          <Icon icon="lucide:user" width={14} />
          <a href={`/library/${libraryId}/artists/${album.artist_id}`} class="cursor-pointer">
            <span class="font-medium text-neutral-800 dark:text-neutral-200">
              {album.artist ?? "Unknown Artist"}
            </span>
          </a>

          {#if album.year}
            • {album.year}
          {/if}

          • {album.total_tracks} titres

          • {Math.floor(album.total_duration / 60)} min
        </div>

        {#if album?.id}
        <div class="flex items-center gap-3 mt-5">
            <button
              type="button"
              onclick={() => { album && handleAlbumEnqueue(album.id)}}
              class="w-fit flex items-center gap-2 px-6 py-2 rounded-full
                    bg-green-600 text-white font-medium
                    hover:bg-green-700 transition-all duration-150 cursor-pointer"
            >
              <Icon icon="lucide:play" width={16} />
              {$t('library.play_album')}
            </button>

            <button
              type="button"
              class="w-9 h-9 rounded-full flex items-center justify-center cursor-pointer
                     bg-white/5 border border-white/10
                     text-neutral-400 hover:text-white hover:bg-white/10
                     transition-all duration-150"
              onclick={(e) => { albumContextMenu = { x: e.clientX, y: e.clientY }; }}
              aria-label="Actions"
            >
              <Icon icon="lucide:ellipsis" width={16} />
            </button>
        </div>
        {/if}
      </div>
    </div>
  </div>


  <!-- tracks -->
  <div class="px-5">
    {#if tracks.length > 1}
      <div class="flex justify-end mb-2">
        <div class="flex items-center rounded-full bg-white/4 border border-white/6 p-0.5">
          <button
            type="button"
            class="flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium cursor-pointer
                   transition-all duration-200
                   {trackSort === 'default'
                     ? 'text-white bg-white/10 shadow-sm'
                     : 'text-neutral-500 hover:text-neutral-300'}"
            onclick={() => toggleTrackSort('default')}
          >
            N°
          </button>
          <button
            type="button"
            class="flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium cursor-pointer
                   transition-all duration-200
                   {trackSort === 'title'
                     ? 'text-white bg-white/10 shadow-sm'
                     : 'text-neutral-500 hover:text-neutral-300'}"
            onclick={() => toggleTrackSort('title')}
          >
            Titre
            {#if trackSort === 'title'}
              <svg class="w-3 h-3 transition-transform duration-200 {trackSortDir === 'desc' ? 'rotate-180' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="m6 9 6 6 6-6"/></svg>
            {/if}
          </button>
          <button
            type="button"
            class="flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium cursor-pointer
                   transition-all duration-200
                   {trackSort === 'duration'
                     ? 'text-white bg-white/10 shadow-sm'
                     : 'text-neutral-500 hover:text-neutral-300'}"
            onclick={() => toggleTrackSort('duration')}
          >
            Durée
            {#if trackSort === 'duration'}
              <svg class="w-3 h-3 transition-transform duration-200 {trackSortDir === 'desc' ? 'rotate-180' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="m6 9 6 6 6-6"/></svg>
            {/if}
          </button>
        </div>
      </div>
    {/if}

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
        {#each sortedTracks as track }
          <AlbumListTrackItem libraryId={libraryId} track={track} />
        {/each}
      </div>
    {/if}
  </div>

  <!-- ================= CONTENT ================= -->
  <div class="flex-1 px-8 py-10 space-y-12">

    <!-- DESCRIPTION -->
    {#if album.notes}
      <div>
        <h2 class="text-lg font-semibold mb-2 text-neutral-800 dark:text-neutral-200">
          À propos
        </h2>
        <p class="text-sm text-neutral-600 dark:text-neutral-400 max-w-3xl leading-relaxed">
          {album.notes}
        </p>
      </div>
    {/if}

    <!-- AUTRES ALBUMS DE L'ARTISTE -->
    {#if otherAlbums.length > 0}
      <div>
        <div class="flex items-end justify-between mb-5">
          <div>
            <h2 class="text-lg font-semibold mb-1 text-neutral-800 dark:text-neutral-200">
              {$t('library.other_albums')} {album.artist ?? "cet artiste"}
            </h2>
            <p class="text-xs text-neutral-400 dark:text-neutral-500">
              {otherAlbums.length} album{otherAlbums.length !== 1 ? 's' : ''}
            </p>
          </div>

          <div class="flex items-center rounded-full bg-white/4 border border-white/6 p-0.5">
            <button
              type="button"
              class="flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium cursor-pointer
                     transition-all duration-200
                     {otherAlbumsSort === 'year'
                       ? 'text-white bg-white/10 shadow-sm'
                       : 'text-neutral-500 hover:text-neutral-300'}"
              onclick={() => toggleOtherAlbumsSort('year')}
            >
              Année
              {#if otherAlbumsSort === 'year'}
                <svg class="w-3 h-3 transition-transform duration-200 {otherAlbumsSortDir === 'asc' ? 'rotate-180' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="m6 9 6 6 6-6"/></svg>
              {/if}
            </button>
            <button
              type="button"
              class="flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium cursor-pointer
                     transition-all duration-200
                     {otherAlbumsSort === 'title'
                       ? 'text-white bg-white/10 shadow-sm'
                       : 'text-neutral-500 hover:text-neutral-300'}"
              onclick={() => toggleOtherAlbumsSort('title')}
            >
              Titre
              {#if otherAlbumsSort === 'title'}
                <svg class="w-3 h-3 transition-transform duration-200 {otherAlbumsSortDir === 'asc' ? 'rotate-180' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="m6 9 6 6 6-6"/></svg>
              {/if}
            </button>
          </div>
        </div>

        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-6 min-[1800px]:grid-cols-7 min-[2200px]:grid-cols-8 gap-6">
          {#each sortedOtherAlbums as other (other.id)}
            <AlbumListItem album={other} libraryId={libraryId} />
          {/each}
        </div>
      </div>
    {/if}

    <!-- ALBUMS DU MÊME GENRE -->
    {#if sameGenreAlbums.length > 0}
      <div>
        <h2 class="text-lg font-semibold mb-1 text-neutral-800 dark:text-neutral-200">
          {$t('library.similar_albums')}
        </h2>
        <p class="text-xs text-neutral-400 dark:text-neutral-500 mb-5">
          {$t('library.same_genre')} · {album.genre}
        </p>

        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-6 min-[1800px]:grid-cols-7 min-[2200px]:grid-cols-8 gap-6">
          {#each sameGenreAlbums as other (other.id)}
            <AlbumListItem album={other} libraryId={libraryId} />
          {/each}
        </div>
      </div>
    {/if}

    <!-- ARTISTES DU MÊME GENRE -->
    {#if sameGenreArtists.length > 0}
      <div>
        <h2 class="text-lg font-semibold mb-1 text-neutral-800 dark:text-neutral-200">
          {$t('library.similar_artists')}
        </h2>
        <p class="text-xs text-neutral-400 dark:text-neutral-500 mb-5">
          {$t('library.same_genre')} · {album.genre}
        </p>

        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-6 min-[1800px]:grid-cols-7 min-[2200px]:grid-cols-8 gap-6">
          {#each sameGenreArtists as artist (artist.id)}
            <ArtistListItem libraryId={libraryId} artist={artist} />
          {/each}
        </div>
      </div>
    {/if}

  </div>

</div>

{#if albumContextMenu && album}
  <CollectionContextMenu
    title={album.title}
    type="album"
    loadTracks={loadAlbumTracksForMenu}
    x={albumContextMenu.x}
    y={albumContextMenu.y}
    onclose={() => albumContextMenu = null}
    onedittags={() => goto(`/library/${libraryId}/tags?album=${album?.id}`)}
    oncover={() => {
        if (album) {
            loadAlbum(album.id).then(result => { if (result) album = result; });
        }
    }}
    albumId={album.id}
    artistName={album.artist}
  />
{/if}
{/if}
