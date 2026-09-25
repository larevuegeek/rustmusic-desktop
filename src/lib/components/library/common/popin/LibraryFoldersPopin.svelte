<script lang="ts">
  import Icon from "@iconify/svelte";
  import { oublierLocalisations } from "$lib/helper/library/trackLocation";
  import { invoke } from "@tauri-apps/api/core";
  import { handleAddDirectory } from "$lib/actions/library/LibraryAction";
  import { libraryContentStore } from "$lib/stores/library/libraryContent.store";

  type LibraryDir = {
    id: string;
    library_id: number;
    path: string;
    name: string;
    is_recursive: boolean;
    is_active: boolean;
    total_files: number;
    total_size: number;
    last_scan_at: string | null;
    scan_status: string;
    created_at: string;
  };

  let { open = $bindable(true), libraryId }: { open: boolean; libraryId: number } = $props();

  let dirs: LibraryDir[] = $state([]);
  let loading = $state(true);
  let rescanningId: string | null = $state(null);

  $effect(() => {
    if (open) loadDirs();
  });

  async function loadDirs() {
    loading = true;
    try {
      dirs = await invoke('get_library_dirs', { libraryId });
    } catch (e) {
      console.error('Failed to load dirs:', e);
    } finally {
      loading = false;
    }
  }

  async function handleRescanDir(dir: LibraryDir) {
    rescanningId = dir.id;
    try {
      await invoke('rescan_library_dir', { libraryId, dirId: dir.id });
      oublierLocalisations();
      await loadDirs();
      libraryContentStore.load(libraryId);
    } catch (e) {
      console.error('Rescan failed:', e);
    } finally {
      rescanningId = null;
    }
  }

  async function handleRemoveDir(dir: LibraryDir) {
    try {
      await invoke('remove_library_dir', { dirId: dir.id });
      dirs = dirs.filter(d => d.id !== dir.id);
    } catch (e) {
      console.error('Remove failed:', e);
    }
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return '—';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return 'Jamais';
    const d = new Date(dateStr);
    return d.toLocaleDateString('fr-FR', {
      day: '2-digit', month: 'short', year: 'numeric',
      hour: '2-digit', minute: '2-digit'
    });
  }

  async function handleAddFolder() {
    await handleAddDirectory(libraryId);
    await loadDirs();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- Overlay -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center"
  onkeydown={(e) => e.key === 'Escape' && (open = false)}
>
  <!-- Backdrop -->
  <button
    type="button"
    class="absolute inset-0 bg-black/60 backdrop-blur-sm cursor-default"
    onclick={() => open = false}
    aria-label="Fermer"
  ></button>

  <!-- Content -->
  <div class="relative w-full max-w-2xl mx-4 max-h-[80vh] flex flex-col
              bg-neutral-50 dark:bg-neutral-900
              border border-neutral-200/60 dark:border-white/8
              rounded-2xl shadow-2xl shadow-black/20
              overflow-hidden">

    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-4
                border-b border-neutral-200/60 dark:border-white/6">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded-lg flex items-center justify-center
                    bg-green-500/10 border border-green-500/20">
          <Icon icon="lucide:folder-cog" width="16" class="text-green-500" />
        </div>
        <div>
          <h2 class="text-base font-semibold text-neutral-800 dark:text-neutral-100">
            Dossiers synchronisés
          </h2>
          <p class="text-[11px] text-neutral-400 dark:text-neutral-500">
            {dirs.length} dossier{dirs.length !== 1 ? 's' : ''} indexé{dirs.length !== 1 ? 's' : ''}
          </p>
        </div>
      </div>

      <button
        class="p-1.5 rounded-lg cursor-pointer
               text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200
               hover:bg-neutral-200/60 dark:hover:bg-white/8
               transition-all"
        onclick={() => open = false}
      >
        <Icon icon="lucide:x" width="16" />
      </button>
    </div>

    <!-- Body -->
    <div class="flex-1 overflow-y-auto px-4 py-3 scrollbar-app">
      {#if loading}
        <div class="flex items-center justify-center py-12">
          <Icon icon="lucide:loader-2" width="20" class="animate-spin text-neutral-400" />
        </div>

      {:else if dirs.length === 0}
        <div class="flex flex-col items-center justify-center py-12 text-center">
          <Icon icon="lucide:folder-open" width="32" class="text-neutral-300 dark:text-neutral-600 mb-3" />
          <p class="text-sm text-neutral-500 dark:text-neutral-400 mb-1">Aucun dossier</p>
          <p class="text-xs text-neutral-400 dark:text-neutral-500">
            Ajoutez un dossier pour indexer vos fichiers audio.
          </p>
        </div>

      {:else}
        <div class="space-y-1">
          {#each dirs as dir (dir.id)}
            <div class="group flex items-center gap-3 px-4 py-3 rounded-xl
                        hover:bg-neutral-100 dark:hover:bg-white/4
                        transition-colors duration-150">

              <!-- Icon -->
              <div class="w-9 h-9 rounded-lg shrink-0 flex items-center justify-center
                          bg-green-500/10 border border-green-500/15">
                <Icon icon="lucide:folder" width="16" class="text-green-500" />
              </div>

              <!-- Info -->
              <div class="flex-1 min-w-0">
                <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200 truncate">
                  {dir.name}
                </p>
                <p class="text-[10px] text-neutral-400 dark:text-neutral-500 truncate mt-0.5 font-mono">
                  {dir.path}
                </p>
              </div>

              <!-- Stats -->
              <div class="hidden sm:flex flex-col items-end text-[10px] text-neutral-400 dark:text-neutral-500 shrink-0 gap-0.5">
                {#if dir.total_files > 0}
                  <span>{dir.total_files} fichiers</span>
                {/if}
                <span>{formatDate(dir.last_scan_at)}</span>
              </div>

              <!-- Actions -->
              <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                <button
                  class="p-1.5 rounded-lg cursor-pointer
                         text-neutral-400 hover:text-green-500
                         hover:bg-green-500/10 transition-all"
                  title="Rescanner"
                  disabled={rescanningId === dir.id}
                  onclick={() => handleRescanDir(dir)}
                >
                  <Icon icon="lucide:refresh-cw" width="14"
                        class={rescanningId === dir.id ? 'animate-spin' : ''} />
                </button>
                <button
                  class="p-1.5 rounded-lg cursor-pointer
                         text-neutral-400 hover:text-red-500
                         hover:bg-red-500/10 transition-all"
                  title="Retirer"
                  onclick={() => handleRemoveDir(dir)}
                >
                  <Icon icon="lucide:trash-2" width="14" />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="px-6 py-3 border-t border-neutral-200/60 dark:border-white/6
                flex items-center justify-between">
      <button
        class="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
               border border-dashed border-neutral-300 dark:border-neutral-700
               text-neutral-500 dark:text-neutral-400
               hover:border-green-500/40 hover:text-green-500 hover:bg-green-500/5
               transition-all duration-150"
        onclick={handleAddFolder}
      >
        <Icon icon="lucide:plus" width="12" />
        Ajouter un dossier
      </button>

      <button
        class="px-4 py-1.5 rounded-lg text-xs font-medium cursor-pointer
               text-neutral-500 dark:text-neutral-400
               hover:bg-neutral-100 dark:hover:bg-white/5
               transition-colors"
        onclick={() => open = false}
      >
        Fermer
      </button>
    </div>
  </div>
</div>
