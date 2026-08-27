<script lang="ts">
import { page } from "$app/state";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { libraryHeader } from "$lib/stores/library/libraryHeader";
import AlbumListItem from "$lib/components/library/album/AlbumListItem.svelte";
import TrackTable from "$lib/components/library/track/TrackTable.svelte";
import { trierPistes, resetTagCache } from "$lib/config/trackColumns";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import { t } from "$lib/i18n";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
import { toQueueTracks } from "$lib/helper/tools/queueTools";
import { queueState } from "$lib/stores/queue/queueState.store";
import { playerService } from "$lib/services/player/player.service";

const libraryId = $derived(Number(page.params.library_id));
const genreName = $derived(decodeURIComponent(String((page.params as Record<string, string>).genre ?? '')));

// Filtrer albums par genre (données déjà en mémoire)
let genreAlbums = $derived(
  $libraryContentStore.albums.filter(a =>
    a.genre?.toLowerCase() === genreName.toLowerCase()
  )
);

let genreTotalTracks = $derived(
  genreAlbums.reduce((sum, a) => sum + (a.total_tracks ?? 0), 0)
);

// Couleur déterministe
function genreColor(name: string): string {
  let hash = 0;
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
  const colors = [
    '#ef4444', '#f97316', '#eab308', '#22c55e', '#14b8a6',
    '#06b6d4', '#3b82f6', '#6366f1', '#8b5cf6', '#a855f7',
    '#ec4899', '#f43f5e', '#10b981', '#0ea5e9', '#d946ef',
  ];
  return colors[Math.abs(hash) % colors.length];
}

const color = $derived(genreColor(genreName));

$effect(() => {
  libraryHeader.update(() => ({
    subtitle: genreName,
    icon: 'lucide:tag',
    total: genreAlbums.length
  }));
});

async function playAll() {
  const allTracks = await invoke<TrackListView[]>('get_tracks_by_genre', { libraryId, genre: genreName });
  if (!allTracks || allTracks.length === 0) return;
  const queueTracks = toQueueTracks(allTracks);
  await queueState.loadTracks(queueTracks);
  playerService.playFile(queueTracks[0]);
}

// ─── Les morceaux du genre ───
//
// Cette page ne montrait que des albums. Les pistes ne sont donc chargées que
// lorsqu'on demande la vue tableau : les tirer à chaque visite ferait payer une
// requête sur toute la bibliothèque à qui vient seulement parcourir les
// pochettes.
type SortDir = 'asc' | 'desc';

let genreTracks = $state<TrackListView[]>([]);
let loadingTracks = $state(false);
let loadedFor = $state<string | null>(null);

let tableSortKey = $state<string | null>(null);
let tableSortDir = $state<SortDir>('asc');

let tableTracks = $derived(trierPistes(genreTracks, tableSortKey, tableSortDir));

function handleTableSort(key: string, dir: SortDir) {
  tableSortKey = key;
  tableSortDir = dir;
}

$effect(() => {
  const cible = `${libraryId}:${genreName}`;
  if ($viewMode !== 'list' || !genreName || loadedFor === cible) return;
  loadedFor = cible;
  loadTracks();
});

async function loadTracks() {
  loadingTracks = true;
  try {
    resetTagCache();
    genreTracks = await invoke<TrackListView[]>('get_tracks_by_genre', { libraryId, genre: genreName });
  } catch (e) {
    console.error('Failed to load genre tracks:', e);
    genreTracks = [];
  } finally {
    loadingTracks = false;
  }
}
</script>

