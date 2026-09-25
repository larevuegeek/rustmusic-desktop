<script lang="ts">
import { page } from "$app/state";
import Icon from "@iconify/svelte";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import LibraryAlbumSkeleton from "$lib/components/library/common/skeleton/LibraryAlbumSkeleton.svelte";
import { libraryHeader } from "$lib/stores/library/libraryHeader";
import { libraryStore } from "$lib/stores/library/library.store";
import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
import AlbumListItem from "$lib/components/library/album/AlbumListItem.svelte";
import AlbumListRow from "$lib/components/library/album/AlbumListRow.svelte";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import { handleAddFiles, handleAddDirectory } from "$lib/actions/library/LibraryAction";
import { t } from "$lib/i18n";
import FilterBar from "$lib/components/library/common/FilterBar.svelte";
import AlphabetNav from "$lib/components/ui/alphabet/AlphabetNav.svelte";
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";

const libraryId = $derived(Number(page.params.library_id));
const currentLibrary = $derived(
  $libraryStore.libraries.find(l => l.id === libraryId)
);

const SORT_KEY = 'filterbar:albums';
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

const sortOptions = [
  { key: 'default', label: 'Par défaut', icon: 'lucide:list' },
  { key: 'title', label: 'Titre', icon: 'lucide:type' },
  { key: 'artist', label: 'Artiste', icon: 'lucide:mic-2' },
  { key: 'year', label: 'Année', icon: 'lucide:calendar' },
  { key: 'tracks', label: 'Nb titres', icon: 'lucide:hash' },
];

let filteredAlbums = $derived.by(() => {
  let result = [...$libraryContentStore.albums];

  // Filtre texte
  if (filterQuery.length >= 2) {
    const q = filterQuery.toLowerCase();
    result = result.filter(a =>
      a.title?.toLowerCase().includes(q) ||
      a.artist?.toLowerCase().includes(q) ||
      a.genre?.toLowerCase().includes(q)
    );
  }

  // Tri
  if (sortBy !== 'default') {
    const dir = sortDir === 'asc' ? 1 : -1;
    result.sort((a, b) => {
      switch (sortBy) {
        case 'title': return (a.title ?? '').localeCompare(b.title ?? '') * dir;
        case 'artist': return (a.artist ?? '').localeCompare(b.artist ?? '') * dir;
        case 'year': return ((a.year ?? 0) - (b.year ?? 0)) * dir;
        case 'tracks': return ((a.total_tracks ?? 0) - (b.total_tracks ?? 0)) * dir;
        default: return 0;
      }
    });
  }

  return result;
});

$effect(() => {
  const total = filteredAlbums.length;
  libraryHeader.update(current => {
    if (current.total === total && current.subtitle === 'Albums') return current;
    return { subtitle: 'Albums', icon: 'lucide:disc-album', total };
  });
});

// --- Alphabet navigation ---
let scrollContainer = $state<HTMLDivElement | null>(null);

function firstLetter(title: string | null | undefined): string {
  if (!title) return '#';
  const first = title.trim().charAt(0).toUpperCase();
  return /[A-Z]/.test(first) ? first : '#';
}

let availableLetters = $derived(
  new Set(filteredAlbums.map(a => firstLetter(a.title)))
);

function scrollToLetter(letter: string) {
  if (!scrollContainer) return;
  const el = scrollContainer.querySelector(`[data-letter="${letter}"]`) as HTMLElement | null;
  // `scrollIntoView` et non un calcul d'offset : les cartes hors écran portent
  // `content-visibility`, donc leur hauteur n'est que présumée tant qu'elles
  // n'ont pas été rendues une fois — la somme serait fausse. Le décalage vient
  // de `scroll-mt-6` sur l'ancre.
  el?.scrollIntoView({ behavior: 'smooth', block: 'start' });
}

function shouldShowLetter(index: number, album: AlbumListView): boolean {
  if (index === 0) return true;
  return firstLetter(filteredAlbums[index - 1].title) !== firstLetter(album.title);
}
</script>

{#if $libraryStore.isImporting}
  <LibraryImportingLoader />

{:else if currentLibrary?.total_albums === 0}
  <div class="flex flex-col items-center justify-center py-20 px-6 text-center">
    <div class="relative mb-5">
      <div class="absolute inset-0 rounded-2xl bg-green-500/25 blur-2xl scale-[2] animate-pulse"></div>
      <div class="relative w-16 h-16 rounded-2xl
                  bg-neutral-100 dark:bg-neutral-800
                  border border-neutral-200/60 dark:border-neutral-700/40
                  flex items-center justify-center">
        <Icon icon="lucide:disc-album" width="24" class="text-green-500/60" />
      </div>
    </div>
    <h3 class="text-base font-semibold text-neutral-700 dark:text-neutral-200 mb-1.5">
      {$t('library.no_album')}
    </h3>
    <p class="text-sm text-neutral-400 dark:text-neutral-500 max-w-xs leading-relaxed mb-6">
      {$t('library.no_album_desc')}
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

{:else if $libraryContentStore.isLoading}
  <LibraryAlbumSkeleton />

{:else}
  <div class="flex flex-col h-full">
    <FilterBar
      bind:filterQuery bind:sortBy bind:sortDir
      {sortOptions}
    />

    <div class="flex-1 relative min-h-0">
      <div class="absolute inset-0 scrollbar-app overflow-y-auto p-6" bind:this={scrollContainer}>
        {#if filteredAlbums.length === 0}
          <div class="flex flex-col items-center justify-center py-20 text-center">
            <Icon icon="lucide:search-x" width="32" class="text-neutral-300 dark:text-neutral-600 mb-3" />
            <p class="text-sm text-neutral-400">{$t('search.no_result_for').replace('{query}', filterQuery)}</p>
          </div>
        {:else if $viewMode === 'grid'}
          <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-8 gap-6">
            {#each filteredAlbums as album, i (album.id)}
              <div class="relative">
                {#if shouldShowLetter(i, album)}
                  <div class="absolute -top-3 left-0 h-0 scroll-mt-6" data-letter={firstLetter(album.title)}></div>
                {/if}
                <AlbumListItem {album} {libraryId} />
              </div>
            {/each}
          </div>
        {:else}
          <div class="flex flex-col">
            {#each filteredAlbums as album, i (album.id)}
              {#if shouldShowLetter(i, album)}
                <div class="h-0 scroll-mt-6" data-letter={firstLetter(album.title)}></div>
              {/if}
              <AlbumListRow {album} {libraryId} />
            {/each}
          </div>
        {/if}
      </div>

      {#if filteredAlbums.length > 0}
        <AlphabetNav {availableLetters} onletter={scrollToLetter} />
      {/if}
    </div>
  </div>
{/if}
