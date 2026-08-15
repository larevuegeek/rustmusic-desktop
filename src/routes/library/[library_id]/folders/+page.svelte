<script lang="ts">
  import Icon from "@iconify/svelte";
  import { page } from "$app/state";
  import { invoke } from "@tauri-apps/api/core";
  import { libraryHeader } from "$lib/stores/library/libraryHeader";
  import { libraryStore } from "$lib/stores/library/library.store";
  import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
  import { handlePlayTrack } from "$lib/actions/player/PlayerAction";
  import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { tagWorkshop } from "$lib/stores/tags/tagWorkshop.store";
  import { canWriteTags } from "$lib/services/tags/tagEditor.service";
  import { toasts } from "$lib/stores/ui/toast.store";
  import { formatBitrate } from "$lib/helper/tools/audioFormatTools";

  type LibraryDir = {
    id: string;
    library_id: number;
    path: string;
    name: string;
    total_files: number;
  };

  type DirEntry = {
    name: string;
    path: string;
    is_dir: boolean;
    size: number;
    extension: string | null;
  };

  type FileTagsInfo = {
    path: string;
    filename: string;
    extension: string;
    size: number;
    title: string | null;
    artist: string | null;
    album: string | null;
    album_artist: string | null;
    year: string | null;
    genre: string | null;
    track_number: number | null;
    disc_number: number | null;
    duration: number;
    bitrate: number;
    sample_rate: number;
    bits_per_sample: number;
    channels: number;
    audio_format: string;
    cover: string | null;
  };

  const libraryId = $derived(Number(page.params.library_id));

  let rootDirs: LibraryDir[] = $state([]);
  let entries: DirEntry[] = $state([]);
  let loading = $state(true);
  let currentPath: string | null = $state(null);
  // Le breadcrumb est la source de vérité pour la navigation
  // Chaque entrée = un niveau, avec le path exact retourné par le backend
  let breadcrumb: { name: string; path: string }[] = $state([]);

  // Context menu
  let contextMenu = $state<{ x: number; y: number; entry: DirEntry } | null>(null);

  // File info popin
  let showFileInfo = $state(false);
  let fileInfoLoading = $state(false);
  let fileInfo: FileTagsInfo | null = $state(null);

  let isRoot = $derived(currentPath === null);

  $effect(() => {
    libraryHeader.update(() => ({
      subtitle: 'Explorateur',
      icon: 'lucide:folder-open',
      total: isRoot ? rootDirs.length : entries.length
    }));
  });

  $effect(() => {
    loadRootDirs();
  });


  async function loadRootDirs() {
    loading = true;
    try {
      rootDirs = await invoke('get_library_dirs', { libraryId });
    } catch (e) {
      console.error('Failed to load dirs:', e);
    } finally {
      loading = false;
    }
  }

  async function navigateTo(path: string, name: string) {
    loading = true;
    currentPath = path;

    // Vérifier si on clique sur un élément déjà dans le breadcrumb (navigation arrière)
    const existingIndex = breadcrumb.findIndex(b => b.path === path);
    if (existingIndex >= 0) {
      // Tronquer le breadcrumb au niveau cliqué
      breadcrumb = breadcrumb.slice(0, existingIndex + 1);
    } else if (breadcrumb.length === 0) {
      // Premier niveau : on entre dans une racine importée
      breadcrumb = [{ name, path }];
    } else {
      // On descend d'un niveau : ajouter au breadcrumb
      breadcrumb = [...breadcrumb, { name, path }];
    }

    try {
      entries = await invoke('list_directory', { libraryId, path });
    } catch (e) {
      console.error('Failed to list directory:', e);
      entries = [];
    } finally {
      loading = false;
    }
  }

  function goToRoot() {
    currentPath = null;
    breadcrumb = [];
    entries = [];
  }

  function handleContextMenu(e: MouseEvent, entry: DirEntry) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, entry };
  }

  async function openFileInfo(entry: DirEntry) {
    contextMenu = null;
    showFileInfo = true;
    fileInfoLoading = true;
    fileInfo = null;
    try {
      fileInfo = await invoke('get_file_tags', { path: entry.path });
    } catch (e) {
      console.error('Failed to get file tags:', e);
    } finally {
      fileInfoLoading = false;
    }
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return '';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  }

  function formatDuration(seconds: number): string {
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60).toString().padStart(2, '0');
    return `${m}:${s}`;
  }

  function formatSampleRate(sr: number): string {
    return sr >= 1000 ? `${(sr / 1000).toFixed(1)} kHz` : `${sr} Hz`;
  }

  let openingWorkshop = $state(false);

  /**
   * Envoie les fichiers du dossier courant dans l'atelier de tags.
   *
   * C'est l'entrée la plus naturelle : un dossier est presque toujours un
   * album, et c'est là qu'on constate qu'il manque l'année ou que l'artiste
   * est mal orthographié. Seuls les fichiers du niveau affiché partent — pas
   * les sous-dossiers, dont le contenu n'est pas sous les yeux.
   */
  async function openWorkshop() {
    const files = entries.filter(e => !e.is_dir).map(e => e.path);
    if (files.length === 0 || openingWorkshop) return;

    openingWorkshop = true;
    try {
      const checks = await Promise.all(files.map(p => canWriteTags(p)));
      const writable = files.filter((_, i) => checks[i]);

      if (writable.length === 0) {
        toasts.push({
          type: "info",
          title: $t("workshop.title"),
          message: $t("tags.none_writable"),
        });
        return;
      }

      const folder = breadcrumb[breadcrumb.length - 1]?.name ?? $t("nav.folders");
      await tagWorkshop.load(writable, folder, `/library/${libraryId}/folders`);
      await goto(`/library/${libraryId}/tags`);
    } finally {
      openingWorkshop = false;
    }
  }

  let dirCount = $derived(entries.filter(e => e.is_dir).length);
  let fileCount = $derived(entries.filter(e => !e.is_dir).length);

  function goToParent() {
    if (breadcrumb.length <= 1) {
      // On est à la racine importée (ou pas de breadcrumb) → retour grille
      goToRoot();
      return;
    }

    // Remonter d'un niveau via le breadcrumb
    const parent = breadcrumb[breadcrumb.length - 2];
    navigateTo(parent.path, parent.name);
  }
