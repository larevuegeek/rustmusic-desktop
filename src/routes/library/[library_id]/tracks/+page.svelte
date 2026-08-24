<script lang="ts">
import { page } from "$app/state";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { libraryStore } from "$lib/stores/library/library.store";
import { libraryHeader } from "$lib/stores/library/libraryHeader";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
import TrackListItem from "$lib/components/library/track/TrackListItem.svelte";
import TrackListCompact from "$lib/components/library/track/TrackListCompact.svelte";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import { handleAddFiles, handleAddDirectory } from "$lib/actions/library/LibraryAction";
import { t } from "$lib/i18n";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
import FilterBar from "$lib/components/library/common/FilterBar.svelte";
import TrackTable from "$lib/components/library/track/TrackTable.svelte";
import { resetTagCache, type SortDir } from "$lib/config/trackColumns";

const libraryId = $derived(Number(page.params.library_id));
const currentLibrary = $derived(
  $libraryStore.libraries.find(l => l.id === libraryId)
);

// Infinite scroll
const BATCH_SIZE = 100;
let tracks: TrackListView[] = $state([]);
let totalTracks = $state(0);
let isLoading = $state(true);
let isLoadingMore = $state(false);
let hasMore = $derived(tracks.length < totalTracks);

// Filtre & tri (tri mémorisé dans localStorage)
const SORT_KEY = 'filterbar:tracks';
const savedSort = (() => {
  try { return JSON.parse(localStorage.getItem(SORT_KEY) || '{}'); }
  catch { return {}; }
})();

let filterQuery = $state('');
let sortBy = $state<string>(savedSort.sortBy ?? 'default');
let sortDir = $state<string>(savedSort.sortDir ?? 'asc');

$effect(() => {
  try { localStorage.setItem(SORT_KEY, JSON.stringify({ sortBy, sortDir })); }
  catch {}
});

/**
 * Tri demandé depuis un en-tête de colonne.
 *
 * On écrit dans le même état que la barre de filtres : les deux commandes
 * disent la même chose, et l'une doit refléter ce que l'autre a fait. Le
 * rechargement suit, puisque le classement est l'affaire de la base.
 */
function handleHeaderSort(key: string, dir: SortDir) {
  sortBy = key;
  sortDir = dir;
  fetchTracks(true);
  scrollEl?.scrollTo({ top: 0 });
}

let scrollEl = $state<HTMLDivElement | null>(null);

const sortOptions = [
  { key: 'default', label: 'Par défaut', icon: 'lucide:list' },
  { key: 'title', label: 'Titre', icon: 'lucide:type' },
  { key: 'artist', label: 'Artiste', icon: 'lucide:mic-2' },
  { key: 'album', label: 'Album', icon: 'lucide:disc-album' },
  { key: 'duration', label: 'Durée', icon: 'lucide:clock' },
  { key: 'rating', label: 'Notation', icon: 'lucide:star' },
  { key: 'date', label: 'Date d\'ajout', icon: 'lucide:calendar' },
];

$effect(() => {
  const _libId = libraryId;
  tracks = [];
  fetchTracks(true);
});

$effect(() => {
  const _missing = $libraryContentStore.missingAlbumCover;
  fetchTracks(true);
});

async function fetchTracks(reset = false) {
  if (reset) {
    isLoading = true;
    tracks = [];
  } else {
    isLoadingMore = true;
  }

  try {
    const result = await invoke<{ tracks: TrackListView[]; total: number }>('get_tracks_paginated', {
      libraryId,
      offset: reset ? 0 : tracks.length,
      limit: BATCH_SIZE,
      sortBy: sortBy === 'default' ? null : sortBy,
      sortDir,
      filter: filterQuery.length >= 2 ? filterQuery : null,
      missingCover: $libraryContentStore.missingAlbumCover,
    });

    if (reset) resetTagCache();
    tracks = reset ? result.tracks : [...tracks, ...result.tracks];
    totalTracks = result.total;
  } catch (e) {
    console.error('Failed to load tracks:', e);
  } finally {
    isLoading = false;
    isLoadingMore = false;
  }
}

$effect(() => {
  libraryHeader.update(current => {
    if (current.total === totalTracks && current.subtitle === 'Morceaux') return current;
    return { subtitle: 'Morceaux', icon: 'lucide:music', total: totalTracks };
  });
});

