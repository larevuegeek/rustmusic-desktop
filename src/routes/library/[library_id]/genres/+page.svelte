<script lang="ts">
import { page } from "$app/state";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { libraryHeader } from "$lib/stores/library/libraryHeader";
import { t } from "$lib/i18n";
import { libraryStore } from "$lib/stores/library/library.store";
import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
import FilterBar from "$lib/components/library/common/FilterBar.svelte";
import type { GenreView } from "$lib/types/ui/library/genre/GenreView";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import GenreListRow from "$lib/components/library/genre/GenreListRow.svelte";
import { cleGroupe, toggleGroupSelection } from "$lib/helper/tools/selectionGroups";

const libraryId = $derived(Number(page.params.library_id));

let genres: GenreView[] = $state([]);
let isLoading = $state(true);
const SORT_KEY = 'filterbar:genres';
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

$effect(() => {
  const _id = libraryId;
  loadGenres();
});

async function loadGenres() {
  isLoading = true;
  try {
    genres = await invoke<GenreView[]>('get_genres', { libraryId });
  } catch (e) {
    console.error('Failed to load genres:', e);
    genres = [];
  } finally {
    isLoading = false;
  }
}

let filteredGenres = $derived.by(() => {
  let result = [...genres];

  if (filterQuery.length >= 2) {
    const q = filterQuery.toLowerCase();
    result = result.filter(g => g.name.toLowerCase().includes(q));
  }

  if (sortBy !== 'default') {
    const dir = sortDir === 'asc' ? 1 : -1;
    result.sort((a, b) => {
      switch (sortBy) {
        case 'name': return a.name.localeCompare(b.name) * dir;
        case 'albums': return (a.total_albums - b.total_albums) * dir;
        case 'tracks': return (a.total_tracks - b.total_tracks) * dir;
        default: return 0;
      }
    });
  }

  return result;
});

$effect(() => {
  libraryHeader.update(current => {
    const total = filteredGenres.length;
    if (current.total === total && current.subtitle === 'Genres') return current;
    return { subtitle: 'Genres', icon: 'lucide:tag', total };
  });
});

// ─── Sélection d'un genre ───
//
// Un genre n'a pas d'identifiant : son nom en tient lieu, ici comme dans la
// route et dans la requête.
const selection = $derived($selectionStore);

function genreSelectionne(nom: string): boolean {
  return selection.groupes.has(cleGroupe('genre', libraryId, nom));
}

function handleGenreClick(nom: string) {
  if (selection.active) {
    toggleGroupSelection('genre', libraryId, nom);
    return;
  }
  goto(`/library/${libraryId}/genres/${encodeURIComponent(nom)}`);
}

// Couleurs aléatoires mais déterministes par genre (hash simple)
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
</script>

{#if $libraryStore.isImporting}
  <LibraryImportingLoader />

{:else if isLoading}
  <div class="flex items-center justify-center py-20">
    <Icon icon="lucide:loader-2" width="24" class="animate-spin text-neutral-400" />
  </div>

{:else if genres.length === 0}
  <div class="flex flex-col items-center justify-center py-20 px-6 text-center">
    <div class="relative mb-5">
      <div class="absolute inset-0 rounded-2xl bg-green-500/25 blur-2xl scale-[2] animate-pulse"></div>
      <div class="relative w-16 h-16 rounded-2xl
                  bg-neutral-100 dark:bg-neutral-800
                  border border-neutral-200/60 dark:border-neutral-700/40
                  flex items-center justify-center">
        <Icon icon="lucide:tag" width="24" class="text-green-500/60" />
      </div>
    </div>
    <h3 class="text-base font-semibold text-neutral-700 dark:text-neutral-200 mb-1.5">
      {$t('library.no_genre')}
    </h3>
    <p class="text-sm text-neutral-400 dark:text-neutral-500 max-w-xs leading-relaxed">
      {$t('library.no_genre_desc')}
    </p>
  </div>

{:else}
  <div class="flex flex-col h-full">
    <FilterBar
      bind:filterQuery bind:sortBy bind:sortDir
      {sortOptions}
    />

    <div class="flex-1 scrollbar-app overflow-y-auto p-6">
      {#if $viewMode === 'list'}
        <!-- En-tête de colonnes : mêmes largeurs que les lignes. Sans elle, les
             deux nombres de droite ne s'expliquent pas. -->
        <div class="flex items-center gap-4 px-3 py-2 mb-1
                    text-[10px] uppercase tracking-wider text-neutral-400
                    border-b border-neutral-200/60 dark:border-white/5">
          <div class="w-12 shrink-0"></div>
          <div class="flex-1 min-w-0">Genre</div>
          <div class="hidden sm:block w-24 text-right shrink-0">Albums</div>
          <div class="w-24 text-right shrink-0">Titres</div>
        </div>
        {#each filteredGenres as genre (genre.name)}
          <GenreListRow {libraryId} {genre} color={genreColor(genre.name)} />
        {/each}
      {:else}
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-8 gap-4">
        {#each filteredGenres as genre (genre.name)}
          {@const color = genreColor(genre.name)}
          <button
            type="button"
            class="group relative aspect-square overflow-hidden rounded-2xl cursor-pointer
                   transition-all duration-200
                   hover:shadow-2xl hover:shadow-black/20 active:scale-[0.97] hover:scale-[1.03]"
            onclick={() => handleGenreClick(genre.name)}
          >
            <!-- Case de sélection, posée sur la vignette : une grille n'a pas
                 de marge où loger une colonne de cases. -->
            {#if selection.active}
              <div class="absolute top-2 left-2 z-20 w-5 h-5 rounded flex items-center justify-center
                          transition-all duration-150
                          {genreSelectionne(genre.name)
                            ? 'bg-emerald-500 text-white'
                            : 'bg-black/40 backdrop-blur-sm border border-white/40 text-transparent'}">
                <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                     stroke-width="3" stroke-linecap="round">
                  <path d="m4.5 12.75 6 6 9-13.5"/>
                </svg>
              </div>
            {/if}
            <!-- Fond -->
            {#if genre.covers.length >= 4}
              <div class="absolute inset-0 grid grid-cols-2">
                {#each genre.covers.slice(0, 4) as cover}
                  <CoverImg
                    path={cover}
                    alt=""
                    class="w-full h-full object-cover"
                  />
                {/each}
              </div>
            {:else if genre.covers.length >= 1}
              <CoverImg
                path={genre.covers[0]}
                alt=""
                class="absolute inset-0 w-full h-full object-cover"
              />
            {:else}
              <div class="absolute inset-0"
                   style="background: linear-gradient(135deg, {color}30, {color}60);"></div>
            {/if}

            <!-- Overlay gradient -->
            <div class="absolute inset-0 bg-linear-to-t from-black via-black/40 to-black/05"></div>

            <!-- Texte en bas -->
            <div class="absolute inset-x-0 bottom-0 p-4 text-left bg-black/60">
              <h3 class="text-base font-bold text-white drop-shadow-lg leading-tight flex items-center gap-2 min-w-0">
                <span class="w-2 h-2 rounded-full shrink-0" style="background: {color};"></span>
                <span class="truncate">{genre.name}</span>
              </h3>
              <p class="text-[11px] text-white/60 mt-1">
                {genre.total_albums} album{genre.total_albums !== 1 ? 's' : ''}
                · {genre.total_tracks} titre{genre.total_tracks !== 1 ? 's' : ''}
              </p>
            </div>
          </button>
        {/each}
      </div>
      {/if}
    </div>
  </div>
{/if}
