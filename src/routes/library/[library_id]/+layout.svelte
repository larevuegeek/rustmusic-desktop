<script lang="ts">
import { page } from "$app/state";
import { profilSelector } from "$lib/stores/profil/profil.store";
import type { Library } from "$lib/types/db/library/Library";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { loadLibrary } from "$lib/services/library/library.service";
import LibraryErrorLoading from "$lib/components/library/common/error/LibraryErrorLoading.svelte";
import Icon from "@iconify/svelte";
import { libraryHeader } from "$lib/stores/library/libraryHeader";
import { t } from "$lib/i18n";
import LibraryActionBar from "$lib/components/library/common/LibraryActionBar.svelte";
import LibraryTabBar from "$lib/components/library/common/LibraryTabBar.svelte";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { lirePlacement } from "$lib/config/libraryTabs";
import { invoke } from "@tauri-apps/api/core";
import { libraryStore } from "$lib/stores/library/library.store";
import { toasts } from "$lib/stores/ui/toast.store";

let library: Library | null = $state(null);
let error: string | null = $state(null);
let dragOver = $state(false);

const libraryId = $derived(Number(page.params.library_id));
const profil = $derived($profilSelector.profilSelected);
const profilColor = $derived(profil?.color ?? '#22c55e');

let currentTag = 0;

$effect(() => {
    const id = libraryId;
    const p = profil;
    library = null;
    error = null;

    if (!p) { error = "Aucun profil sélectionné"; return; }
    if (!id || isNaN(id)) { error = "ID de bibliothèque invalide"; return; }

    const tag = ++currentTag;
    (async () => {
        const result = await loadLibrary(id, p.id, tag, currentTag);
        if (tag !== currentTag) return;
        if (!result) { error = "Bibliothèque introuvable"; return; }
        library = result;
        libraryContentStore.load(id);
    })();
});

let { children } = $props();

/**
 * L'atelier de tags se passe de l'habillage de bibliothèque.
 *
 * C'est un écran de **travail**, pas de navigation : l'en-tête (statistiques,
 * import, synchronisation) et les onglets de section appartiennent au parcours
 * de la bibliothèque et n'ont rien à y faire. Ils coûtent surtout près de
 * 250 px de hauteur — dans un tableau, plusieurs lignes de moins sous les yeux.
 *
 * L'atelier porte déjà son propre en-tête, avec son titre, sa source et son
 * bouton de retour.
 */
let isFocusedView = $derived(page.url.pathname.endsWith("/tags"));

const placementOnglets = $derived(lirePlacement($settingsStore.library_tabs_position));

// Drag & drop : importer des fichiers audio directement dans la bibliothèque
function handleDragOver(e: DragEvent) {
  e.preventDefault();
  dragOver = true;
}
function handleDragLeave() {
  dragOver = false;
}
async function handleDrop(e: DragEvent) {
  e.preventDefault();
  dragOver = false;
  if (!library?.id || !e.dataTransfer?.items.length) return;

  // Récupérer les paths via webkitGetAsEntry (Tauri expose les paths natifs)
  const audioExts = ['mp3', 'flac', 'ogg', 'm4a', 'wav', 'aac', 'opus', 'dsf', 'dff', 'aiff'];
  const files: string[] = [];

  for (let i = 0; i < e.dataTransfer.files.length; i++) {
    const file = e.dataTransfer.files[i];
    const ext = file.name.split('.').pop()?.toLowerCase() ?? '';
    // Sur Tauri, File a un attribut path non-standard
    const filePath = (file as any).path as string | undefined;
    if (filePath && audioExts.includes(ext)) {
      files.push(filePath);
    }
  }

  if (files.length === 0) {
    toasts.push({ type: "info", title: "Aucun fichier audio", message: "Glissez des fichiers audio (.mp3, .flac, etc.)" });
    return;
  }

  libraryStore.setImporting(true);
  try {
    const tracks = await invoke('add_files', { libraryId: library.id, files });
    await libraryContentStore.load(library.id as number);
    await libraryStore.refresh();
    toasts.push({ type: "success", title: "Import terminé", message: `${(tracks as any[]).length} piste(s) ajoutée(s)` });
  } catch (err) {
    toasts.push({ type: "error", title: "Erreur", message: "Impossible d'importer les fichiers" });
  } finally {
    libraryStore.setImporting(false);
  }
}
</script>

