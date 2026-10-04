<script lang="ts">
import { page } from "$app/state";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { fade } from "svelte/transition";
import { profilSelector } from "$lib/stores/profil/profil.store";
import type { Library } from "$lib/types/db/library/Library";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { loadLibrary } from "$lib/services/library/library.service";
import LibraryErrorLoading from "$lib/components/library/common/error/LibraryErrorLoading.svelte";
import Icon from "@iconify/svelte";
import LibraryHeader from "$lib/components/library/common/LibraryHeader.svelte";
import LibraryTabBar from "$lib/components/library/common/LibraryTabBar.svelte";
import ImportAccueil from "$lib/components/library/common/ImportAccueil.svelte";
import { importAffiche } from "$lib/stores/library/importProgress.store";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { lirePlacement, ongletCourant } from "$lib/config/libraryTabs";
import { invoke } from "@tauri-apps/api/core";
import { libraryStore } from "$lib/stores/library/library.store";
import { toasts } from "$lib/stores/ui/toast.store";
import { t } from "$lib/i18n";

let library: Library | null = $state(null);
// Clé de traduction, traduite à l'affichage.
let error: string | null = $state(null);
let dragOver = $state(false);

const libraryId = $derived(Number(page.params.library_id));
const profil = $derived($profilSelector.profilSelected);

let currentTag = 0;

$effect(() => {
    const id = libraryId;
    const p = profil;
    library = null;
    error = null;

    if (!p) { error = "system.no_profile_selected"; return; }
    if (!id || isNaN(id)) { error = "system.invalid_library_id"; return; }

    const tag = ++currentTag;
    (async () => {
        const result = await loadLibrary(id, p.id, tag, currentTag);
        if (tag !== currentTag) return;
        if (!result) { error = "system.library_not_found"; return; }
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
// Les fiches (album, artiste, morceau, période) ont leur propre barre de retour : pas d'en-tête de bibliothèque.
let isFocusedView = $derived(page.url.pathname.endsWith("/tags") || /^\/library\/\d+\/(albums|artists|tracks|years)\/[^/]+\/?$/.test(page.url.pathname));
// Les pages de liste ont leur propre barre d'outils : pas de rangée de commandes vide au-dessus.
const avecSaBarre = $derived(/^\/library\/\d+\/(albums|tracks|artists|genres|years|mixes|folders)\/?$/.test(page.url.pathname));

const placementOnglets = $derived(lirePlacement($settingsStore.library_tabs_position));

// Bibliothèque sans morceau et sans import en cours : les listes cèdent la place à l'import.
const vide = $derived(
  avecSaBarre && !$importAffiche
  && $libraryStore.libraries.find((l) => l.id === libraryId)?.total_tracks === 0
);

// Glisser-déposer : WebView2 ne donne pas les chemins des fichiers déposés, l'événement de Tauri si.
const EXTENSIONS_AUDIO = ['mp3', 'flac', 'ogg', 'm4a', 'wav', 'aac', 'opus', 'dsf', 'dff', 'aiff'];

$effect(() => {
  let arreter: (() => void) | null = null;
  let fini = false;
  try {
    getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "enter" || p.type === "over") dragOver = true;
      else if (p.type === "leave") dragOver = false;
      else if (p.type === "drop") {
        dragOver = false;
        importer(p.paths);
      }
    }).then((u) => { if (fini) u(); else arreter = u; }).catch(() => {});
  } catch {
    // Hors Tauri (banc d'essai) : pas de dépôt de fichiers.
  }
  return () => { fini = true; arreter?.(); };
});

async function importer(chemins: string[]) {
  if (!library?.id) return;
  const files = chemins.filter((c) => EXTENSIONS_AUDIO.includes(c.split('.').pop()?.toLowerCase() ?? ''));
  if (files.length === 0) {
    toasts.push({ type: "info", title: $t("system.drop_no_audio"), message: $t("system.drop_no_audio_desc") });
    return;
  }

  libraryStore.setImporting(true);
  try {
    const tracks = await invoke<unknown[]>('add_files', { libraryId: library.id, files });
    await libraryContentStore.refresh(library.id as number);
    await libraryStore.refresh();
    toasts.push({ type: "success", title: $t("system.import_done"), message: $t(tracks.length === 1 ? "system.tracks_added_one" : "system.tracks_added_n").replace("{n}", String(tracks.length)) });
  } catch (err) {
    console.error("[bibliothèque] import par dépôt :", err);
    toasts.push({ type: "error", title: $t("system.error"), message: $t("system.import_files_failed") });
  } finally {
    libraryStore.setImporting(false);
  }
}
</script>

{#if error}
    <LibraryErrorLoading error={$t(error)} />
{:else if library}
<div class="biblio-maquette flex flex-col h-full relative">

  {#if vide}
    <ImportAccueil {library} section={ongletCourant(page.url.pathname)} />
  {:else}
  {#if !isFocusedView}
  <LibraryHeader {library} />

  <!-- Seulement en mode « à gauche » : ailleurs l'en-tête général porte déjà
       les sections et les commandes. -->
  {#if placementOnglets === 'sidebar' && !avecSaBarre}
    <div class="shrink-0">
      <LibraryTabBar />
    </div>
  {/if}
  {/if}

  <!-- Contenu (prend tout l'espace restant, scroll interne) -->
  <div class="flex-1 min-h-0 overflow-hidden">
    <!-- Par route, pas par adresse : passer d'un album au suivant garde la page (et son contenu pendant le chargement). -->
    {#key page.route.id}
      <div class="h-full" in:fade={{ duration: 120, delay: 60 }}>
        {@render children()}
      </div>
    {/key}
  </div>
  {/if}

  <!-- Overlay drag & drop -->
  {#if dragOver}
    <div class="absolute inset-0 z-50 flex items-center justify-center
                bg-black/40 backdrop-blur-sm rounded-lg pointer-events-none">
      <div class="flex flex-col items-center gap-3 p-8 rounded-2xl
                  bg-white/90 dark:bg-neutral-900/90 border-2 border-dashed border-green-500
                  shadow-2xl shadow-green-500/20">
        <Icon icon="lucide:upload" width={32} class="text-green-500" />
        <p class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">
          {$t("system.drop_here")}
        </p>
        <p class="text-xs text-neutral-500">MP3, FLAC, OGG, M4A, WAV…</p>
      </div>
    </div>
  {/if}
</div>
{/if}
