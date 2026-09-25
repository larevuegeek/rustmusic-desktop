<script lang="ts">
  import Icon from "@iconify/svelte";
  import { t } from "$lib/i18n";
  import { page } from "$app/state";
  import { invoke } from "@tauri-apps/api/core";
  import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
  import { goto } from "$app/navigation";
  import { handlePlayTrack } from "$lib/actions/player/PlayerAction";
  import { versFileDAttente } from "$lib/mapper/queue/mapQueueTrack";
  import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";

  type SearchResult = {
    id: string;
    result_type: string;
    title: string;
    subtitle: string | null;
    thumbnail_path: string | null;
    path: string | null;
    library_id: number | null;
  };

  let results: SearchResult[] = $state([]);
  let loading = $state(false);
  let contextMenu = $state<{ x: number; y: number; result: SearchResult } | null>(null);

  const query = $derived(page.url.searchParams.get('q') ?? '');

  $effect(() => {
    if (query.length >= 2) doSearch();
  });

  async function doSearch() {
    loading = true;
    try {
      results = await invoke<SearchResult[]>('search', { query, limit: 50 });
    } catch (e) {
      console.error('Search failed:', e);
    } finally {
      loading = false;
    }
  }

  function handleClick(result: SearchResult) {
    if (result.result_type === 'track' && result.path) {
      handlePlayTrack(result.path, versFileDAttente(tracks));
    } else if (result.result_type === 'album' && result.library_id) {
      goto(`/library/${result.library_id}/albums/${result.id}`);
    } else if (result.result_type === 'artist' && result.library_id) {
      goto(`/library/${result.library_id}/artists/${result.id}`);
    }
  }

  function getIcon(type: string) {
    switch (type) {
      case 'track': return 'lucide:music';
      case 'album': return 'lucide:disc-album';
      case 'artist': return 'lucide:mic-2';
      default: return 'lucide:search';
    }
  }

  let tracks = $derived(results.filter(r => r.result_type === 'track'));
  let albums = $derived(results.filter(r => r.result_type === 'album'));
  let artists = $derived(results.filter(r => r.result_type === 'artist'));
</script>

<div class="py-5 px-4 md:px-10 scrollbar-app overflow-y-auto" style="height: calc(100vh - 290px);">

  <!-- Header -->
  <div class="mb-6">
    <p class="text-xs uppercase tracking-widest text-neutral-400 mb-1">{$t('search.results_for')}</p>
    <h1 class="text-2xl font-bold text-neutral-900 dark:text-neutral-100">
      « {query} »
    </h1>
    <p class="text-sm text-neutral-400 mt-1">{results.length} résultat{results.length !== 1 ? 's' : ''}</p>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-20">
      <Icon icon="lucide:loader-2" width="24" class="animate-spin text-neutral-400" />
    </div>

  {:else if results.length === 0}
    <div class="flex flex-col items-center justify-center py-20 text-center">
      <Icon icon="lucide:search-x" width="40" class="text-neutral-300 dark:text-neutral-600 mb-4" />
      <h3 class="text-base font-semibold text-neutral-600 dark:text-neutral-300">{$t('search.no_result')}</h3>
      <p class="text-sm text-neutral-400 mt-1">{$t('search.no_result_desc')}</p>
    </div>

  {:else}

    <!-- Artistes -->
    {#if artists.length > 0}
      <div class="mb-8">
        <h2 class="text-xs font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500 mb-3">
          Artistes ({artists.length})
        </h2>
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3">
          {#each artists as result (result.id)}
            <button
              class="group flex flex-col items-center gap-2 p-4 rounded-2xl cursor-pointer
                     bg-white/50 dark:bg-white/3
                     border border-neutral-200/50 dark:border-white/5
                     hover:bg-green-500/5 hover:border-green-500/20
                     transition-all duration-200"
              onclick={() => handleClick(result)}
            >
              <div class="w-14 h-14 rounded-full flex items-center justify-center
                          bg-neutral-100 dark:bg-neutral-800
                          group-hover:bg-green-500/10 transition-colors">
                <Icon icon="lucide:mic-2" width="20" class="text-neutral-400 group-hover:text-green-500" />
              </div>
              <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200 truncate w-full text-center">
                {result.title}
              </p>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Albums -->
    {#if albums.length > 0}
      <div class="mb-8">
        <h2 class="text-xs font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500 mb-3">
          Albums ({albums.length})
        </h2>
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-6 min-[1800px]:grid-cols-7 min-[2200px]:grid-cols-8 gap-3">
          {#each albums as result (result.id)}
            <button
              class="group flex flex-col items-center gap-2 p-3 rounded-xl cursor-pointer
                     hover:bg-neutral-50 dark:hover:bg-white/4
                     transition-all duration-200"
              onclick={() => handleClick(result)}
            >
              {#if result.thumbnail_path}
                <CoverImg path={result.thumbnail_path} alt=""
                     class="w-full aspect-square rounded-lg object-cover shadow-sm" />
              {:else}
                <div class="w-full aspect-square rounded-lg flex items-center justify-center
                            bg-neutral-100 dark:bg-neutral-800">
                  <Icon icon="lucide:disc-album" width="28" class="text-neutral-400" />
                </div>
              {/if}
              <div class="w-full text-left">
                <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200 truncate" title={result.title}>{result.title}</p>
                {#if result.subtitle}
                  <p class="text-xs text-neutral-400 truncate">{result.subtitle}</p>
                {/if}
              </div>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Morceaux -->
    {#if tracks.length > 0}
      <div class="mb-8">
        <h2 class="text-xs font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500 mb-3">
          Morceaux ({tracks.length})
        </h2>
        <div class="flex flex-col">
          {#each tracks as result, i (result.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="group flex items-center gap-3 px-3 py-2.5 rounded-xl cursor-pointer
                     hover:bg-neutral-50 dark:hover:bg-white/4
                     transition-colors duration-100"
              ondblclick={() => result.path && handlePlayTrack(result.path, versFileDAttente(tracks))}
              oncontextmenu={(e) => { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY, result }; }}
            >
              <div class="w-6 text-xs text-neutral-400 text-right shrink-0">
                {String(i + 1).padStart(2, '0')}
              </div>

              {#if result.thumbnail_path}
                <CoverImg path={result.thumbnail_path} alt=""
                     class="w-10 h-10 rounded-md object-cover shrink-0" />
              {:else}
                <div class="w-10 h-10 rounded-md shrink-0 flex items-center justify-center
                            bg-neutral-100 dark:bg-neutral-800">
                  <Icon icon="lucide:music" width="14" class="text-neutral-400" />
                </div>
              {/if}

              <div class="flex-1 min-w-0">
                <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200 truncate" title={result.title}>{result.title}</p>
                {#if result.subtitle}
                  <p class="text-xs text-neutral-400 truncate">{result.subtitle}</p>
                {/if}
              </div>

              <button
                class="p-1.5 rounded-lg shrink-0 cursor-pointer opacity-0 group-hover:opacity-100
                       text-green-500 hover:bg-green-500/15 transition-all"
                onclick={() => result.path && handlePlayTrack(result.path, versFileDAttente(tracks))}
              >
                <Icon icon="lucide:play" width="14" />
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

{#if contextMenu}
  <TrackContextMenu
    track={{ path: contextMenu.result.path, title: contextMenu.result.title, id: contextMenu.result.id }}
    x={contextMenu.x}
    y={contextMenu.y}
    libraryId={contextMenu.result.library_id}
    onclose={() => contextMenu = null}
  />
{/if}
