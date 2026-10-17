<script lang="ts">
import { libraryStore } from "#lib/stores/library/library.store"
import { pistesAvecImport } from "#lib/stores/library/importProgress.store";
import { t } from "#lib/i18n";
import Icon from "@iconify/svelte";
import { goto } from "$app/navigation";
import { scale } from "svelte/transition";
import type { Library } from "#lib/types/db/library/Library";
import { popinStore } from "#lib/stores/ui/popin.store";
import AddLibraryPopin from "#lib/components/library/common/popin/AddLibraryPopin.svelte";
import LibraryManageMenu, { isLibraryBusy } from "#lib/components/library/common/LibraryManageMenu.svelte";
import LibraryFoldersPopin from "#lib/components/library/common/popin/LibraryFoldersPopin.svelte";
import { page } from "$app/state";
import { portal } from "#lib/helper/portal";
import { currentLocale } from "#lib/i18n";

let { replie = false }: { replie?: boolean } = $props();

let isOpen = $state(false);
let selectorEl: HTMLElement | null = $state(null);
let dropdownEl: HTMLElement | null = $state(null);
let position = $state("");

let activeLibrary = $derived($libraryStore.librarySelected);
let otherLibraries = $derived($libraryStore.libraries.filter(lib => lib.id !== activeLibrary?.id));

const sousTitre = $derived.by(() => {
  const n = $pistesAvecImport(activeLibrary);
  const titres = n === 1 ? $t('home.track_one') : $t('home.tracks_n').replace('{n}', n.toLocaleString($currentLocale));
  const nb = $libraryStore.libraries.length;
  return nb > 1 ? `${titres} · ${$t('sidebar.libraries_n').replace('{n}', String(nb))}` : titres;
});