</script>

<div class="flex-1 px-6 py-4 overflow-y-auto scrollbar-app h-full">

  <!-- ═══ BREADCRUMB ═══ -->
  {#if !isRoot}
    <div class="flex items-center gap-1 mb-4 px-1">
      <button
        class="flex items-center gap-1 text-xs text-neutral-400 hover:text-green-500 cursor-pointer transition-colors"
        onclick={goToRoot}
      >
        <Icon icon="lucide:hard-drive" width="12" />
        Racine
      </button>

      {#each breadcrumb as crumb, i (crumb.path)}
        <Icon icon="lucide:chevron-right" width="12" class="text-neutral-300 dark:text-neutral-600" />
        {#if i < breadcrumb.length - 1}
          <button
            class="text-xs text-neutral-400 hover:text-green-500 cursor-pointer transition-colors truncate max-w-[120px]"
            onclick={() => navigateTo(crumb.path, crumb.name)}
          >
            {crumb.name}
          </button>
        {:else}
          <span class="text-xs font-medium text-neutral-700 dark:text-neutral-200 truncate max-w-[200px]">
            {crumb.name}
          </span>
        {/if}
      {/each}

      <div class="ml-auto flex items-center gap-3 text-[10px] text-neutral-400">
        {#if fileCount > 0}
          <button
            type="button"
            onclick={openWorkshop}
            disabled={openingWorkshop}
            class="flex items-center gap-1.5 px-2 h-6 rounded-lg text-[11px]
                   cursor-pointer transition-colors disabled:opacity-40
                   text-neutral-500 dark:text-neutral-400
                   hover:text-emerald-600 dark:hover:text-emerald-400
                   hover:bg-emerald-500/10"
            title={$t('workshop.fix_tags')}
          >
            <Icon
              icon={openingWorkshop ? 'lucide:loader-circle' : 'lucide:table-properties'}
              width="12"
              class={openingWorkshop ? 'animate-spin' : ''}
            />
            {$t('workshop.short')}
          </button>
        {/if}
        {#if dirCount > 0}
          <span>{dirCount} dossier{dirCount > 1 ? 's' : ''}</span>
        {/if}
        {#if fileCount > 0}
          <span>{fileCount} fichier{fileCount > 1 ? 's' : ''}</span>
        {/if}
      </div>
    </div>
  {/if}

  <!-- ═══ CONTENU ═══ -->
  {#if $libraryStore.isImporting}
    <LibraryImportingLoader />

  {:else if loading}
    <div class="flex items-center justify-center py-20">
      <Icon icon="lucide:loader-2" width="24" class="animate-spin text-neutral-400" />
    </div>

  {:else if isRoot}
    {#if rootDirs.length === 0}
      <div class="flex flex-col items-center justify-center py-20 text-center">
        <div class="relative mb-5">
          <div class="absolute inset-0 rounded-2xl bg-green-500/25 blur-2xl scale-[2] animate-pulse"></div>
          <div class="relative w-16 h-16 rounded-2xl bg-neutral-100 dark:bg-neutral-800
                      border border-neutral-200/60 dark:border-neutral-700/40
                      flex items-center justify-center">
            <Icon icon="lucide:folder-open" width="24" class="text-green-500/60" />
          </div>
        </div>
        <h3 class="text-base font-semibold text-neutral-700 dark:text-neutral-200 mb-1.5">
          Aucun dossier importé
        </h3>
        <p class="text-sm text-neutral-400 dark:text-neutral-500 max-w-xs">
          Importez un dossier depuis la barre d'action pour l'explorer ici.
        </p>
      </div>
    {:else}
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 2xl:grid-cols-6 min-[1800px]:grid-cols-7 min-[2200px]:grid-cols-8 gap-3">
        {#each rootDirs as dir (dir.id)}
          <button
            class="group flex flex-col items-center gap-2 p-4 rounded-2xl cursor-pointer
                   bg-white/50 dark:bg-white/3
                   border border-neutral-200/50 dark:border-white/5
                   hover:bg-green-500/5 hover:border-green-500/20
                   hover:shadow-lg hover:shadow-green-500/5
                   active:scale-[0.97] transition-all duration-200"
            onclick={() => navigateTo(dir.path, dir.name)}
          >
            <div class="w-14 h-14 rounded-xl flex items-center justify-center
                        bg-green-500/8 group-hover:bg-green-500/15 transition-colors duration-200">
              <Icon icon="lucide:folder" width="24"
                    class="text-green-500/70 group-hover:text-green-500 transition-colors" />
            </div>
            <div class="text-center min-w-0 w-full">
              <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200 truncate">{dir.name}</p>
              <p class="text-[10px] text-neutral-400 mt-0.5">
                {dir.total_files} fichier{dir.total_files !== 1 ? 's' : ''}
              </p>
            </div>
          </button>
        {/each}
      </div>
    {/if}

  {:else if entries.length === 0}
    <div class="flex flex-col items-center justify-center py-20 text-center">
      <Icon icon="lucide:folder-x" width="32" class="text-neutral-300 dark:text-neutral-600 mb-3" />
      <p class="text-sm text-neutral-400">Dossier vide</p>
    </div>

  {:else}
    <!-- ═══ VUE EXPLORATEUR ═══ -->
    <div class="flex flex-col">
      <!-- Dossier parent -->
      <button
        type="button"
        class="group flex items-center gap-3 px-3 py-2.5 rounded-xl cursor-pointer
               hover:bg-neutral-50 dark:hover:bg-white/4
               active:bg-neutral-100 dark:active:bg-white/6
               transition-colors duration-100 text-left w-full"
        onclick={goToParent}
      >
        <div class="w-9 h-9 rounded-lg shrink-0 flex items-center justify-center
                    bg-neutral-200/50 dark:bg-white/5 group-hover:bg-neutral-300/50 dark:group-hover:bg-white/8 transition-colors">
          <Icon icon="lucide:corner-left-up" width="16" class="text-neutral-400" />
        </div>
        <p class="text-sm text-neutral-500 dark:text-neutral-400 group-hover:text-neutral-700 dark:group-hover:text-neutral-200 transition-colors">
          ..
        </p>
      </button>

      {#each entries as entry (entry.path)}
        <button
          type="button"
          class="group flex items-center gap-3 px-3 py-2.5 rounded-xl cursor-pointer
                 hover:bg-neutral-50 dark:hover:bg-white/4
                 active:bg-neutral-100 dark:active:bg-white/6
                 transition-colors duration-100 w-full text-left"
          onclick={() => entry.is_dir ? navigateTo(entry.path, entry.name) : handlePlayTrack(entry.path)}
          oncontextmenu={(e) => !entry.is_dir && handleContextMenu(e, entry)}
        >
          <!-- Icône -->
          {#if entry.is_dir}
            <div class="w-9 h-9 rounded-lg shrink-0 flex items-center justify-center
                        bg-amber-500/8 group-hover:bg-amber-500/15 transition-colors">
              <Icon icon="lucide:folder" width="16" class="text-amber-500" />
            </div>
          {:else}
            <div class="w-9 h-9 rounded-lg shrink-0 flex items-center justify-center
                        bg-green-500/8 group-hover:bg-green-500/15 transition-colors">
              <Icon icon="lucide:file-audio" width="16" class="text-green-500" />
            </div>
          {/if}

          <!-- Nom -->
          <div class="flex-1 min-w-0 text-left">
            <p class="text-sm text-neutral-700 dark:text-neutral-200 truncate
                      group-hover:text-neutral-900 dark:group-hover:text-white transition-colors">
              {entry.name}
            </p>
          </div>

          <!-- Extension badge -->
          {#if entry.extension}
            <span class="text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded
                         bg-neutral-100 dark:bg-white/5
                         text-neutral-400 dark:text-neutral-500">
              {entry.extension}
            </span>
          {/if}

          <!-- Taille -->
          {#if !entry.is_dir && entry.size > 0}
            <span class="text-[11px] text-neutral-400 dark:text-neutral-500 tabular-nums shrink-0 w-16 text-right">
              {formatSize(entry.size)}
            </span>
          {/if}

          <!-- Actions -->
          {#if entry.is_dir}
            <Icon icon="lucide:chevron-right" width="14"
                  class="text-neutral-300 dark:text-neutral-600
                         group-hover:text-neutral-500 dark:group-hover:text-neutral-400
                         transition-colors shrink-0" />
          {:else}
            <span
              class="p-1.5 rounded-lg shrink-0
                     opacity-0 group-hover:opacity-100
                     text-green-500
                     transition-all duration-150"
            >
              <Icon icon="lucide:play" width="14" />
            </span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<!-- ═══ MENU CONTEXTUEL ═══ -->
{#if contextMenu}
  <TrackContextMenu
    track={{ path: contextMenu.entry.path, title: contextMenu.entry.name }}
    x={contextMenu.x}
    y={contextMenu.y}
    libraryId={libraryId}
    onclose={() => contextMenu = null}
  />
{/if}

<!-- ═══ POPIN INFOS FICHIER ═══ -->
{#if showFileInfo}
  <div class="fixed inset-0 z-50 flex items-center justify-center">
    <button type="button" class="absolute inset-0 bg-black/60 backdrop-blur-sm cursor-default" onclick={() => showFileInfo = false} aria-label="Fermer"></button>

    <div class="relative w-full max-w-lg mx-4
                bg-neutral-50 dark:bg-neutral-900
                border border-neutral-200/60 dark:border-white/8
                rounded-2xl shadow-2xl shadow-black/20
                overflow-hidden">

      <!-- Header avec cover -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-neutral-200/60 dark:border-white/6">
        <div class="flex items-center gap-3">
          {#if fileInfo?.cover}
            <img src={fileInfo.cover} alt="Cover"
                 class="w-10 h-10 rounded-lg object-cover shadow-sm" />
          {:else}
            <div class="w-10 h-10 rounded-lg flex items-center justify-center bg-green-500/10 border border-green-500/20">
              <Icon icon="lucide:file-audio" width="16" class="text-green-500" />
            </div>
          {/if}
          <div class="min-w-0">
            <h2 class="text-base font-semibold text-neutral-800 dark:text-neutral-100 truncate max-w-75">
              {fileInfo?.title ?? fileInfo?.filename ?? 'Chargement...'}
            </h2>
            {#if fileInfo?.artist}
              <p class="text-xs text-neutral-400 truncate">{fileInfo.artist}</p>
            {/if}
          </div>
        </div>
        <button
          class="p-1.5 rounded-lg cursor-pointer text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200
                 hover:bg-neutral-200/60 dark:hover:bg-white/8 transition-all"
          onclick={() => showFileInfo = false}
        >
          <Icon icon="lucide:x" width="16" />
        </button>
      </div>

      <!-- Body -->
      <div class="px-6 py-5 max-h-[60vh] overflow-y-auto scrollbar-app">
        {#if fileInfoLoading}
          <div class="flex items-center justify-center py-12">
            <Icon icon="lucide:loader-2" width="20" class="animate-spin text-neutral-400" />
          </div>
        {:else if fileInfo}
          <div class="space-y-5">

            <!-- Cover -->
            {#if fileInfo.cover}
              <div class="flex justify-center">
                <img src={fileInfo.cover} alt="Cover"
                     class="w-40 h-40 rounded-xl object-cover shadow-lg shadow-black/20" />
              </div>
            {/if}

            <!-- Tags principaux -->
            <div>
              <h3 class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 mb-3">Tags</h3>
              <div class="grid grid-cols-2 gap-x-6 gap-y-2.5">
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Titre</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.title ?? '—'}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Artiste</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.artist ?? '—'}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Album</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.album ?? '—'}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Album Artist</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.album_artist ?? '—'}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Année</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.year ?? '—'}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Genre</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.genre ?? '—'}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Piste</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.track_number ?? '—'}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Disque</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.disc_number ?? '—'}</p>
                </div>
              </div>
            </div>

            <div class="h-px bg-neutral-200/60 dark:bg-white/5"></div>

            <!-- Infos techniques -->
            <div>
              <h3 class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 mb-3">Audio</h3>
              <div class="grid grid-cols-2 gap-x-6 gap-y-2.5">
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Durée</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{formatDuration(fileInfo.duration)}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Format</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.audio_format}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Bitrate</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{formatBitrate(fileInfo.bitrate)}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Sample Rate</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{formatSampleRate(fileInfo.sample_rate)}</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Bits/Sample</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.bits_per_sample} bits</p>
                </div>
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Canaux</p>
                  <p class="text-sm text-neutral-700 dark:text-neutral-200">{fileInfo.channels === 2 ? 'Stéréo' : fileInfo.channels === 1 ? 'Mono' : `${fileInfo.channels} ch`}</p>
                </div>
              </div>
            </div>

            <div class="h-px bg-neutral-200/60 dark:bg-white/5"></div>

            <!-- Infos fichier -->
            <div>
              <h3 class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 mb-3">Fichier</h3>
              <div class="space-y-2">
                <div>
                  <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Chemin</p>
                  <p class="text-xs text-neutral-600 dark:text-neutral-300 font-mono break-all">{fileInfo.path}</p>
                </div>
                <div class="grid grid-cols-2 gap-x-6">
                  <div>
                    <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Taille</p>
                    <p class="text-sm text-neutral-700 dark:text-neutral-200">{formatSize(fileInfo.size)}</p>
                  </div>
                  <div>
                    <p class="text-[10px] text-neutral-400 dark:text-neutral-500">Extension</p>
                    <p class="text-sm text-neutral-700 dark:text-neutral-200 uppercase">{fileInfo.extension}</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
