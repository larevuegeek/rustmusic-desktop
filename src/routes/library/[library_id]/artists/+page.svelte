<script lang="ts">
import { page } from "$app/state";
import Icon from "@iconify/svelte";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import LibraryArtistSkeleton from "$lib/components/library/common/skeleton/LibraryArtistSkeleton.svelte";
import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
import { libraryHeader } from "$lib/stores/library/libraryHeader";
import { handleAddFiles, handleAddDirectory } from "$lib/actions/library/LibraryAction";
import { libraryStore } from "$lib/stores/library/library.store";
import ArtistListItem from "$lib/components/library/artist/ArtistListItem.svelte";
import ArtistListRow from "$lib/components/library/artist/ArtistListRow.svelte";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import { t } from "$lib/i18n";
import FilterBar from "$lib/components/library/common/FilterBar.svelte";
import AlphabetNav from "$lib/components/ui/alphabet/AlphabetNav.svelte";
import type { ArtistListView } from "$lib/types/ui/library/artist/ArtistListView";

const libraryId = $derived(Number(page.params.library_id));
const currentLibrary = $derived(
  $libraryStore.libraries.find(l => l.id === libraryId)
);

const SORT_KEY = 'filterbar:artists';
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
  { key: 'name', label: 'Nom', icon: 'lucide:type' },
  { key: 'albums', label: 'Albums', icon: 'lucide:disc-album' },
  { key: 'tracks', label: 'Titres', icon: 'lucide:music' },
];

let filteredArtists = $derived.by(() => {
  let result = [...$libraryContentStore.artists];

  if (filterQuery.length >= 2) {
    const q = filterQuery.toLowerCase();
    result = result.filter(a => a.name?.toLowerCase().includes(q));
  }

  if (sortBy !== 'default') {
    const dir = sortDir === 'asc' ? 1 : -1;
    result.sort((a, b) => {
      switch (sortBy) {
        case 'name': return (a.name ?? '').localeCompare(b.name ?? '') * dir;
        case 'albums': return ((a.total_albums ?? 0) - (b.total_albums ?? 0)) * dir;
        case 'tracks': return ((a.total_tracks ?? 0) - (b.total_tracks ?? 0)) * dir;
        default: return 0;
      }
    });
  }

  return result;
});

$effect(() => {
  const total = filteredArtists.length;
  libraryHeader.update(current => {
    if (current.total === total && current.subtitle === 'Artistes') return current;
    return { subtitle: 'Artistes', icon: 'lucide:mic-2', total };
  });
});

// --- Alphabet navigation ---
let scrollContainer = $state<HTMLDivElement | null>(null);

function firstLetter(name: string | null | undefined): string {
  if (!name) return '#';
  const first = name.trim().charAt(0).toUpperCase();
  return /[A-Z]/.test(first) ? first : '#';
}

let availableLetters = $derived(
  new Set(filteredArtists.map(a => firstLetter(a.name)))
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

function shouldShowLetter(index: number, artist: ArtistListView): boolean {
  if (index === 0) return true;
  return firstLetter(filteredArtists[index - 1].name) !== firstLetter(artist.name);
}
</script>

{#if $libraryStore.isImporting}
  <LibraryImportingLoader />

{:else if currentLibrary?.total_artists === 0}
  <div class="flex flex-col items-center justify-center py-20 px-6 text-center">
    <div class="relative mb-5">
      <div class="absolute inset-0 rounded-full bg-green-500/25 blur-2xl scale-[2] animate-pulse"></div>
      <div class="relative w-16 h-16 rounded-full
                  bg-neutral-100 dark:bg-neutral-800
                  border border-neutral-200/60 dark:border-neutral-700/40
                  flex items-center justify-center">
        <Icon icon="lucide:mic-2" width="24" class="text-green-500/60" />
      </div>
    </div>
    <h3 class="text-base font-semibold text-neutral-700 dark:text-neutral-200 mb-1.5">
      {$t('library.no_artist')}
    </h3>
    <p class="text-sm text-neutral-400 dark:text-neutral-500 max-w-xs leading-relaxed mb-6">
      {$t('library.no_artist_desc')}
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
  <LibraryArtistSkeleton />

{:else}
  <div class="flex flex-col h-full">
    <FilterBar
      bind:filterQuery bind:sortBy bind:sortDir
      {sortOptions}
    />

    <div class="flex-1 relative min-h-0">
      <div class="absolute inset-0 scrollbar-app overflow-y-auto p-6" bind:this={scrollContainer}>
        {#if filteredArtists.length === 0}
          <div class="flex flex-col items-center justify-center py-20 text-center">
            <Icon icon="lucide:search-x" width="32" class="text-neutral-300 dark:text-neutral-600 mb-3" />
            <p class="text-sm text-neutral-400">{$t('search.no_result_for').replace('{query}', filterQuery)}</p>
          </div>
        {:else if $viewMode === 'grid'}
          <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-8 gap-6 relative">
            {#each filteredArtists as artist, i (artist.id)}
              <div class="relative">
                {#if shouldShowLetter(i, artist)}
                  <div class="absolute -top-3 left-0 h-0 scroll-mt-6" data-letter={firstLetter(artist.name)}></div>
                {/if}
                <ArtistListItem {libraryId} {artist} />
              </div>
            {/each}
          </div>
        {:else}
          <div class="flex flex-col">
            {#each filteredArtists as artist, i (artist.id)}
              {#if shouldShowLetter(i, artist)}
                <div class="h-0 scroll-mt-6" data-letter={firstLetter(artist.name)}></div>
              {/if}
              <ArtistListRow {libraryId} {artist} />
            {/each}
          </div>
        {/if}
      </div>

      {#if filteredArtists.length > 0}
        <AlphabetNav {availableLetters} onletter={scrollToLetter} />
      {/if}
    </div>
  </div>
{/if}
