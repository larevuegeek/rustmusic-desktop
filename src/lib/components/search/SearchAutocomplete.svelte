<script lang="ts">
  import { detectOS } from "#lib/helper/tools/osDetection";
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
  import { goto } from "$app/navigation";
  import { handlePlayTrack } from "#lib/actions/player/PlayerAction";
  import { versFileDAttente } from "#lib/mapper/queue/mapQueueTrack";
  import { sidebarStore } from "#lib/stores/ui/sidebar.store";
  import { t } from "#lib/i18n";

  type SearchResult = {
    id: string;
    result_type: string;
    title: string;
    subtitle: string | null;
    thumbnail_path: string | null;
    path: string | null;
    library_id: number | null;
  };

  let { query = $bindable('') }: { query?: string } = $props();

  let results: SearchResult[] = $state([]);
  let loading = $state(false);
  let showDropdown = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let selectedIndex = $state(-1);

  // Pas de recherche différée après la disparition du champ.
  $effect(() => () => { if (debounceTimer) clearTimeout(debounceTimer); });

  function handleInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    if (query.length < 2) {
      results = [];
      showDropdown = false;
      return;
    }

    debounceTimer = setTimeout(() => doSearch(), 200);
  }

  async function doSearch() {
    const demande = query;
    if (demande.length < 2) return;
    loading = true;
    try {
      const r = await invoke<SearchResult[]>('search', { query: demande, limit: 10 });
      // Champ modifié ou vidé pendant la recherche : ce résultat n'est plus le bon.
      if (demande !== query) return;
      results = r;
      showDropdown = results.length > 0;
      selectedIndex = -1;
    } catch (e) {
      console.error('Search failed:', e);
      if (demande === query) results = [];
    } finally {
      if (demande === query) loading = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!showDropdown) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, -1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (selectedIndex >= 0) {
        handleResultClick(results[selectedIndex]);
      } else if (query.length >= 2) {
        // Aller à la page de résultats complète
        goto(`/search?q=${encodeURIComponent(query)}`);
        close();
      }
    } else if (e.key === 'Escape') {
      close();
    }
  }

  function handleResultClick(result: SearchResult) {
    if (result.result_type === 'track' && result.path) {
      // Seuls les morceaux de la liste entrent dans la file.
      handlePlayTrack(result.path, versFileDAttente(
        results.filter((r) => r.result_type === 'track' && r.path)
      ));
    } else if (result.result_type === 'album' && result.library_id) {
      goto(`/library/${result.library_id}/albums/${result.id}`);
    } else if (result.result_type === 'artist' && result.library_id) {
      goto(`/library/${result.library_id}/artists/${result.id}`);
    }
    close();
  }

  function close() {
    showDropdown = false;
    selectedIndex = -1;
    sidebarStore.close();
  }

  function handleFocus() {
    if (results.length > 0 && query.length >= 2) showDropdown = true;
  }

  function handleBlur() {
    // Petit délai pour permettre le clic sur un résultat
    setTimeout(() => { showDropdown = false; }, 200);
  }

  function getIcon(type: string) {
    switch (type) {
      case 'track': return 'lucide:music';
      case 'album': return 'lucide:disc-album';
      case 'artist': return 'lucide:mic-2';
      default: return 'lucide:search';
    }
  }

  function getTypeLabel(type: string) {
    switch (type) {
      case 'track': return $t('track_view.kicker');
      case 'album': return $t('sidebar.album');
      case 'artist': return $t('sidebar.artist');
      default: return '';
    }
  }
  const raccourci = detectOS() === 'macos' ? '⌘ K' : 'Ctrl K';
</script>