// Porté dans `body` : la barre défile et, repliée, rognerait le menu.
function toggleDropdown() {
  if (!isOpen && selectorEl) {
    const r = selectorEl.getBoundingClientRect();
    position = replie
      ? `left: ${r.right + 8}px; top: ${r.top}px; width: 272px;`
      : `left: ${r.left}px; top: ${r.bottom + 6}px; width: ${Math.max(r.width, 248)}px;`;
  }
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

// ─── Menu « Gérer » d'une bibliothèque, ouvert à côté de la liste ───
const LARGEUR_GESTION = 340;
let gestion = $state<{ library: Library; style: string } | null>(null);
let gestionEl: HTMLElement | null = $state(null);
let dossiersDe = $state<number | null>(null);
let dossiersOuverts = $state(false);

function openManage(library: Library, e: MouseEvent) {
  if (gestion?.library.id === library.id) {
    gestion = null;
    return;
  }
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  // À côté de la liste, pas du bouton : le menu ne la chevauche pas.
  const liste = dropdownEl?.getBoundingClientRect() ?? r;
  const aDroite = liste.right + 6 + LARGEUR_GESTION <= window.innerWidth - 8;
  const x = aDroite ? liste.right + 6 : Math.max(8, liste.left - 6 - LARGEUR_GESTION);
  // Près du bas, le menu s'appuie sur le bas de la fenêtre.
  const y = r.top + 560 > window.innerHeight ? "bottom: 8px;" : `top: ${Math.max(8, r.top - 8)}px;`;
  gestion = { library, style: `left: ${x}px; ${y} width: ${LARGEUR_GESTION}px; max-height: calc(100vh - 16px);` };
}

const isBusy = (library: Library) => isLibraryBusy(library.id as number);

function closeAll() {
  gestion = null;
  isOpen = false;
}

// Le menu suit la bibliothèque mise à jour (nom, défaut).
const gestionLib = $derived(
  gestion ? ($libraryStore.libraries.find((l) => l.id === gestion!.library.id) ?? gestion.library) : null,
);

function handleViewLibrary(library: Library) {
  if (!library) return;
  goto(`/library/${library.id}`);
  isOpen = false;
}

// Fermer au clic extérieur
$effect(() => {
  if (!isOpen) return;

  function handleClickOutside(e: MouseEvent) {
    const cible = e.target as Node;
    if (gestionEl?.contains(cible)) return;
    if (gestion && !(cible instanceof Element && cible.closest('[data-gestion]'))) gestion = null;
    if (selectorEl && !selectorEl.contains(cible) && !dropdownEl?.contains(cible)) {
      isOpen = false;
    }
  }
  function handleEscape(e: KeyboardEvent) {
    if (e.key === 'Escape') gestion ? (gestion = null) : (isOpen = false);
  }

  const timer = setTimeout(() => {
    document.addEventListener('click', handleClickOutside);
  }, 50);
  document.addEventListener('keydown', handleEscape);

  return () => {
    clearTimeout(timer);
    document.removeEventListener('click', handleClickOutside);
    document.removeEventListener('keydown', handleEscape);
    gestion = null;
  };
});
</script>

<div class="relative {replie ? 'flex-none' : 'flex-1 min-w-0'}" bind:this={selectorEl}>
  <button
    type="button"
    class="flex items-center gap-2.5 w-full rounded-xl cursor-pointer transition-colors
           bg-(--sb-s1) border border-(--sb-bd) hover:border-(--sb-bd2)
           {replie ? 'p-1.75' : 'py-2 pr-1.5 pl-2'}"
    onclick={toggleDropdown}
    aria-expanded={isOpen}
    title={$t('sidebar.change_library')}
  >
    <span class="w-9 h-9 rounded-[9px] shrink-0 flex items-center justify-center bg-(--sb-gbg) text-(--sb-g)">
      <Icon icon="material-symbols:library-music-outline-rounded" width="20" class="sb-icone" />
    </span>

    {#if !replie}
      <span class="flex-1 flex flex-col gap-px min-w-0 text-left">
        {#if $libraryStore.isLoading}
          <small class="text-xs text-(--sb-mu)">{$t('common.loading')}</small>
        {:else if activeLibrary}
          <b class="text-sm font-bold truncate text-(--sb-tx)">{activeLibrary.name}</b>
          <small class="text-xs truncate text-(--sb-mu)">{sousTitre}</small>
        {:else}
          <small class="text-xs text-(--sb-mu)">{$t('selector.no_library')}</small>
        {/if}
      </span>
      <Icon icon="material-symbols:unfold-more-rounded" width="18" class="shrink-0 text-(--sb-tx2)" />
    {/if}
  </button>

  <!-- Dropdown -->
  {#if isOpen}
    <div
      use:portal
      bind:this={dropdownEl}
      class="fixed z-[9999]"
      style={position}
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
            <!-- Deux boutons côte à côte : sélectionner, et ⋯ pour gérer (un bouton
                 imbriqué dans un autre n'est pas du HTML valide). -->
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
                      {$pistesAvecImport(library)} titre{$pistesAvecImport(library) !== 1 ? 's' : ''}
                      · {library.total_albums} album{library.total_albums !== 1 ? 's' : ''}
                    </div>
                  </div>
                </button>

                <button
                  type="button"
                  data-gestion
                  title={$t('library_head.manage')}
                  aria-label={$t('library_head.manage')}
                  aria-expanded={gestion?.library.id === library.id}
                  class="shrink-0 w-7 h-7 rounded-md flex items-center justify-center
                         cursor-pointer transition-colors
                         {gestion?.library.id === library.id
                           ? 'text-neutral-700 dark:text-neutral-200 bg-neutral-200/70 dark:bg-white/10'
                           : 'text-neutral-400 dark:text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-200 hover:bg-neutral-200/60 dark:hover:bg-white/8'}"
                  onclick={(e) => openManage(library, e)}
                >
                  <Icon icon={isBusy(library) ? "material-symbols:progress-activity" : "material-symbols:more-horiz"} width="16" class={isBusy(library) ? "animate-spin" : ""} />
                </button>
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

{#if isOpen && gestion && gestionLib}
  <div
    use:portal
    bind:this={gestionEl}
    role="menu"
    class="fixed z-[10000] overflow-y-auto rounded-2xl border text-(--rg-tx)
           bg-(--rg-carte) border-(--rg-bd) shadow-[0_18px_40px_rgba(0,0,0,0.18)]
           dark:bg-linear-to-b dark:from-[#1c2220] dark:to-[#161a18] dark:border-[#2a312d] dark:shadow-[0_18px_40px_rgba(0,0,0,0.6)]"
    style={gestion.style}
    transition:scale={{ duration: 120, start: 0.97 }}
  >
    <LibraryManageMenu
      library={gestionLib}
      onclose={closeAll}
      onfolders={() => { dossiersDe = gestionLib.id; dossiersOuverts = true; }}
    />
  </div>
{/if}

{#if dossiersOuverts && dossiersDe !== null}
  <LibraryFoldersPopin bind:open={dossiersOuverts} libraryId={dossiersDe} />
{/if}
