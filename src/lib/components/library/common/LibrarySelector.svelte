<script lang="ts">
import { libraryStore } from "$lib/stores/library/library.store"
import { t } from "$lib/i18n";
import Icon from "@iconify/svelte";
import { goto } from "$app/navigation";
import { scale } from "svelte/transition";
import type { Library } from "$lib/types/db/library/Library";
import { popinStore } from "$lib/stores/ui/popin.store";
import AddLibraryPopin from "$lib/components/library/common/popin/AddLibraryPopin.svelte";
import { page } from "$app/state";

let isOpen = $state(false);
let selectorEl: HTMLElement | null = $state(null);

let activeLibrary = $derived($libraryStore.librarySelected);
let otherLibraries = $derived($libraryStore.libraries.filter(lib => lib.id !== activeLibrary?.id));

function toggleDropdown() {
  isOpen = !isOpen;
}

function handleSelectLibrary(library: Library) {
  libraryStore.selectLibrary(library);
  isOpen = false;

  // Redirect to the same section with the new library_id
  const path = page.url.pathname;
  const match = path.match(/^\/library\/\d+\/(albums|artists|tracks|genres|folders)(\/.*)?$/);
  if (match) {
    goto(`/library/${library.id}/${match[1]}`);
  } else if (path.startsWith('/library/')) {
    goto(`/library/${library.id}`);
  }
}

/**
 * Désigne la bibliothèque ouverte au démarrage.
 *
 * Le menu reste ouvert : marquer un défaut n'est pas une navigation, et le
 * refermer masquerait le repère qui vient d'apparaître.
 */
async function handleSetDefault(library: Library) {
  try {
    await libraryStore.setDefault(library);
  } catch (e) {
    console.error("Failed to set default library", e);
  }
}

function handleViewLibrary(library: Library) {
  if (!library) return;
  goto(`/library/${library.id}`);
  isOpen = false;
}

// Fermer au clic extérieur
$effect(() => {
  if (!isOpen) return;

  function handleClickOutside(e: MouseEvent) {
    if (selectorEl && !selectorEl.contains(e.target as Node)) {
      isOpen = false;
    }
  }

  const timer = setTimeout(() => {
    document.addEventListener('click', handleClickOutside);
  }, 50);

  return () => {
    clearTimeout(timer);
    document.removeEventListener('click', handleClickOutside);
  };
});
</script>