<div class="relative w-full">
  <!-- Champ. En contraste élevé, le trait de focus fait le tour du champ (`row`)
       plutôt que de la seule zone de saisie. -->
  <div
    data-focus-ring="row"
    class="group relative flex items-center gap-2.5 h-9 pl-3 pr-1.5 rounded-[10px] border
           transition-[background-color,border-color,box-shadow] duration-150
           bg-white border-(--c-bord) hover:border-(--c-bord2)
           focus-within:border-emerald-500/60 focus-within:shadow-[0_0_0_3px_rgba(34,197,94,0.14)]
           dark:bg-[#131715] dark:border-[#1f2522] dark:hover:border-[#2e3632]
           dark:focus-within:bg-[#151a17] dark:focus-within:border-emerald-500/50
           dark:focus-within:shadow-[0_0_0_3px_rgba(34,197,94,0.12)]"
  >
    {#if loading}
      <Icon icon="lucide:loader-2" class="h-4 w-4 shrink-0 animate-spin text-green-500" />
    {:else}
      <Icon icon="mynaui:search" class="h-4 w-4 shrink-0 transition-colors text-[#5e625d] dark:text-[#8b948f]
                                        group-focus-within:text-emerald-500" />
    {/if}

    <input
      type="search"
      data-focus-ring="none"
      bind:value={query}
      oninput={handleInput}
      onkeydown={handleKeydown}
      onfocus={handleFocus}
      onblur={handleBlur}
      class="w-full bg-transparent outline-none text-[13px]
             dark:text-neutral-100 dark:placeholder:text-[#8b948f]
             text-[#1a1c1a] placeholder:text-[#6f736d]"
      placeholder={$t("search.placeholder")}
      autocomplete="off"
      spellcheck="false"
    />
    <!-- Le raccourci existe (layout) : on le montre tant que le champ est vide. -->
    {#if !query}
      <kbd class="hidden lg:inline-flex group-focus-within:hidden shrink-0 items-center h-5.5 px-1.5 rounded-md border pointer-events-none
                  font-mono text-[10.5px] leading-none text-[#5e625d] dark:text-[#8b948f]
                  border-(--c-bord) dark:border-[#2e3632] bg-(--c-kbd) dark:bg-[#0b0d0c]">{raccourci}</kbd>
    {/if}
  </div>

  <!-- Dropdown résultats -->
  {#if showDropdown}
    <div class="absolute top-full left-0 right-0 mt-2 z-50
                bg-white dark:bg-neutral-900
                border border-neutral-200/60 dark:border-white/10
                rounded-xl shadow-2xl shadow-black/20
                overflow-hidden max-h-[400px] overflow-y-auto">

      {#each results as result, i (result.id + result.result_type)}
        <button
          class="w-full flex items-center gap-3 px-4 py-2.5 text-left cursor-pointer
                 transition-colors duration-100
                 {i === selectedIndex
                   ? 'bg-green-500/10 dark:bg-green-500/15'
                   : 'hover:bg-neutral-50 dark:hover:bg-white/5'}"
          onclick={() => handleResultClick(result)}
        >
          <!-- Thumbnail -->
          {#if result.thumbnail_path}
            <CoverImg path={result.thumbnail_path} alt=""
                 class="w-10 h-10 rounded-lg object-cover shrink-0" />
          {:else}
            <div class="w-10 h-10 rounded-lg shrink-0 flex items-center justify-center
                        bg-neutral-100 dark:bg-neutral-800">
              <Icon icon={getIcon(result.result_type)} width="16"
                    class="text-neutral-400" />
            </div>
          {/if}

          <!-- Infos -->
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200 truncate">
              {result.title}
            </p>
            {#if result.subtitle}
              <p class="text-xs text-neutral-500 dark:text-neutral-400 truncate">
                {result.subtitle}
              </p>
            {/if}
          </div>

          <!-- Type badge -->
          <span class="text-[9px] uppercase tracking-wider font-semibold px-1.5 py-0.5 rounded
                       bg-neutral-100 dark:bg-white/5
                       text-neutral-400 dark:text-neutral-500 shrink-0">
            {getTypeLabel(result.result_type)}
          </span>
        </button>
      {/each}

      <!-- Voir tous les résultats -->
      <button
        class="w-full flex items-center justify-center gap-2 px-4 py-2.5 text-xs
               text-green-600 dark:text-green-400 font-medium
               border-t border-neutral-200/60 dark:border-white/5
               hover:bg-green-500/5 cursor-pointer transition-colors"
        onclick={() => { goto(`/search?q=${encodeURIComponent(query)}`); close(); }}
      >
        <Icon icon="lucide:search" width="12" />
        {$t("search.see_all")}
      </button>
    </div>
  {/if}
</div>
