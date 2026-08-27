<script lang="ts">
  import Icon from "@iconify/svelte";
  import { slide } from "svelte/transition";
  import { tick } from "svelte";
  import { selectionStore } from "$lib/stores/ui/selection.store";
  import { libraryContentStore } from "$lib/stores/library/libraryContent.store";

  type SortOption = { key: string; label: string; icon: string };

  let {
    filterQuery = $bindable(''),
    sortBy = $bindable('default'),
    sortDir = $bindable('asc'),
    sortOptions = [],
    onchange = () => {},
  }: {
    filterQuery?: string;
    sortBy?: string;
    sortDir?: string;
    sortOptions?: SortOption[];
    onchange?: () => void;
  } = $props();

  let selection = $derived($selectionStore);

  let showFilters = $state(false);
  let showSortMenu = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  let currentSort = $derived(
    sortOptions.find(o => o.key === sortBy) ?? sortOptions[0]
  );

  let activeCount = $derived(
    (filterQuery.length >= 2 ? 1 : 0)
    + (sortBy !== 'default' ? 1 : 0)
    + ($libraryContentStore.missingAlbumCover ? 1 : 0)
  );

  function handleInput(e: Event) {
    filterQuery = (e.target as HTMLInputElement).value;
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => { await tick(); onchange(); }, 300);
  }

  async function clearFilter() {
    filterQuery = '';
    await tick();
    onchange();
  }

  const descDefaults = new Set(['rating', 'duration', 'date', 'year', 'tracks', 'albums']);

  async function selectSort(col: string) {
    sortBy = col;
    sortDir = descDefaults.has(col) ? 'desc' : 'asc';
    showSortMenu = false;
    await tick();
    onchange();
  }

  async function toggleSortDir() {
    if (sortBy === 'default') return;
    sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    await tick();
    onchange();
  }

  function closeOnClickOutside(node: HTMLElement) {
    const handler = (e: MouseEvent) => {
      if (!node.contains(e.target as Node)) showSortMenu = false;
    };
    document.addEventListener('mousedown', handler);
    return { destroy: () => document.removeEventListener('mousedown', handler) };
  }
</script>