{#if error}
    <LibraryErrorLoading error={error} />
{:else if library}
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="flex flex-col h-full relative"
     ondragover={handleDragOver}
     ondragleave={handleDragLeave}
     ondrop={handleDrop}>

  {#if !isFocusedView}
  <!-- HEADER PREMIUM -->
  <div class="relative px-4 md:px-8 pt-4 md:pt-6 pb-3 md:pb-4 overflow-hidden shrink-0">
    <div class="relative flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3 sm:gap-6">
      <!-- Infos -->
      <div class="flex items-start gap-3 sm:gap-5">
        <!-- Icône bibliothèque -->
        <div class="w-12 h-12 sm:w-16 sm:h-16 rounded-xl sm:rounded-2xl shrink-0 flex items-center justify-center
                    shadow-sm border border-neutral-200/60 dark:border-white/8"
             style="background: {profilColor}12;">
          <Icon icon="lucide:library" width="20" height="20"
                style="color: {profilColor};" />
        </div>

        <div>
          <h1 class="text-lg sm:text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">
            {library.name}
          </h1>

          {#if library.description}
            <p class="text-sm text-neutral-500 dark:text-neutral-400 mt-0.5 max-w-md">
              {library.description}
            </p>
          {/if}

          <!-- Stats -->
          <div class="flex items-center gap-4 mt-3">
            <div class="flex items-center gap-1.5 text-xs text-neutral-500 dark:text-neutral-400">
              <Icon icon={$libraryHeader.icon} width="13" class="opacity-60" />
              <span class="font-medium text-neutral-700 dark:text-neutral-300">{$libraryHeader.total}</span>
              <span>{$libraryHeader.subtitle}</span>
            </div>

            {#if library.total_albums > 0}
              <div class="flex items-center gap-1.5 text-xs text-neutral-500 dark:text-neutral-400">
                <Icon icon="lucide:disc-album" width="13" class="opacity-60" />
                <span class="font-medium text-neutral-700 dark:text-neutral-300">{library.total_albums}</span>
                <span>album{library.total_albums !== 1 ? 's' : ''}</span>
              </div>
            {/if}

            {#if library.total_artists > 0}
              <div class="flex items-center gap-1.5 text-xs text-neutral-500 dark:text-neutral-400">
                <Icon icon="lucide:mic-2" width="13" class="opacity-60" />
                <span class="font-medium text-neutral-700 dark:text-neutral-300">{library.total_artists}</span>
                <span>artiste{library.total_artists !== 1 ? 's' : ''}</span>
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- Actions -->
      <LibraryActionBar library={library} />
    </div>
  </div>

  <!-- Séparateur -->
  <div class="h-px mx-6 shrink-0 bg-linear-to-r from-transparent via-neutral-200/80 dark:via-neutral-700/30 to-transparent"></div>

  <!-- Rien en mode « en haut » : tout est dans l'en-tête général. -->
  {#if placementOnglets !== 'top'}
    <div class="shrink-0">
      <LibraryTabBar
        libraryId={library.id as number}
        avecOnglets={placementOnglets === 'both'}
      />
    </div>
  {/if}
  {/if}

  <!-- Contenu (prend tout l'espace restant, scroll interne) -->
  <div class="flex-1 min-h-0 overflow-hidden">
    {@render children()}
  </div>

  <!-- Overlay drag & drop -->
  {#if dragOver}
    <div class="absolute inset-0 z-50 flex items-center justify-center
                bg-black/40 backdrop-blur-sm rounded-lg pointer-events-none">
      <div class="flex flex-col items-center gap-3 p-8 rounded-2xl
                  bg-white/90 dark:bg-neutral-900/90 border-2 border-dashed border-green-500
                  shadow-2xl shadow-green-500/20">
        <Icon icon="lucide:upload" width={32} class="text-green-500" />
        <p class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">
          Glissez vos fichiers audio ici
        </p>
        <p class="text-xs text-neutral-500">MP3, FLAC, OGG, M4A, WAV…</p>
      </div>
    </div>
  {/if}
</div>
{/if}