<div class="flex flex-col scrollbar-app overflow-y-auto h-full">

  <!-- HERO -->
  <div class="relative shrink-0 overflow-hidden">
    <!-- Mosaïque de covers floutée en fond -->
    {#if genreAlbums.length > 0}
      <div class="absolute inset-0 pointer-events-none" aria-hidden="true" style="z-index: 0;">
        <!-- Dark: covers floutées visibles / Light: quasi invisibles -->
        <div class="absolute inset-0 grid grid-cols-3 opacity-10 dark:opacity-30 scale-110">
          {#each genreAlbums.slice(0, 3) as album}
            {#if album.cover_url}
              <CoverImg path={album.cover_url} alt=""
                   class="w-full h-full object-cover" />
            {/if}
          {/each}
        </div>
        <div class="absolute inset-0 backdrop-blur-2xl"></div>
        <!-- Dark: assombrit / Light: teinte subtile avec la couleur du genre -->
        <div class="absolute inset-0 dark:bg-black/40"
             style="background: linear-gradient(135deg, {color}08, {color}15);"></div>
      </div>
      <!-- Fondu bas vers le fond de page -->
      <div class="absolute left-0 right-0 bottom-0 h-24 pointer-events-none" aria-hidden="true"
           style="z-index: 1; background: linear-gradient(to bottom, transparent 0%, var(--hero-bg-hex) 100%);"></div>
    {:else}
      <div class="absolute inset-0"
           style="background: linear-gradient(135deg, {color}15, transparent);"></div>
    {/if}

    <div class="relative z-10 px-8 pt-6 pb-10">
      <!-- Retour -->
      <button
        type="button"
        onclick={() => goto(`/library/${libraryId}/genres`)}
        class="mb-5 inline-flex items-center gap-2 text-xs text-neutral-400
               hover:text-neutral-900 dark:hover:text-white transition-colors cursor-pointer"
      >
        <Icon icon="lucide:arrow-left" width={14} />
        {$t('library.back_genres')}
      </button>

      <div class="flex items-end gap-6">
        <!-- Icône genre -->
        <div class="w-24 h-24 rounded-2xl flex items-center justify-center shrink-0
                    backdrop-blur-md border border-neutral-200/40 dark:border-white/10"
             style="background: {color}20;">
          <Icon icon="lucide:tag" width={36} class="text-neutral-600 dark:text-white/80" />
        </div>

        <div class="flex-1 min-w-0">
          <span class="text-[10px] uppercase tracking-[0.2em] font-medium text-neutral-500 dark:text-white/40">Genre</span>
          <h1 class="text-3xl font-bold text-neutral-900 dark:text-white mt-1 truncate">{genreName}</h1>
          <div class="flex items-center gap-4 mt-3">
            <span class="text-sm text-neutral-600 dark:text-white/50">
              {genreAlbums.length} album{genreAlbums.length !== 1 ? 's' : ''}
              · {genreTotalTracks} titre{genreTotalTracks !== 1 ? 's' : ''}
            </span>

            {#if genreAlbums.length > 0}
              <button
                type="button"
                class="flex items-center gap-2 px-5 py-2 rounded-full
                       bg-green-500 text-black text-sm font-semibold
                       hover:bg-green-400
                       active:scale-[0.97] transition-all duration-150 cursor-pointer
                       shadow-lg shadow-green-500/20"
                onclick={playAll}
              >
                <Icon icon="lucide:play" width={14} />
                {$t('library.play_all')}
              </button>
            {/if}
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- ALBUMS ou MORCEAUX, selon le mode d'affichage choisi -->
  <div class="px-8 py-8">
    {#if $viewMode === 'list'}
      <h2 class="text-lg font-semibold text-neutral-800 dark:text-neutral-200 mb-4">
        {$t('library.tracks')}
      </h2>
      {#if loadingTracks}
        <div class="flex items-center justify-center py-16">
          <Icon icon="lucide:loader-2" width="22" class="animate-spin text-neutral-400" />
        </div>
      {:else if genreTracks.length === 0}
        <p class="text-sm text-neutral-400 text-center py-10">{$t('library.no_album_genre')}</p>
      {:else}
        <TrackTable
          {libraryId}
          tracks={tableTracks}
          sortKey={tableSortKey}
          sortDir={tableSortDir}
          onsort={handleTableSort}
        />
      {/if}
    {:else if genreAlbums.length === 0}
      <p class="text-sm text-neutral-400 text-center py-10">{$t('library.no_album_genre')}</p>
    {:else}
      <h2 class="text-lg font-semibold text-neutral-800 dark:text-neutral-200 mb-4">{$t('library.albums')}</h2>
      <!-- Pas de variante en liste ici : sur cette page, le mode liste montre
           les morceaux et non les albums. Cette grille ne s'affiche donc qu'en
           mode grille, et lui proposer une seconde forme reviendrait à écrire
           une branche que rien n'atteint. -->
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-6 min-[1800px]:grid-cols-7 min-[2200px]:grid-cols-8 gap-6">
        {#each genreAlbums as album (album.id)}
          <AlbumListItem {album} {libraryId} />
        {/each}
      </div>
    {/if}
  </div>
</div>