function handleScroll(e: Event) {
  const el = e.target as HTMLDivElement;
  if (isLoadingMore || !hasMore) return;
  if (el.scrollHeight - el.scrollTop - el.clientHeight < 300) {
    fetchTracks(false);
  }
}

function handleFilterChange() {
  fetchTracks(true);
}
</script>

{#if $libraryStore.isImporting}
  <LibraryImportingLoader />

{:else if currentLibrary?.total_tracks === 0}
  <div class="flex flex-col items-center justify-center py-20 px-6 text-center">
    <div class="relative mb-5">
      <div class="absolute inset-0 rounded-2xl bg-green-500/25 blur-2xl scale-[2] animate-pulse"></div>
      <div class="relative w-16 h-16 rounded-2xl
                  bg-neutral-100 dark:bg-neutral-800
                  border border-neutral-200/60 dark:border-neutral-700/40
                  flex items-center justify-center">
        <Icon icon="lucide:music" width="24" class="text-green-500/60" />
      </div>
    </div>
    <h3 class="text-base font-semibold text-neutral-700 dark:text-neutral-200 mb-1.5">
      {$t('library.empty_title')}
    </h3>
    <p class="text-sm text-neutral-400 dark:text-neutral-500 max-w-xs leading-relaxed mb-6">
      {$t('library.empty_desc')}
    </p>
    <div class="flex items-center gap-2">
      <button
        class="flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-semibold cursor-pointer
               bg-green-500 text-black hover:bg-green-400
               active:scale-[0.97] transition-all duration-150"
        onclick={() => libraryId && handleAddFiles(libraryId)}
      >
        <Icon icon="lucide:file-audio" width="14" />
        {$t('library.import_files')}
      </button>
      <button
        class="flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium cursor-pointer
               border border-neutral-200/80 dark:border-neutral-700/60
               text-neutral-600 dark:text-neutral-300
               hover:bg-neutral-100/80 dark:hover:bg-neutral-800/60
               active:scale-[0.97] transition-all duration-150"
        onclick={() => libraryId && handleAddDirectory(libraryId)}
      >
        <Icon icon="lucide:folder-plus" width="14" />
        {$t('library.import_folder')}
      </button>
    </div>
  </div>

{:else}
  <div class="flex flex-col h-full">
    <FilterBar
      bind:filterQuery bind:sortBy bind:sortDir
      {sortOptions}
      onchange={handleFilterChange}
      {tracks}
    />

    <!-- Liste avec infinite scroll -->
    <div class="flex-1 relative min-h-0">
      <div
        class="absolute inset-0 overflow-y-auto scrollbar-app px-3"
        bind:this={scrollEl}
        onscroll={handleScroll}
      >
        {#if isLoading}
          <div class="flex items-center justify-center py-20">
            <Icon icon="lucide:loader-2" width="24" class="animate-spin text-neutral-400" />
          </div>
        {:else if tracks.length === 0}
          <div class="flex flex-col items-center justify-center py-20 text-center">
            <Icon icon="lucide:search-x" width="32" class="text-neutral-300 dark:text-neutral-600 mb-3" />
            <p class="text-sm text-neutral-400">{$t('search.no_result_for').replace('{query}', filterQuery)}</p>
          </div>
        {:else}
          {#if $viewMode === 'list'}
            <!-- Le tri part en base, pas en mémoire : ne ranger que les cent
                 lignes déjà chargées donnerait un résultat faux à l'air juste. -->
            <TrackTable
              {libraryId}
              {tracks}
              sortKey={sortBy === 'default' ? null : sortBy}
              sortDir={sortDir as SortDir}
              onsort={handleHeaderSort}
            />
          {:else}
            {#each tracks as track (track.id)}
              <TrackListItem {libraryId} {track} />
            {/each}
          {/if}

          {#if isLoadingMore}
            <div class="flex items-center justify-center py-4">
              <Icon icon="lucide:loader-2" width="18" class="animate-spin text-neutral-400" />
            </div>
          {/if}

          <div class="text-center py-3 text-[10px] text-neutral-400 tabular-nums">
            {tracks.length} / {totalTracks} morceaux
          </div>
        {/if}
      </div>

    </div>
  </div>
{/if}