<div class="shrink-0">
  <!-- Header row -->
  <div class="flex items-center justify-between px-4 py-1.5">
    <button
      type="button"
      class="flex items-center gap-1.5 cursor-pointer
             text-[11px] font-medium transition-colors
             {showFilters || activeCount > 0
               ? 'text-green-600 dark:text-green-400'
               : 'text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-300'}"
      onclick={() => showFilters = !showFilters}
    >
      <Icon icon="lucide:sliders-horizontal" width={12} />
      Filtres & tri
      {#if activeCount > 0}
        <span class="w-4 h-4 rounded-full bg-green-500 text-white text-[9px] flex items-center justify-center font-bold">
          {activeCount}
        </span>
      {/if}
      <Icon icon={showFilters ? 'lucide:chevron-up' : 'lucide:chevron-down'} width={12} />
    </button>

    <!-- Le bouton d'entrée en sélection est passé dans la barre d'onglets, qui
         couvre aussi les pages de détail et l'onglet Dossiers. Ne restent ici
         que les commandes qui n'ont de sens qu'une fois la sélection active. -->
    {#if selection.active}
      <div class="flex items-center gap-1 shrink-0">
        <button
          type="button"
          class="flex items-center gap-1 px-2 py-1 rounded-md
                 text-[10px] font-medium cursor-pointer transition-colors
                 text-emerald-500 hover:bg-emerald-500/10"
          onclick={() => selectionStore.selectAllVisible()}
        >
          <Icon icon="lucide:check-check" width={11} />
          Tout
        </button>
        <button
          type="button"
          class="flex items-center gap-1 px-2 py-1 rounded-md
                 text-[10px] font-medium cursor-pointer transition-colors
                 text-neutral-400 hover:text-neutral-300 hover:bg-white/5"
          onclick={() => selectionStore.stop()}
        >
          <Icon icon="lucide:x" width={11} />
          Annuler
        </button>
      </div>
    {/if}
  </div>

  <!-- Filter content row -->
  {#if showFilters}
    <div class="pl-4 pr-6 pb-2.5 pt-0.5 border-b border-neutral-200/60 dark:border-white/5
                flex items-center gap-2" transition:slide={{ duration: 200 }}>

      <!-- Recherche -->
      <div class="relative flex-1 min-w-0">
        <Icon icon="lucide:search" width={13}
              class="absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-400 pointer-events-none" />
        <input
          type="text"
          value={filterQuery}
          oninput={handleInput}
          placeholder="Filtrer…"
          class="w-full h-8 pl-8 pr-8 text-xs rounded-lg
                 bg-neutral-100/80 dark:bg-white/5
                 border border-neutral-200/60 dark:border-white/8
                 text-neutral-800 dark:text-neutral-200
                 placeholder:text-neutral-400 dark:placeholder:text-neutral-500
                 focus:outline-none focus:ring-1 focus:ring-green-500/30
                 transition-all"
        />
        {#if filterQuery}
          <button
            class="absolute right-2 top-1/2 -translate-y-1/2 text-neutral-400 hover:text-neutral-600 cursor-pointer"
            onclick={clearFilter}
          >
            <Icon icon="lucide:x" width={12} />
          </button>
        {/if}
      </div>

      <!-- Sans pochette -->
      <button
        type="button"
        class="flex items-center gap-1.5 h-8 px-2.5 rounded-lg text-[11px] font-medium cursor-pointer
               transition-all duration-150 shrink-0
               {$libraryContentStore.missingAlbumCover
                 ? 'bg-amber-500/10 text-amber-500 ring-1 ring-amber-500/25'
                 : 'bg-neutral-100/80 dark:bg-white/5 border border-neutral-200/60 dark:border-white/8 text-neutral-500 dark:text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200'}"
        onclick={() => libraryContentStore.setMissingAlbumCover(!$libraryContentStore.missingAlbumCover)}
        title="Afficher uniquement les éléments sans pochette"
      >
        <Icon icon="lucide:image-off" width={12} />
        <span class="hidden md:inline">Sans pochette</span>
        {#if $libraryContentStore.missingAlbumCover}
          <Icon icon="lucide:check" width={11} />
        {/if}
      </button>

      <!-- Sort select premium -->
      <div class="relative shrink-0" use:closeOnClickOutside>
        <div class="flex items-stretch h-8 rounded-lg overflow-hidden
                    bg-neutral-100/80 dark:bg-white/5
                    border border-neutral-200/60 dark:border-white/8">

          <button
            type="button"
            class="flex items-center gap-1.5 px-2.5 text-[11px] font-medium cursor-pointer
                   transition-colors
                   {sortBy !== 'default'
                     ? 'text-green-600 dark:text-green-400'
                     : 'text-neutral-700 dark:text-neutral-200'}
                   hover:bg-neutral-200/40 dark:hover:bg-white/5"
            onclick={() => showSortMenu = !showSortMenu}
          >
            <Icon icon="lucide:arrow-up-down" width={12} class="opacity-50" />
            <span class="text-neutral-400 dark:text-neutral-500">Trier</span>
            <span class="w-px h-3.5 bg-neutral-200/60 dark:bg-white/10 mx-0.5"></span>
            <Icon icon={currentSort?.icon ?? 'lucide:list'} width={12} />
            <span>{currentSort?.label ?? 'Par défaut'}</span>
            <Icon icon={showSortMenu ? 'lucide:chevron-up' : 'lucide:chevron-down'}
                  width={11} class="opacity-60" />
          </button>

          {#if sortBy !== 'default'}
            <button
              type="button"
              class="flex items-center justify-center w-8 cursor-pointer
                     border-l border-neutral-200/60 dark:border-white/8
                     text-green-600 dark:text-green-400
                     hover:bg-green-500/10 transition-colors"
              onclick={toggleSortDir}
              title={sortDir === 'asc' ? 'Ascendant' : 'Descendant'}
            >
              <Icon icon={sortDir === 'asc' ? 'lucide:arrow-up-narrow-wide' : 'lucide:arrow-down-wide-narrow'} width={13} />
            </button>
          {/if}
        </div>

        {#if showSortMenu}
          <div class="absolute right-0 top-[calc(100%+4px)] z-50 w-52
                      rounded-lg overflow-hidden
                      bg-white dark:bg-neutral-900
                      border border-neutral-200/60 dark:border-white/10
                      shadow-xl shadow-black/20 dark:shadow-black/40
                      py-1"
               transition:slide={{ duration: 150 }}>
            {#each sortOptions as opt (opt.key)}
              <button
                type="button"
                class="w-full flex items-center gap-2.5 px-3 py-2 text-[11px] font-medium
                       cursor-pointer transition-colors text-left
                       {sortBy === opt.key
                         ? 'text-green-600 dark:text-green-400 bg-green-500/5'
                         : 'text-neutral-700 dark:text-neutral-300 hover:bg-neutral-100 dark:hover:bg-white/5'}"
                onclick={() => selectSort(opt.key)}
              >
                <Icon icon={opt.icon} width={13} class="shrink-0" />
                <span class="flex-1">{opt.label}</span>
                {#if sortBy === opt.key}
                  <Icon icon="lucide:check" width={12} />
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