<div class="relative mb-3" bind:this={selectorEl}>
  <!-- Bouton principal -->
  <button
    type="button"
    class="group flex items-center gap-2.5 rounded-xl w-full px-3 py-2
           transition-all duration-200 cursor-pointer
           bg-neutral-100/60 dark:bg-white/3
           border border-neutral-200/40 dark:border-white/6
           hover:bg-neutral-100 dark:hover:bg-white/6
           hover:border-neutral-300/60 dark:hover:border-white/10"
    onclick={toggleDropdown}
    aria-expanded={isOpen}
  >
    <!-- Icône -->
    <div class="w-8 h-8 rounded-lg shrink-0 flex items-center justify-center
                bg-green-500/15 text-green-500">
      <Icon icon="lucide:library" width="14" height="14" />
    </div>

    <!-- Infos -->
    <span class="flex-1 flex flex-col leading-tight text-left min-w-0">
      {#if $libraryStore.isLoading}
        <span class="text-xs text-neutral-400">{$t('common.loading')}</span>
      {:else if activeLibrary}
        <span class="text-[13px] font-medium text-neutral-800 dark:text-neutral-200 truncate">
          {activeLibrary.name}
        </span>
        <span class="text-[10px] text-neutral-400 dark:text-neutral-500">
          {activeLibrary.total_tracks} titre{activeLibrary.total_tracks !== 1 ? 's' : ''}
        </span>
      {:else}
        <span class="text-xs text-neutral-400">{$t('selector.no_library')}</span>
      {/if}
    </span>

    <!-- Chevron -->
    <svg
      class="w-3.5 h-3.5 shrink-0 text-neutral-400 transition-transform duration-200
             {isOpen ? 'rotate-180' : ''}"
      viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
      <path d="m6 9 6 6 6-6"/>
    </svg>
  </button>

  <!-- Dropdown -->
  {#if isOpen}
    <div
      class="absolute top-full left-0 right-0 mt-1.5 z-50"
      transition:scale={{ duration: 120, start: 0.97 }}
    >
      <div class="rounded-xl border overflow-hidden
                  bg-white/98 dark:bg-neutral-900/98 backdrop-blur-xl
                  border-neutral-200/60 dark:border-white/8
                  shadow-xl shadow-black/10 dark:shadow-black/40">

        <!-- Header -->
        <div class="px-3 pt-3 pb-2">
          <span class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500">
            {$t('selector.libraries')}
          </span>
        </div>

        <div class="max-h-64 overflow-y-auto px-1.5">
          {#if $libraryStore.libraries.length === 0}
            <div class="px-3 py-6 text-center">
              <div class="w-10 h-10 rounded-xl mx-auto mb-2
                          bg-green-500/10 flex items-center justify-center">
                <Icon icon="lucide:library" width="16" class="text-green-500/60" />
              </div>
              <p class="text-xs text-neutral-400">{$t('selector.no_library')}</p>
            </div>
          {:else}
            <!-- Deux boutons côte à côte plutôt qu'un seul : sélectionner et
                 désigner par défaut sont deux gestes distincts, et un bouton
                 imbriqué dans un autre n'est pas du HTML valide. -->
            {#each $libraryStore.libraries as library (library.id)}
              {@const isActive = library.id === activeLibrary?.id}
              <div
                class="w-full flex items-center gap-1 pr-1 rounded-lg
                       transition-all duration-150
                       {isActive
                         ? 'bg-green-500/10'
                         : 'hover:bg-neutral-100/80 dark:hover:bg-white/4'}"
              >
                <button
                  type="button"
                  class="flex-1 min-w-0 flex items-center gap-2.5 px-2.5 py-2 cursor-pointer"
                  onclick={() => handleSelectLibrary(library)}
                >
                  <!-- Check / icône -->
                  <div class="w-7 h-7 rounded-md shrink-0 flex items-center justify-center
                              {isActive
                                ? 'bg-green-500/20 text-green-500'
                                : 'bg-neutral-100 dark:bg-neutral-800 text-neutral-400 dark:text-neutral-500'}">
                    {#if isActive}
                      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                        <path d="m4.5 12.75 6 6 9-13.5"/>
                      </svg>
                    {:else}
                      <Icon icon="lucide:library" width="12" />
                    {/if}
                  </div>

                  <!-- Infos -->
                  <div class="flex-1 min-w-0 text-left">
                    <div class="flex items-center gap-1.5 min-w-0">
                      <span class="text-[13px] font-medium truncate
                                  {isActive ? 'text-green-600 dark:text-green-400' : 'text-neutral-700 dark:text-neutral-300'}">
                        {library.name}
                      </span>
                      <!-- Repère permanent : la marque doit se lire sans avoir
                           à survoler quoi que ce soit. -->
                      {#if library.is_default}
                        <span
                          title={$t('selector.default_hint')}
                          class="shrink-0 flex items-center gap-0.5 px-1 py-px rounded
                                 text-[9px] font-semibold uppercase tracking-wide
                                 bg-green-500/15 text-green-600 dark:text-green-400"
                        >
                          <Icon icon="lucide:pin" width="9" />
                          {$t('selector.default')}
                        </span>
                      {/if}
                    </div>
                    <div class="text-[10px] text-neutral-400 dark:text-neutral-500">
                      {library.total_tracks} titre{library.total_tracks !== 1 ? 's' : ''}
                      · {library.total_albums} album{library.total_albums !== 1 ? 's' : ''}
                    </div>
                  </div>
                </button>

                {#if !library.is_default}
                  <button
                    type="button"
                    title={$t('selector.set_default')}
                    aria-label={$t('selector.set_default')}
                    class="shrink-0 w-7 h-7 rounded-md flex items-center justify-center
                           cursor-pointer transition-colors
                           text-neutral-300 dark:text-neutral-600
                           hover:text-green-600 dark:hover:text-green-400
                           hover:bg-green-500/12"
                    onclick={() => handleSetDefault(library)}
                  >
                    <Icon icon="lucide:pin" width="13" />
                  </button>
                {/if}
              </div>
            {/each}
          {/if}
        </div>

        <!-- Footer -->
        <div class="p-1.5 border-t border-neutral-200/60 dark:border-white/6 flex gap-1">
          {#if activeLibrary}
            <button
              type="button"
              class="flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg
                     text-xs font-medium cursor-pointer transition-all duration-150
                     text-neutral-500 dark:text-neutral-400
                     hover:bg-neutral-100 dark:hover:bg-white/4"
              onclick={() => { if (activeLibrary) handleViewLibrary(activeLibrary); }}
            >
              <Icon icon="lucide:arrow-right" width="12" />
              {$t('selector.view')}
            </button>
          {/if}
          <button
            type="button"
            class="flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg
                   text-xs font-medium cursor-pointer transition-all duration-150
                   text-green-600 dark:text-green-400
                   hover:bg-green-500/10"
            onclick={() => {
              isOpen = false;
              popinStore.open($t('library.create_library'), AddLibraryPopin, {});
            }}
          >
            <Icon icon="lucide:plus" width="12" />
            {$t('selector.new')}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
